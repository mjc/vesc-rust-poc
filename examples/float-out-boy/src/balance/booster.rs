use super::loop_io::LoopConfig;
use super::loop_io::LoopInput;
use crate::domain::FloatOutBoyRealtimeRuntimeSetpoint;
use vescpkg_rs::Rpm;
use vescpkg_rs::prelude::{AngleDegrees, Current, ElectricalSpeed, MotorCurrent};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct Direction(f32);

impl Direction {
    #[inline]
    fn from_proportional(proportional: Proportional) -> Self {
        // C map: `booster_update` chooses boost direction from the sign of the
        // proportional angle at `third_party/float-out-boy/src/booster.c:63-72`.
        Self(proportional.angle().signum())
    }

    /// C map: `third_party/float-out-boy/src/booster.c:65-66`.
    #[inline]
    fn ramp_current(
        self,
        current: MotorCurrent,
        ramp_offset: RampOffset,
        ramp: AngleDegrees,
    ) -> MotorCurrent {
        current * ((ramp_offset.angle() * self.0) / ramp)
    }

    /// C map: `third_party/float-out-boy/src/booster.c:67-69`.
    #[inline]
    fn saturated_current(self, current: MotorCurrent) -> MotorCurrent {
        current * self.0
    }
}

/// Booster proportional angle supplied to upstream `booster_update`.
///
/// Source map: upstream computes this as
/// `d->setpoint - d->brake_tilt.setpoint - d->imu.pitch` at
/// `third_party/float-out-boy/src/main.c:921`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct Proportional(AngleDegrees);

impl Proportional {
    #[inline]
    pub(super) const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    #[inline]
    pub(super) fn from_input(
        setpoint: FloatOutBoyRealtimeRuntimeSetpoint,
        brake_tilt: FloatOutBoyRealtimeRuntimeSetpoint,
        raw_pitch: AngleDegrees,
    ) -> Self {
        Self::new(setpoint.angle() - brake_tilt.angle() - raw_pitch)
    }

    #[inline]
    pub(super) const fn angle(self) -> AngleDegrees {
        self.0
    }

    #[inline]
    fn direction(self) -> Direction {
        Direction::from_proportional(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Branch {
    Accel,
    Brake,
}

impl Branch {
    #[inline]
    pub(super) fn from_motor_current(motor_current: MotorCurrent) -> Self {
        // C map: `booster_update` selects accel vs brake booster path from
        // the sign of the current at `third_party/float-out-boy/src/booster.c:32-45`.
        if motor_current.current().is_negative() {
            Self::Brake
        } else {
            Self::Accel
        }
    }

    #[inline]
    pub(super) const fn is_braking(self) -> bool {
        matches!(self, Self::Brake)
    }

    #[inline]
    fn profile(self, config: LoopConfig) -> Profile {
        // C map: `third_party/float-out-boy/src/booster.c:32-45` picks the accel
        // or brake booster parameters before applying speed stiffness.
        match self {
            Self::Accel => Profile {
                current: config.booster_current,
                angle: config.booster_angle,
                ramp: config.booster_ramp,
            },
            Self::Brake => Profile {
                current: config.brkbooster_current,
                angle: config.brkbooster_angle,
                ramp: config.brkbooster_ramp,
            },
        }
    }

    /// Source map: upstream computes booster target current at
    /// `third_party/float-out-boy/src/booster.c:32-73`.
    #[inline]
    pub(super) fn target_current(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        proportional: Proportional,
    ) -> MotorCurrent {
        self.profile(config)
            .with_speed_stiffness(self, motor_erpm)
            .target_current(proportional)
    }

    /// Source map: upstream filters booster current at
    /// `third_party/float-out-boy/src/booster.c:74-75`.
    #[inline]
    pub(super) fn filtered_current(
        self,
        config: LoopConfig,
        motor_erpm: ElectricalSpeed,
        proportional: Proportional,
        previous: MotorCurrent,
    ) -> MotorCurrent {
        let target = self.target_current(config, motor_erpm, proportional);

        // C map: `third_party/float-out-boy/src/booster.c:74-75` uses the same 1%
        // one-pole filter shape as PID scale smoothing.
        target * 0.01 + previous * 0.99
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct RampOffset(AngleDegrees);

impl RampOffset {
    #[inline]
    fn from_profile(proportional: Proportional, profile: Profile) -> Self {
        // C map: `third_party/float-out-boy/src/booster.c:63-72` compares the
        // proportional angle against the deadband and ramp threshold.
        Self(proportional.angle().abs() - profile.angle)
    }

    #[inline]
    const fn angle(self) -> AngleDegrees {
        self.0
    }

    #[inline]
    fn range(self, ramp: AngleDegrees) -> Range {
        // C map: `third_party/float-out-boy/src/booster.c:63-72` uses deadband,
        // then ramp, then saturated current.
        if self.angle() <= AngleDegrees::ZERO {
            Range::Deadband
        } else if self.angle() < ramp {
            Range::Ramp(self)
        } else {
            Range::Saturated
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Range {
    Deadband,
    Ramp(RampOffset),
    Saturated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct SpeedStiffness(f32);

impl SpeedStiffness {
    #[inline]
    fn from_abs_erpm(abs_erpm: Rpm) -> Self {
        // C map: `third_party/float-out-boy/src/booster.c:48-51` starts stiffness
        // above 3000 ERPM and caps `(abs_erpm - 3000) / 10000` at 1.
        let start = Rpm::from_revolutions_per_minute(3000.0);
        let range = Rpm::from_revolutions_per_minute(10000.0);
        Self(((abs_erpm - start) / range).clamp(0.0, 1.0))
    }

    #[inline]
    fn from_motor_erpm(motor_erpm: ElectricalSpeed) -> Self {
        // C map: `third_party/float-out-boy/src/booster.c:48-60` applies no stiffness
        // below 3000 ERPM and reaches full stiffness 10000 ERPM later.
        Self::from_abs_erpm(motor_erpm.rpm().abs())
    }

    #[inline]
    fn boosted_current(self, current: MotorCurrent) -> MotorCurrent {
        // C map: braking adds `current * speedstiffness` at
        // `third_party/float-out-boy/src/booster.c:52-54`.
        current + current * self.0
    }

    #[inline]
    fn softened_angle(self, angle: AngleDegrees) -> AngleDegrees {
        // C map: accelerating divides start angle by `1 + speedstiffness` at
        // `third_party/float-out-boy/src/booster.c:55-59`.
        angle / (1.0 + self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Profile {
    pub(super) current: MotorCurrent,
    pub(super) angle: AngleDegrees,
    pub(super) ramp: AngleDegrees,
}

impl Profile {
    #[inline]
    fn zero_current() -> MotorCurrent {
        MotorCurrent::new(Current::ZERO)
    }

    #[inline]
    fn with_speed_stiffness(self, branch: Branch, motor_erpm: ElectricalSpeed) -> Self {
        // C map: `third_party/float-out-boy/src/booster.c:48-60` scales current or
        // start angle by speed before the booster ramp is applied.
        let stiffness = SpeedStiffness::from_motor_erpm(motor_erpm);
        match branch {
            Branch::Brake => Self {
                current: stiffness.boosted_current(self.current),
                ..self
            },
            Branch::Accel => Self {
                angle: stiffness.softened_angle(self.angle),
                ..self
            },
        }
    }

    #[inline]
    pub(super) fn target_current(self, proportional: Proportional) -> MotorCurrent {
        let direction = proportional.direction();

        // C map: `third_party/float-out-boy/src/booster.c:63-72` applies booster as
        // a deadband, then a linear ramp, then saturated current.
        match RampOffset::from_profile(proportional, self).range(self.ramp) {
            Range::Deadband => Self::zero_current(),
            Range::Ramp(offset) => direction.ramp_current(self.current, offset, self.ramp),
            Range::Saturated => direction.saturated_current(self.current),
        }
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
        // C map: `third_party/float-out-boy/src/booster.c:32-75` runs from the
        // current loop's config and live input.
        Self { config, input }
    }

    #[inline]
    pub(super) fn filtered_current(self, previous: MotorCurrent) -> MotorCurrent {
        // C map: `third_party/float-out-boy/src/booster.c:74-75` filters the newly
        // computed booster current with the previous sample.
        Branch::from_motor_current(self.input.motor_current).filtered_current(
            self.config,
            self.input.motor_erpm,
            self.input.booster_proportional(),
            previous,
        )
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn booster_proportional(self) -> Proportional {
        // C map: `booster_update` uses setpoint, brake tilt, and pitch to
        // compute the booster proportional angle at
        // `third_party/float-out-boy/src/main.c:921-922`.
        Proportional::from_input(self.setpoint, self.brake_tilt_setpoint, self.raw_pitch)
    }
}
