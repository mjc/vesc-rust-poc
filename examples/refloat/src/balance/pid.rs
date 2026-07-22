use super::current::{PitchBasedCurrent, RequestedCurrent};
use super::loop_io::{LoopConfig, LoopInput, LoopState};
use crate::domain::{RefloatDarkRideState, RefloatRealtimeRuntimeSetpoint};
use vescpkg_rs::prelude::SampleRate;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, ElectricalSpeed, ImuRoll, MotorCurrent, MotorCurrentLimit,
};
use vescpkg_rs::{AngleCurrentGain, IntegralCurrentGain, PidScale, RateCurrentGain, Rpm};

/// Board setpoint error used by Refloat PID P/I terms.
///
/// Source map: upstream computes `setpoint - imu->balance_pitch` at
/// `third_party/refloat/src/pid.c:40`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct SetpointError(AngleDegrees);

impl SetpointError {
    #[inline(always)]
    const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    #[inline(always)]
    pub(super) fn from_input(
        setpoint: RefloatRealtimeRuntimeSetpoint,
        balance_pitch: AngleDegrees,
    ) -> Self {
        Self::new(setpoint.angle() - balance_pitch)
    }

    #[inline(always)]
    const fn angle(self) -> AngleDegrees {
        self.0
    }

    #[inline(always)]
    pub(super) fn integral_current(
        self,
        integral: MotorCurrent,
        ki: IntegralCurrentGain,
        limit: MotorCurrentLimit,
    ) -> MotorCurrent {
        // C map: `third_party/refloat/src/pid.c:40-46` integrates `p * ki`, then
        // clamps by `ki_limit` while preserving sign.
        let next = integral + self.angle() * ki;
        limit.clamp(next)
    }

    #[inline(always)]
    pub(super) fn angle_proportional_current(
        self,
        kp: AngleCurrentGain,
        accel_scale: PidScale,
        brake_scale: PidScale,
    ) -> MotorCurrent {
        // C map: `third_party/refloat/src/pid.c:69` applies KP and selects the
        // accel/brake scale from the sign of the setpoint error.
        let scale = ScaleSide::from_setpoint_error(self).scale(accel_scale, brake_scale);
        self.angle() * kp.scaled_by(scale)
    }
}

/// Refloat pitch-rate value after roll/yaw mixing and darkride sign handling.
///
/// Source map: upstream computes `imu->pitch_rate` at
/// `third_party/refloat/src/imu.c:46-53`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct PitchRate(AngularVelocity);

impl PitchRate {
    #[inline(always)]
    fn from_roll_corrected(rate: AngularVelocity, darkride: RefloatDarkRideState) -> Self {
        // C map: `imu_update` flips pitch rate when darkride is active at
        // `third_party/refloat/src/imu.c:52-54`.
        Self(match darkride {
            RefloatDarkRideState::Active => -rate,
            RefloatDarkRideState::Upright => rate,
        })
    }

    #[inline(always)]
    pub(super) fn from_imu(
        roll: ImuRoll,
        gyro_pitch: AngularVelocity,
        gyro_yaw: AngularVelocity,
        darkride: RefloatDarkRideState,
    ) -> Self {
        let pitch_rate = RollProjection::from_roll(roll).pitch_rate(gyro_pitch, gyro_yaw);

        Self::from_roll_corrected(pitch_rate, darkride)
    }

    #[inline(always)]
    pub(super) const fn rate(self) -> AngularVelocity {
        self.0
    }

    #[inline(always)]
    fn damping_current(self, kp2: RateCurrentGain) -> MotorCurrent {
        // C map: `third_party/refloat/src/pid.c:71` negates pitch rate before
        // multiplying by the rate gain.
        self.rate() * -kp2
    }

    #[inline(always)]
    pub(super) fn rate_proportional_current(
        self,
        kp2: RateCurrentGain,
        accel_scale: PidScale,
        brake_scale: PidScale,
    ) -> MotorCurrent {
        let rate_p = self.damping_current(kp2);
        // C map: `third_party/refloat/src/pid.c:72` picks accel/brake scale
        // from the sign of `rate_p`.
        let scale = ScaleSide::from_current(rate_p).scale(accel_scale, brake_scale);

        rate_p * scale.value()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RollProjection {
    sin: f32,
    cos: f32,
}

impl RollProjection {
    #[inline(always)]
    fn from_roll(roll: ImuRoll) -> Self {
        // C map: `imu_update` uses raw roll radians for the pitch-rate yaw
        // projection at `third_party/refloat/src/imu.c:46-51`.
        let roll_radians = roll.angle().as_radians();
        Self {
            sin: sin(roll_radians),
            cos: cos(roll_radians),
        }
    }

    #[inline(always)]
    fn pitch_rate(self, gyro_pitch: AngularVelocity, gyro_yaw: AngularVelocity) -> AngularVelocity {
        // C map: `third_party/refloat/src/imu.c:49-51` damps yaw influence
        // on pitch-rate while the board is rolled.
        let Self { sin, cos } = self;
        gyro_pitch * (cos * cos) + gyro_yaw * (sin * cos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleDirection {
    Coasting,
    Forward,
    Reverse,
}

impl ScaleDirection {
    #[inline(always)]
    fn from_motor_erpm(motor_erpm: ElectricalSpeed) -> Self {
        // C map: PID scale smoothing returns to unity below 500 ERPM, then
        // chooses forward/reverse scaling at `third_party/refloat/src/pid.c:48-67`.
        let erpm = motor_erpm.rpm();
        if erpm.abs() < Rpm::from_revolutions_per_minute(500.0) {
            Self::Coasting
        } else if erpm.is_positive() {
            Self::Forward
        } else {
            Self::Reverse
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    const fn new(angle_proportional: PidScale, rate_damping: PidScale) -> Self {
        Self {
            angle_proportional,
            rate_damping,
        }
    }

    #[inline(always)]
    fn smoothed_angle_proportional(self, current: PidScale) -> PidScale {
        // C map: `third_party/refloat/src/pid.c:51-66` uses a 1% target / 99%
        // previous one-pole filter for all PID scale coefficients.
        PidScale::new(self.angle_proportional.value() * 0.01 + current.value() * 0.99)
    }

    #[inline(always)]
    fn smoothed_rate_damping(self, current: PidScale) -> PidScale {
        // C map: `third_party/refloat/src/pid.c:51-66` uses the same 1% / 99%
        // filter for angle-P and rate-P scale coefficients.
        PidScale::new(self.rate_damping.value() * 0.01 + current.value() * 0.99)
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

    #[inline(always)]
    fn smoothed_into(self, state: LoopState) -> LoopState {
        // C map: `third_party/refloat/src/pid.c:51-66` smooths brake and accel
        // PID scale pairs back into the stored loop state.
        LoopState {
            pid_kp_brake_scale: self
                .brake
                .smoothed_angle_proportional(state.pid_kp_brake_scale),
            pid_kp2_brake_scale: self.brake.smoothed_rate_damping(state.pid_kp2_brake_scale),
            pid_kp_accel_scale: self
                .accel
                .smoothed_angle_proportional(state.pid_kp_accel_scale),
            pid_kp2_accel_scale: self.accel.smoothed_rate_damping(state.pid_kp2_accel_scale),
            ..state
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleSide {
    Accel,
    Brake,
}

impl ScaleSide {
    #[inline(always)]
    fn from_setpoint_error(error: SetpointError) -> Self {
        // C map: `third_party/refloat/src/pid.c:69` picks accel vs brake scale
        // from the sign of the setpoint error.
        if error.angle().is_positive() {
            Self::Accel
        } else {
            Self::Brake
        }
    }

    #[inline(always)]
    fn from_current(current: MotorCurrent) -> Self {
        // C map: `third_party/refloat/src/pid.c:72` picks accel vs brake scale
        // from the sign of the rate-P current contribution.
        if current.current().is_positive() {
            Self::Accel
        } else {
            Self::Brake
        }
    }

    #[inline(always)]
    const fn scale(self, accel_scale: PidScale, brake_scale: PidScale) -> PidScale {
        // C map: `third_party/refloat/src/pid.c:69-72` chooses between the
        // accel and brake scale coefficients after the sign check.
        match self {
            Self::Accel => accel_scale,
            Self::Brake => brake_scale,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Currents {
    angle_proportional: MotorCurrent,
    rate_damping: MotorCurrent,
    integral: MotorCurrent,
}

impl Currents {
    #[inline(always)]
    pub(super) fn pitch_based_current(
        self,
        booster_current: MotorCurrent,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrentLimit,
        hertz: SampleRate,
    ) -> PitchBasedCurrent {
        PitchBasedCurrent::from_rate_and_booster(
            self.rate_damping,
            booster_current,
            softstart_pid_limit,
            motor_current_max,
            hertz,
        )
    }

    #[inline(always)]
    pub(super) fn requested_with_pitch_based(
        self,
        pitch_based: PitchBasedCurrent,
    ) -> RequestedCurrent {
        RequestedCurrent(self.angle_proportional + self.integral + pitch_based.current)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SideScale {
    accel: PidScale,
    brake: PidScale,
}

impl SideScale {
    #[inline(always)]
    const fn new(accel: PidScale, brake: PidScale) -> Self {
        Self { accel, brake }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CurrentScales {
    angle_proportional: SideScale,
    rate_damping: SideScale,
}

impl CurrentScales {
    #[inline(always)]
    const fn from_state(state: LoopState) -> Self {
        // C map: `third_party/refloat/src/pid.c:51-66` keeps separate accel
        // and brake PID scale pairs for angle-P and rate-P smoothing.
        Self {
            angle_proportional: SideScale::new(state.pid_kp_accel_scale, state.pid_kp_brake_scale),
            rate_damping: SideScale::new(state.pid_kp2_accel_scale, state.pid_kp2_brake_scale),
        }
    }

    #[inline(always)]
    fn angle_proportional_current(
        self,
        error: SetpointError,
        kp: AngleCurrentGain,
    ) -> MotorCurrent {
        error.angle_proportional_current(
            kp,
            self.angle_proportional.accel,
            self.angle_proportional.brake,
        )
    }

    #[inline(always)]
    fn rate_damping_current(self, pitch_rate: PitchRate, kp2: RateCurrentGain) -> MotorCurrent {
        pitch_rate.rate_proportional_current(kp2, self.rate_damping.accel, self.rate_damping.brake)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Phase {
    config: LoopConfig,
    input: LoopInput,
}

impl Phase {
    #[inline(always)]
    pub(super) const fn from_step(config: LoopConfig, input: LoopInput) -> Self {
        Self { config, input }
    }

    #[inline(always)]
    pub(super) fn update_state(self, state: LoopState) -> (Currents, LoopState) {
        // C map: `third_party/refloat/src/pid.c:37-73` updates P/I/rate-P before
        // smoothing the accel/brake scale coefficients for the next tick.
        let config = self.config;
        let current_scales = CurrentScales::from_state(state);
        let setpoint_error = self.input.setpoint_error();
        let pitch_rate = self.input.pitch_rate();
        let currents = Currents {
            angle_proportional: current_scales
                .angle_proportional_current(setpoint_error, config.kp),
            rate_damping: current_scales.rate_damping_current(pitch_rate, config.kp2),
            integral: setpoint_error.integral_current(
                state.pid_integral_current,
                config.ki,
                config.ki_limit,
            ),
        };
        let state =
            state.with_updated_pid_state(self.config, self.input.motor_erpm, currents.integral);

        (currents, state)
    }
}

impl LoopInput {
    #[inline(always)]
    pub(super) fn setpoint_error(&self) -> SetpointError {
        SetpointError::from_input(self.setpoint, self.balance_pitch)
    }

    #[inline(always)]
    pub(super) fn pitch_rate(&self) -> PitchRate {
        PitchRate::from_imu(self.roll, self.gyro_pitch, self.gyro_yaw, self.darkride)
    }
}

impl LoopState {
    /// Source map: upstream stores integral current and smooths PID scales at
    /// `third_party/refloat/src/pid.c:40-67`.
    #[inline(always)]
    pub(super) fn with_updated_pid_state(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        integral: MotorCurrent,
    ) -> Self {
        Self {
            pid_integral_current: integral,
            ..ScaleDirection::from_motor_erpm(motor_erpm)
                .targets(config)
                .smoothed_into(self)
        }
    }
}
use vescpkg_rs::{cos, sin};
