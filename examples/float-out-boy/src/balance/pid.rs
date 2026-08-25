use super::current::{PitchBasedCurrent, RequestedCurrent};
use super::loop_io::{LoopConfig, LoopInput, LoopState, PidState};
use crate::domain::{FloatOutBoyDarkRideState, FloatOutBoyRealtimeRuntimeSetpoint};
use crate::ema::EmaAlpha;
use crate::motor_torque::{MotorTorque, MotorTorqueConstant, MotorTorqueLimit};
use vescpkg_rs::prelude::VescSeconds;
use vescpkg_rs::prelude::{AngleDegrees, AngularVelocity, ElectricalSpeed, ImuRoll};
use vescpkg_rs::{AngleCurrentGain, IntegralCurrentGain, PidScale, RateCurrentGain, Rpm};

/// Board setpoint error used by Float Out Boy PID P/I terms.
///
/// Source map: upstream computes `setpoint - imu->balance_pitch` at
/// `third_party/float-out-boy/src/pid.c:40`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct SetpointError(AngleDegrees);

impl SetpointError {
    #[inline]
    const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    #[inline]
    pub(super) fn from_input(
        setpoint: FloatOutBoyRealtimeRuntimeSetpoint,
        balance_pitch: AngleDegrees,
    ) -> Self {
        Self::new(setpoint.angle() - balance_pitch)
    }

    #[inline]
    const fn angle(self) -> AngleDegrees {
        self.0
    }

    #[inline]
    pub(super) fn integral_torque(
        self,
        integral: MotorTorque,
        ki: IntegralCurrentGain,
        limit: MotorTorqueLimit,
        elapsed: VescSeconds,
    ) -> MotorTorque {
        // C map: `third_party/float-out-boy/src/pid.c:40-46` integrates `p * ki`, then
        // clamps by a positive `ki_limit` while preserving sign. Zero is the
        // disabled-limit sentinel exposed at
        // `third_party/float-out-boy/src/conf/settings.xml:1679-1707`.
        let increment = (self.angle() * ki).scaled_by(PidScale::new(720.0 * elapsed.as_seconds()));
        let next =
            integral.add(MotorTorqueConstant::REFLOAT_COMPAT.torque_from_motor_current(increment));
        if limit.torque().is_positive() {
            limit.clamp(next)
        } else {
            next
        }
    }

    #[inline]
    pub(super) fn angle_proportional_torque(
        self,
        kp: AngleCurrentGain,
        accel_scale: PidScale,
        brake_scale: PidScale,
    ) -> MotorTorque {
        // C map: `third_party/float-out-boy/src/pid.c:69` applies KP and selects the
        // accel/brake scale from the sign of the setpoint error.
        let scale = ScaleSide::from_setpoint_error(self).scale(accel_scale, brake_scale);
        MotorTorqueConstant::REFLOAT_COMPAT
            .torque_from_motor_current(self.angle() * kp.scaled_by(scale))
    }
}

/// Float Out Boy pitch-rate value after roll/yaw mixing and darkride sign handling.
///
/// Source map: upstream computes `imu->pitch_rate` at
/// `third_party/float-out-boy/src/imu.c:46-53`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct PitchRate(AngularVelocity);

impl PitchRate {
    #[inline]
    fn from_roll_corrected(rate: AngularVelocity, darkride: FloatOutBoyDarkRideState) -> Self {
        // C map: `imu_update` flips pitch rate when darkride is active at
        // `third_party/float-out-boy/src/imu.c:52-54`.
        Self(match darkride {
            FloatOutBoyDarkRideState::Active => -rate,
            FloatOutBoyDarkRideState::Upright => rate,
        })
    }

    #[inline]
    pub(super) fn from_imu(
        roll: ImuRoll,
        gyro_pitch: AngularVelocity,
        gyro_yaw: AngularVelocity,
        darkride: FloatOutBoyDarkRideState,
    ) -> Self {
        let pitch_rate = RollProjection::from_roll(roll).pitch_rate(gyro_pitch, gyro_yaw);

        Self::from_roll_corrected(pitch_rate, darkride)
    }

    #[inline]
    pub(super) const fn rate(self) -> AngularVelocity {
        self.0
    }

    #[inline]
    fn damping_torque(self, kp2: RateCurrentGain) -> MotorTorque {
        // C map: `third_party/float-out-boy/src/pid.c:71` negates pitch rate before
        // multiplying by the rate gain.
        MotorTorqueConstant::REFLOAT_COMPAT.torque_from_motor_current(self.rate() * -kp2)
    }

    #[inline]
    pub(super) fn rate_damping_torque(
        self,
        kp2: RateCurrentGain,
        accel_scale: PidScale,
        brake_scale: PidScale,
    ) -> MotorTorque {
        let rate_damping = self.damping_torque(kp2);
        // C map: `third_party/float-out-boy/src/pid.c:72` picks accel/brake scale
        // from the sign of `rate_p`.
        let scale = ScaleSide::from_torque(rate_damping).scale(accel_scale, brake_scale);

        rate_damping.scaled_by(scale.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RollProjection {
    sin: f32,
    cos: f32,
}

impl RollProjection {
    #[inline]
    fn from_roll(roll: ImuRoll) -> Self {
        // C map: `imu_update` uses raw roll radians for the pitch-rate yaw
        // projection at `third_party/float-out-boy/src/imu.c:46-51`.
        let roll_radians = roll.angle().as_radians();
        if !roll_radians.is_finite() {
            return Self {
                sin: f32::NAN,
                cos: f32::NAN,
            };
        }
        let roll_radians = roll_radians.clamp(-core::f32::consts::PI, core::f32::consts::PI);
        let (sin, cos) = bounded_sin_cos(roll_radians);
        Self { sin, cos }
    }

    #[inline]
    fn pitch_rate(self, gyro_pitch: AngularVelocity, gyro_yaw: AngularVelocity) -> AngularVelocity {
        // C map: `third_party/float-out-boy/src/imu.c:49-51` damps yaw influence
        // on pitch-rate while the board is rolled.
        let Self { sin, cos } = self;
        gyro_pitch * (cos * cos) + gyro_yaw * (sin * cos)
    }
}

fn bounded_sin_cos(angle: f32) -> (f32, f32) {
    let (angle, cosine_sign) = if angle > core::f32::consts::FRAC_PI_2 {
        (core::f32::consts::PI - angle, -1.0)
    } else if angle < -core::f32::consts::FRAC_PI_2 {
        (-core::f32::consts::PI - angle, -1.0)
    } else {
        (angle, 1.0)
    };
    let squared = angle * angle;
    let sin = angle
        * (1.0
            + squared
                * (-1.0 / 6.0
                    + squared * (1.0 / 120.0 + squared * (-1.0 / 5040.0 + squared / 362_880.0))));
    let cos = cosine_sign
        * (1.0
            + squared
                * (-1.0 / 2.0
                    + squared * (1.0 / 24.0 + squared * (-1.0 / 720.0 + squared / 40_320.0))));
    (sin, cos)
}

#[cfg(test)]
mod tests {
    use super::{RollProjection, bounded_sin_cos};
    use vescpkg_rs::prelude::{AngleRadians, ImuRoll};

    #[test]
    fn roll_projection_bounds_out_of_contract_firmware_angles_before_trigonometry() {
        let projection =
            RollProjection::from_roll(ImuRoll::new(AngleRadians::from_radians(f32::MAX)));

        assert!(projection.sin.is_finite());
        assert!(projection.cos.is_finite());
    }

    #[test]
    fn bounded_projection_matches_refloat_trigonometry_over_valid_rolls() {
        for angle in [
            -core::f32::consts::PI,
            -1.0,
            0.0,
            1.0,
            core::f32::consts::PI,
        ] {
            let (sin, cos) = bounded_sin_cos(angle);
            assert!((sin - vescpkg_rs::sin(angle)).abs() < 0.001);
            assert!((cos - vescpkg_rs::cos(angle)).abs() < 0.001);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleDirection {
    Coasting,
    Forward,
    Reverse,
}

impl ScaleDirection {
    #[inline]
    fn from_motor_erpm(motor_erpm: ElectricalSpeed) -> Self {
        // C map: PID scale smoothing returns to unity below 500 ERPM, then
        // chooses forward/reverse scaling at `third_party/float-out-boy/src/pid.c:48-67`.
        let erpm = motor_erpm.rpm();
        if erpm.abs() < Rpm::from_revolutions_per_minute(500.0) {
            Self::Coasting
        } else if erpm.is_positive() {
            Self::Forward
        } else {
            Self::Reverse
        }
    }

    #[inline]
    const fn targets(self, config: LoopConfig) -> ScaleTargets {
        match self {
            Self::Coasting => ScaleTargets::UNITY,
            Self::Forward => ScaleTargets {
                brake: ScalePair::new(config.kp_brake, config.kp2_brake),
                accel: ScalePair::UNITY,
            },
            Self::Reverse => ScaleTargets {
                brake: ScalePair::UNITY,
                accel: ScalePair::new(config.kp_brake, config.kp2_brake),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScalePair {
    angle_proportional: PidScale,
    rate_damping: PidScale,
}

impl ScalePair {
    const UNITY: Self = Self {
        angle_proportional: PidScale::new(1.0),
        rate_damping: PidScale::new(1.0),
    };

    #[inline]
    const fn new(angle_proportional: PidScale, rate_damping: PidScale) -> Self {
        Self {
            angle_proportional,
            rate_damping,
        }
    }

    #[inline]
    fn smoothed_angle_proportional(self, current: PidScale, alpha: EmaAlpha) -> PidScale {
        // C map: `third_party/float-out-boy/src/pid.c:51-66` uses a 1% target / 99%
        // previous one-pole filter for all PID scale coefficients.
        current.lerp(self.angle_proportional, alpha.factor())
    }

    #[inline]
    fn smoothed_rate_damping(self, current: PidScale, alpha: EmaAlpha) -> PidScale {
        // C map: `third_party/float-out-boy/src/pid.c:51-66` uses the same 1% / 99%
        // filter for angle-P and rate-P scale coefficients.
        current.lerp(self.rate_damping, alpha.factor())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScaleTargets {
    brake: ScalePair,
    accel: ScalePair,
}

impl ScaleTargets {
    const UNITY: Self = Self {
        brake: ScalePair::UNITY,
        accel: ScalePair::UNITY,
    };

    #[inline]
    fn smoothed_into(self, state: LoopState, elapsed: VescSeconds) -> PidState {
        // C map: `third_party/float-out-boy/src/pid.c:51-66` smooths brake and accel
        // PID scale pairs back into the stored loop state.
        let alpha = EmaAlpha::from_elapsed(vescpkg_rs::Frequency::from_hertz(1.0), elapsed);
        PidState {
            kp_brake_scale: self
                .brake
                .smoothed_angle_proportional(state.pid.kp_brake_scale, alpha),
            kp2_brake_scale: self
                .brake
                .smoothed_rate_damping(state.pid.kp2_brake_scale, alpha),
            kp_accel_scale: self
                .accel
                .smoothed_angle_proportional(state.pid.kp_accel_scale, alpha),
            kp2_accel_scale: self
                .accel
                .smoothed_rate_damping(state.pid.kp2_accel_scale, alpha),
            ..state.pid
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleSide {
    Accel,
    Brake,
}

impl ScaleSide {
    #[inline]
    fn from_setpoint_error(error: SetpointError) -> Self {
        // C map: `third_party/float-out-boy/src/pid.c:69` picks accel vs brake scale
        // from the sign of the setpoint error.
        if error.angle().is_positive() {
            Self::Accel
        } else {
            Self::Brake
        }
    }

    #[inline]
    fn from_torque(torque: MotorTorque) -> Self {
        // C map: `third_party/float-out-boy/src/pid.c:72` picks accel vs brake scale
        // from the sign of the rate-P current contribution.
        if torque.is_positive() {
            Self::Accel
        } else {
            Self::Brake
        }
    }

    #[inline]
    const fn scale(self, accel_scale: PidScale, brake_scale: PidScale) -> PidScale {
        // C map: `third_party/float-out-boy/src/pid.c:69-72` chooses between the
        // accel and brake scale coefficients after the sign check.
        match self {
            Self::Accel => accel_scale,
            Self::Brake => brake_scale,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Torques {
    angle_proportional: MotorTorque,
    rate_damping: MotorTorque,
    integral: MotorTorque,
}

impl Torques {
    #[inline]
    pub(super) fn pitch_based_current(
        self,
        booster_torque: MotorTorque,
        motor_torque_constant: MotorTorqueConstant,
        softstart_pid_limit: vescpkg_rs::MotorCurrentLimit,
        motor_current_max: vescpkg_rs::MotorCurrentLimit,
        softstart_increment: vescpkg_rs::Current,
    ) -> PitchBasedCurrent {
        PitchBasedCurrent::from_torques(
            self.rate_damping,
            booster_torque,
            motor_torque_constant,
            softstart_pid_limit,
            motor_current_max,
            softstart_increment,
        )
    }

    #[inline]
    pub(super) fn requested_with_pitch_based(
        self,
        pitch_based: PitchBasedCurrent,
        motor_torque_constant: MotorTorqueConstant,
    ) -> RequestedCurrent {
        RequestedCurrent(
            motor_torque_constant
                .motor_current_from_torque(self.angle_proportional.add(self.integral))
                + pitch_based.current,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SideScale {
    accel: PidScale,
    brake: PidScale,
}

impl SideScale {
    #[inline]
    const fn new(accel: PidScale, brake: PidScale) -> Self {
        Self { accel, brake }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TorqueScales {
    angle_proportional: SideScale,
    rate_damping: SideScale,
}

impl TorqueScales {
    #[inline]
    const fn from_state(state: LoopState) -> Self {
        // C map: `third_party/float-out-boy/src/pid.c:51-66` keeps separate accel
        // and brake PID scale pairs for angle-P and rate-P smoothing.
        Self {
            angle_proportional: SideScale::new(state.pid.kp_accel_scale, state.pid.kp_brake_scale),
            rate_damping: SideScale::new(state.pid.kp2_accel_scale, state.pid.kp2_brake_scale),
        }
    }

    #[inline]
    fn angle_proportional_torque(self, error: SetpointError, kp: AngleCurrentGain) -> MotorTorque {
        error.angle_proportional_torque(
            kp,
            self.angle_proportional.accel,
            self.angle_proportional.brake,
        )
    }

    #[inline]
    fn rate_damping_torque(self, pitch_rate: PitchRate, kp2: RateCurrentGain) -> MotorTorque {
        pitch_rate.rate_damping_torque(kp2, self.rate_damping.accel, self.rate_damping.brake)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Phase {
    config: LoopConfig,
    input: LoopInput,
}

impl Phase {
    #[inline]
    pub(super) const fn from_step(config: LoopConfig, input: LoopInput) -> Self {
        Self { config, input }
    }

    #[inline]
    pub(super) fn update_state(
        self,
        state: LoopState,
        elapsed: VescSeconds,
    ) -> (Torques, LoopState) {
        // C map: `third_party/float-out-boy/src/pid.c:37-73` updates P/I/rate-P before
        // smoothing the accel/brake scale coefficients for the next tick.
        let config = self.config;
        let torque_scales = TorqueScales::from_state(state);
        let setpoint_error = self.input.setpoint_error();
        let pitch_rate = self.input.pitch_rate();
        let torques = Torques {
            angle_proportional: torque_scales.angle_proportional_torque(setpoint_error, config.kp),
            rate_damping: torque_scales.rate_damping_torque(pitch_rate, config.kp2),
            integral: setpoint_error.integral_torque(
                state.pid.integral_torque,
                config.ki,
                config.ki_limit,
                elapsed,
            ),
        };
        let state = state.with_updated_pid_state(
            self.config,
            self.input.motor_erpm,
            torques.integral,
            elapsed,
        );

        (torques, state)
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn setpoint_error(&self) -> SetpointError {
        SetpointError::from_input(self.setpoint, self.balance_pitch)
    }

    #[inline]
    pub(super) fn pitch_rate(&self) -> PitchRate {
        PitchRate::from_imu(self.roll, self.gyro_pitch, self.gyro_yaw, self.darkride)
    }
}

impl LoopState {
    /// Source map: upstream stores integral current and smooths PID scales at
    /// `third_party/float-out-boy/src/pid.c:40-67`.
    #[inline]
    pub(super) fn with_updated_pid_state(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        integral: MotorTorque,
        elapsed: VescSeconds,
    ) -> Self {
        Self {
            pid: PidState {
                integral_torque: integral,
                ..ScaleDirection::from_motor_erpm(motor_erpm)
                    .targets(config)
                    .smoothed_into(self, elapsed)
            },
            ..self
        }
    }
}
