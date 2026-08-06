use super::loop_io::{LoopConfig, LoopInput};
use vescpkg_rs::Rpm;
use vescpkg_rs::prelude::{AngleDegrees, Current, ElectricalSpeed, MotorCurrent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Branch {
    Accel,
    Brake,
}

impl Branch {
    #[inline]
    pub(super) fn from_motor_current(motor_current: MotorCurrent) -> Self {
        if motor_current.is_negative() {
            Self::Brake
        } else {
            Self::Accel
        }
    }

    /// Compute upstream's speed-adjusted deadband/ramp/saturated booster target.
    #[inline]
    pub(super) fn target_current(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        proportional: AngleDegrees,
    ) -> MotorCurrent {
        // C map: `booster.c:32-60` selects accel/brake parameters and applies
        // stiffness above 3000 ERPM, reaching full stiffness 10000 ERPM later.
        let (mut current, mut angle, ramp) = match self {
            Self::Accel => (
                config.booster_current,
                config.booster_angle,
                config.booster_ramp,
            ),
            Self::Brake => (
                config.brkbooster_current,
                config.brkbooster_angle,
                config.brkbooster_ramp,
            ),
        };
        let stiffness = ((motor_erpm.rpm().abs() - Rpm::from_revolutions_per_minute(3000.0))
            / Rpm::from_revolutions_per_minute(10000.0))
        .clamp(0.0, 1.0);
        match self {
            Self::Brake => current = current + current * stiffness,
            Self::Accel => angle = angle / (1.0 + stiffness),
        }

        // C map: `booster.c:63-72` applies a deadband, linear ramp, then
        // saturated current in the proportional angle's direction.
        let offset = proportional.abs() - angle;
        if offset <= AngleDegrees::ZERO {
            MotorCurrent::new(Current::ZERO)
        } else if offset < ramp {
            current * ((offset * proportional.signum()) / ramp)
        } else {
            current * proportional.signum()
        }
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn filtered_booster_current(
        self,
        config: LoopConfig,
        previous: MotorCurrent,
        elapsed: vescpkg_rs::prelude::VescSeconds,
    ) -> MotorCurrent {
        let branch = Branch::from_motor_current(self.motor_current);
        // C map: `main.c:921-922` subtracts brake tilt and raw pitch.
        let proportional =
            self.setpoint.angle() - self.brake_tilt_setpoint.angle() - self.raw_pitch;
        // C map: Refloat configures booster current as a 1 Hz EMA.
        let target = branch.target_current(config, self.motor_erpm, proportional);
        previous + (target - previous) * super::ema_alpha(1.0, elapsed)
    }
}
