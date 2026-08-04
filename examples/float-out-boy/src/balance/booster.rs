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

    #[inline]
    pub(super) const fn is_braking(self) -> bool {
        matches!(self, Self::Brake)
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

    #[inline]
    fn filtered_current(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        proportional: AngleDegrees,
        previous: MotorCurrent,
    ) -> MotorCurrent {
        // C map: `booster.c:74-75` uses a 1% target / 99% previous filter.
        self.target_current(config, motor_erpm, proportional) * 0.01 + previous * 0.99
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn booster_proportional(self) -> AngleDegrees {
        // C map: `main.c:921-922` subtracts brake tilt and raw pitch.
        self.setpoint.angle() - self.brake_tilt_setpoint.angle() - self.raw_pitch
    }

    #[inline]
    pub(super) fn filtered_booster_current(
        self,
        config: LoopConfig,
        previous: MotorCurrent,
    ) -> MotorCurrent {
        Branch::from_motor_current(self.motor_current).filtered_current(
            config,
            self.motor_erpm,
            self.booster_proportional(),
            previous,
        )
    }
}
