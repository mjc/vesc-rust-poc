//! Motion, distance, angle, and count unit newtypes.

use core::ops::{Div, Mul};

use crate::time::{SystemTicks, VescSeconds, system_ticks_as_secs_f32};
use crate::{scalar_int_unit, scalar_unit};

scalar_unit!(
    Rpm,
    from_revolutions_per_minute,
    as_revolutions_per_minute,
    "revolutions per minute"
);
scalar_unit!(Distance, from_meters, as_meters, "meters");
scalar_unit!(
    Speed,
    from_meters_per_second,
    as_meters_per_second,
    "meters per second"
);
scalar_unit!(AngleDegrees, from_degrees, as_degrees, "degrees");
scalar_unit!(AngleRadians, from_radians, as_radians, "radians");
scalar_unit!(AccelerationG, from_g, as_g, "g");
scalar_unit!(
    AngularVelocity,
    from_degrees_per_second,
    as_degrees_per_second,
    "degrees per second"
);
scalar_int_unit!(
    TachometerSteps,
    from_steps,
    as_steps,
    i32,
    "tachometer steps"
);
scalar_int_unit!(
    OdometerMeters,
    from_meters,
    as_meters,
    u64,
    "odometer meters"
);

const RADIANS_PER_DEGREE: f32 = core::f32::consts::PI / 180.0;
const DEGREES_PER_RADIAN: f32 = 180.0 / core::f32::consts::PI;

impl AngleDegrees {
    /// Create an angle value from radians.
    #[must_use]
    pub fn from_radians(value: f32) -> Self {
        Self::from_degrees(value * DEGREES_PER_RADIAN)
    }

    /// Return this angle value in radians.
    #[must_use]
    pub fn as_radians(self) -> f32 {
        self.as_degrees() * RADIANS_PER_DEGREE
    }
}

impl AngleRadians {
    /// Create an angle value from degrees.
    #[must_use]
    pub fn from_degrees(value: f32) -> Self {
        Self::from_radians(value * RADIANS_PER_DEGREE)
    }

    /// Return this angle value in degrees.
    #[must_use]
    pub fn as_degrees(self) -> f32 {
        self.as_radians() * DEGREES_PER_RADIAN
    }
}

impl AngularVelocity {
    /// Create an angular velocity from radians per second.
    #[must_use]
    pub fn from_radians_per_second(value: f32) -> Self {
        Self::from_degrees_per_second(value * DEGREES_PER_RADIAN)
    }

    /// Return this angular velocity in radians per second.
    #[must_use]
    pub fn as_radians_per_second(self) -> f32 {
        self.as_degrees_per_second() * RADIANS_PER_DEGREE
    }
}

impl Mul<VescSeconds> for AngularVelocity {
    type Output = AngleRadians;

    fn mul(self, rhs: VescSeconds) -> Self::Output {
        AngleRadians::from_radians(self.as_radians_per_second() * rhs.as_seconds())
    }
}

impl Mul<AngularVelocity> for VescSeconds {
    type Output = AngleRadians;

    fn mul(self, rhs: AngularVelocity) -> Self::Output {
        rhs * self
    }
}

impl From<AngleDegrees> for AngleRadians {
    fn from(angle: AngleDegrees) -> Self {
        Self::from_degrees(angle.as_degrees())
    }
}

impl From<AngleRadians> for AngleDegrees {
    fn from(angle: AngleRadians) -> Self {
        Self::from_radians(angle.as_radians())
    }
}

impl Speed {
    /// Create a speed value from kilometers per hour.
    #[must_use]
    pub const fn from_kilometers_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value / 3.6)
    }

    /// Return this speed value in kilometers per hour.
    #[must_use]
    pub const fn as_kilometers_per_hour(self) -> f32 {
        self.as_meters_per_second() * 3.6
    }

    /// Create a speed value from miles per hour.
    #[must_use]
    pub const fn from_miles_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value * 0.447_04)
    }

    /// Return this speed value in miles per hour.
    #[must_use]
    pub const fn as_miles_per_hour(self) -> f32 {
        self.as_meters_per_second() / 0.447_04
    }
}

impl Mul<SystemTicks> for Speed {
    type Output = Distance;

    fn mul(self, rhs: SystemTicks) -> Self::Output {
        Distance::from_meters(self.as_meters_per_second() * system_ticks_as_secs_f32(rhs))
    }
}

impl Div<SystemTicks> for Distance {
    type Output = Speed;

    fn div(self, rhs: SystemTicks) -> Self::Output {
        Speed::from_meters_per_second(self.as_meters() / system_ticks_as_secs_f32(rhs))
    }
}
