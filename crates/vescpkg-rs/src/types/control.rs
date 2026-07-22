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
