//! Motion, distance, angle, and count unit newtypes.

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
