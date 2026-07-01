//! Motion, distance, angle, and count unit newtypes.

use core::ops::{Div, Mul};

use crate::time::{SystemTicks, system_ticks_as_secs_f32};
use crate::{scalar_int_unit, scalar_unit};

scalar_unit!(
    MechanicalRpm,
    from_revolutions_per_minute,
    as_revolutions_per_minute,
    "mechanical revolutions per minute"
);
scalar_unit!(
    ElectricalRpm,
    from_revolutions_per_minute,
    as_revolutions_per_minute,
    "electrical revolutions per minute"
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

/// Unitless attitude quaternion components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Quaternion([f32; 4]);

impl Quaternion {
    /// Create a quaternion from components in firmware order `[q0, q1, q2, q3]`.
    pub const fn from_components(components: [f32; 4]) -> Self {
        Self(components)
    }

    /// Return quaternion components in firmware order.
    pub const fn components(self) -> [f32; 4] {
        self.0
    }
}

impl Speed {
    /// Create a speed value from kilometers per hour.
    pub const fn from_kilometers_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value / 3.6)
    }

    /// Return this speed value in kilometers per hour.
    pub const fn as_kilometers_per_hour(self) -> f32 {
        self.as_meters_per_second() * 3.6
    }

    /// Create a speed value from miles per hour.
    pub const fn from_miles_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value * 0.447_04)
    }

    /// Return this speed value in miles per hour.
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
