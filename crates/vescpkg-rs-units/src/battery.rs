//! Energy, charge, efficiency units, and compatibility names.

use core::ops::Div;

use crate::motion::Distance;
use crate::scalar_unit;

const METERS_PER_KILOMETER: f32 = 1000.0;
const METERS_PER_MILE: f32 = 1609.344;

scalar_unit!(Energy, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(Charge, from_amp_hours, as_amp_hours, "amp-hours");
/// Compatibility alias for older package code; prefer [`Energy`] in new APIs.
pub type WattHours = Energy;
/// Compatibility alias for older package code; prefer [`Charge`] in new APIs.
pub type AmpHours = Charge;
scalar_unit!(
    EnergyPerDistance,
    from_watt_hours_per_meter,
    as_watt_hours_per_meter,
    "watt-hours per meter"
);
scalar_unit!(
    DistancePerEnergy,
    from_meters_per_watt_hour,
    as_meters_per_watt_hour,
    "meters per watt-hour"
);

impl Energy {
    /// Create an energy value from joules.
    pub const fn from_joules(value: f32) -> Self {
        Self::from_watt_hours(value / 3600.0)
    }

    /// Return this energy value in joules.
    pub const fn as_joules(self) -> f32 {
        self.as_watt_hours() * 3600.0
    }
}

impl EnergyPerDistance {
    /// Create an efficiency value from watt-hours per kilometer.
    ///
    /// Uses the canonical conversion of 1 kilometer = 1000 meters.
    pub const fn from_watt_hours_per_kilometer(value: f32) -> Self {
        Self::from_watt_hours_per_meter(value / METERS_PER_KILOMETER)
    }

    /// Return this efficiency value in watt-hours per kilometer.
    ///
    /// Uses the canonical conversion of 1 kilometer = 1000 meters.
    pub const fn as_watt_hours_per_kilometer(self) -> f32 {
        self.as_watt_hours_per_meter() * METERS_PER_KILOMETER
    }

    /// Create an efficiency value from watt-hours per mile.
    ///
    /// Uses the international mile conversion of 1 mile = 1609.344 meters.
    pub const fn from_watt_hours_per_mile(value: f32) -> Self {
        Self::from_watt_hours_per_meter(value / METERS_PER_MILE)
    }

    /// Return this efficiency value in watt-hours per mile.
    ///
    /// Uses the international mile conversion of 1 mile = 1609.344 meters.
    pub const fn as_watt_hours_per_mile(self) -> f32 {
        self.as_watt_hours_per_meter() * METERS_PER_MILE
    }
}

impl EnergyPerDistance {
    /// Create an efficiency value from watt-hours per kilometer.
    ///
    /// Uses the canonical conversion of 1 kilometer = 1000 meters.
    pub const fn from_watt_hours_per_kilometer(value: f32) -> Self {
        Self::from_watt_hours_per_meter(value / METERS_PER_KILOMETER)
    }

    /// Return this efficiency value in watt-hours per kilometer.
    ///
    /// Uses the canonical conversion of 1 kilometer = 1000 meters.
    pub const fn as_watt_hours_per_kilometer(self) -> f32 {
        self.as_watt_hours_per_meter() * METERS_PER_KILOMETER
    }

    /// Create an efficiency value from watt-hours per mile.
    ///
    /// Uses the international mile conversion of 1 mile = 1609.344 meters.
    pub const fn from_watt_hours_per_mile(value: f32) -> Self {
        Self::from_watt_hours_per_meter(value / METERS_PER_MILE)
    }

    /// Return this efficiency value in watt-hours per mile.
    ///
    /// Uses the international mile conversion of 1 mile = 1609.344 meters.
    pub const fn as_watt_hours_per_mile(self) -> f32 {
        self.as_watt_hours_per_meter() * METERS_PER_MILE
    }
}

impl Div<Distance> for Energy {
    type Output = EnergyPerDistance;

    fn div(self, rhs: Distance) -> Self::Output {
        EnergyPerDistance::from_watt_hours_per_meter(self.as_watt_hours() / rhs.as_meters())
    }
}

impl Div<Energy> for Distance {
    type Output = DistancePerEnergy;

    fn div(self, rhs: Energy) -> Self::Output {
        DistancePerEnergy::from_meters_per_watt_hour(self.as_meters() / rhs.as_watt_hours())
    }
}
