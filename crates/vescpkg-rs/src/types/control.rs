use core::ops::{Mul, Neg};

use crate::{AngleDegrees, AngularVelocity, Current, MotorCurrent};

/// Mahony pitch feedback gain.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct MahonyPitchGain(f32);

impl MahonyPitchGain {
    /// Create a Mahony pitch feedback gain.
    #[inline(always)]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Return the scalar gain used by the feedback filter.
    #[inline(always)]
    pub const fn value(self) -> f32 {
        self.0
    }
}

/// Mahony roll feedback gain.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct MahonyRollGain(f32);

impl MahonyRollGain {
    /// Create a Mahony roll feedback gain.
    #[inline(always)]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Return the scalar gain used by the feedback filter.
    #[inline(always)]
    pub const fn value(self) -> f32 {
        self.0
    }
}

/// Dimensionless scale applied to a motor-current control gain.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct PidScale(f32);

impl PidScale {
    /// Create a control-gain scale.
    #[inline(always)]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Return the dimensionless scale.
    #[inline(always)]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Apply another dimensionless control scale.
    #[inline(always)]
    pub const fn scaled_by(self, scale: Self) -> Self {
        Self(self.0 * scale.0)
    }

    /// Move this scale toward a target by a dimensionless fraction.
    #[inline(always)]
    pub const fn lerp(self, target: Self, amount: f32) -> Self {
        Self(self.0 + (target.0 - self.0) * amount)
    }
}

impl MotorCurrent {
    /// Apply a dimensionless control-gain scale without erasing the current type.
    #[inline(always)]
    pub const fn scaled_by(self, scale: PidScale) -> Self {
        Self::new(Current::from_amps(self.current().as_amps() * scale.0))
    }
}

/// Board-angle error to motor-current gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngleCurrentGain {
    amps_per_degree: f32,
    scale: PidScale,
}

impl AngleCurrentGain {
    /// Create a gain in amps per degree.
    #[inline(always)]
    pub const fn new(amps_per_degree: f32) -> Self {
        Self {
            amps_per_degree,
            scale: PidScale::new(1.0),
        }
    }

    /// Apply a dimensionless control-gain scale.
    #[inline(always)]
    pub const fn scaled_by(self, scale: PidScale) -> Self {
        Self { scale, ..self }
    }

    /// Return the gain in amps per degree.
    #[inline(always)]
    pub const fn as_amps_per_degree(self) -> f32 {
        self.amps_per_degree
    }
}

impl Mul<AngleCurrentGain> for AngleDegrees {
    type Output = MotorCurrent;

    #[inline(always)]
    fn mul(self, rhs: AngleCurrentGain) -> Self::Output {
        // C map: Float Out Boy multiplies degree error by `kp` and side scale at
        // `third_party/float-out-boy/src/pid.c:40-70`.
        MotorCurrent::new(Current::from_amps(
            self.as_degrees() * rhs.amps_per_degree * rhs.scale.value(),
        ))
    }
}

/// Angular-rate error to motor-current gain.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RateCurrentGain(f32);

impl RateCurrentGain {
    /// Create a gain in amps per degree per second.
    #[inline(always)]
    pub const fn new(amps_per_degree_per_second: f32) -> Self {
        Self(amps_per_degree_per_second)
    }

    /// Apply a dimensionless control-gain scale.
    #[inline(always)]
    pub const fn scaled_by(self, scale: PidScale) -> Self {
        Self(self.0 * scale.0)
    }

    /// Return the gain in amps per degree per second.
    #[inline(always)]
    pub const fn as_amps_per_degree_per_second(self) -> f32 {
        self.0
    }
}

impl Neg for RateCurrentGain {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Mul<RateCurrentGain> for AngularVelocity {
    type Output = MotorCurrent;

    #[inline(always)]
    fn mul(self, rhs: RateCurrentGain) -> Self::Output {
        // C map: Float Out Boy exposes degrees/second at
        // `third_party/float-out-boy/src/imu.c:43-53` and applies `kp2` at
        // `third_party/float-out-boy/src/pid.c:71-72`.
        MotorCurrent::new(Current::from_amps(self.as_degrees_per_second() * rhs.0))
    }
}

/// Per-update board-angle error to accumulated motor-current gain.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct IntegralCurrentGain(f32);

impl IntegralCurrentGain {
    /// Create a gain in amps per degree per control update.
    #[inline(always)]
    pub const fn new(amps_per_degree_per_update: f32) -> Self {
        Self(amps_per_degree_per_update)
    }

    /// Return the gain in amps per degree per control update.
    #[inline(always)]
    pub const fn as_amps_per_degree_per_tick(self) -> f32 {
        self.0
    }
}

impl Mul<IntegralCurrentGain> for AngleDegrees {
    type Output = MotorCurrent;

    #[inline(always)]
    fn mul(self, rhs: IntegralCurrentGain) -> Self::Output {
        // C map: Float Out Boy accumulates degree error and applies `ki` at
        // `third_party/float-out-boy/src/pid.c:40-73`.
        MotorCurrent::new(Current::from_amps(self.as_degrees() * rhs.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{MotorCurrent, PidScale, RateCurrentGain};
    use crate::Current;

    #[test]
    fn typed_control_arithmetic_preserves_domain_values() {
        assert_eq!(
            RateCurrentGain::new(2.0)
                .scaled_by(PidScale::new(0.25))
                .as_amps_per_degree_per_second(),
            0.5
        );
        assert_eq!(
            PidScale::new(1.0).lerp(PidScale::new(3.0), 0.01),
            PidScale::new(1.02)
        );
        assert_eq!(
            MotorCurrent::new(Current::from_amps(8.0))
                .scaled_by(PidScale::new(0.25))
                .current()
                .as_amps(),
            2.0
        );
    }
}
