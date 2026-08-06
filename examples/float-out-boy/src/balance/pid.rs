use super::loop_io::{LoopConfig, LoopInput, LoopState, PidState};
use crate::domain::FloatOutBoyDarkRideState;
use vescpkg_rs::prelude::{AngularVelocity, ElectricalSpeed, ImuRoll, MotorCurrent, PidScale};
use vescpkg_rs::{Rpm, cos, sin};

/// Float Out Boy pitch rate after roll/yaw mixing and darkride sign handling.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct PitchRate(AngularVelocity);

impl PitchRate {
    #[inline]
    pub(super) fn from_imu(
        roll: ImuRoll,
        gyro_pitch: AngularVelocity,
        gyro_yaw: AngularVelocity,
        darkride: FloatOutBoyDarkRideState,
    ) -> Self {
        // C map: `imu_update` projects yaw through roll at `imu.c:46-51`, then
        // flips pitch rate for darkride at `imu.c:52-54`.
        let roll = roll.angle().as_radians();
        let sin = sin(roll);
        let cos = cos(roll);
        let rate = gyro_pitch * (cos * cos) + gyro_yaw * (sin * cos);
        Self(match darkride {
            FloatOutBoyDarkRideState::Active => -rate,
            FloatOutBoyDarkRideState::Upright => rate,
        })
    }

    #[inline]
    pub(super) const fn rate(self) -> AngularVelocity {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Currents {
    pub(super) angle_proportional: MotorCurrent,
    pub(super) rate_damping: MotorCurrent,
    pub(super) integral: MotorCurrent,
}

#[inline]
const fn selected_scale(positive: bool, accel: PidScale, brake: PidScale) -> PidScale {
    if positive { accel } else { brake }
}

impl LoopInput {
    #[inline]
    pub(super) fn pitch_rate(&self) -> PitchRate {
        PitchRate::from_imu(self.roll, self.gyro_pitch, self.gyro_yaw, self.darkride)
    }
}

impl LoopState {
    /// Update Float Out Boy's P/I/rate-P currents and scale state for one tick.
    #[inline]
    pub(super) fn update_pid(self, config: LoopConfig, input: LoopInput) -> (Currents, Self) {
        // C map: `pid.c:37-73` computes currents from the old scales, then smooths
        // the direction-dependent scale targets for the next tick.
        let error = input.setpoint.angle() - input.balance_pitch;
        let angle_scale = selected_scale(
            error.is_positive(),
            self.pid.kp_accel_scale,
            self.pid.kp_brake_scale,
        );
        let angle_proportional = error * config.kp.scaled_by(angle_scale);

        let rate_damping = input.pitch_rate().rate() * -config.kp2;
        let rate_scale = selected_scale(
            rate_damping.is_positive(),
            self.pid.kp2_accel_scale,
            self.pid.kp2_brake_scale,
        );
        let rate_damping = rate_damping.scaled_by(rate_scale);

        let integral = self.pid.integral_current + error * config.ki;
        let integral = if config.ki_limit.current().is_positive() {
            config.ki_limit.clamp(integral)
        } else {
            integral
        };
        let currents = Currents {
            angle_proportional,
            rate_damping,
            integral,
        };

        (
            currents,
            self.with_updated_pid_state(config, input.motor_erpm, integral),
        )
    }

    /// Store integral current and smooth PID scales like upstream `pid_update`.
    #[inline]
    pub(super) fn with_updated_pid_state(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        integral: MotorCurrent,
    ) -> Self {
        let unity = PidScale::new(1.0);
        let erpm = motor_erpm.rpm();
        let ((brake_angle_target, brake_rate_target), (accel_angle_target, accel_rate_target)) =
            if erpm.abs() < Rpm::from_revolutions_per_minute(500.0) {
                ((unity, unity), (unity, unity))
            } else if erpm.is_positive() {
                ((config.kp_brake, config.kp2_brake), (unity, unity))
            } else {
                ((unity, unity), (config.kp_brake, config.kp2_brake))
            };
        Self {
            pid: PidState {
                integral_current: integral,
                kp_brake_scale: self.pid.kp_brake_scale.lerp(brake_angle_target, 0.01),
                kp2_brake_scale: self.pid.kp2_brake_scale.lerp(brake_rate_target, 0.01),
                kp_accel_scale: self.pid.kp_accel_scale.lerp(accel_angle_target, 0.01),
                kp2_accel_scale: self.pid.kp2_accel_scale.lerp(accel_rate_target, 0.01),
            },
            ..self
        }
    }
}
