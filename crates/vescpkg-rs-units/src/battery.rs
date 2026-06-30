//! Energy, charge, and efficiency unit newtypes.

use core::ops::Div;

use crate::motion::Distance;
use crate::scalar_unit;

scalar_unit!(Energy, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(Charge, from_amp_hours, as_amp_hours, "amp-hours");
scalar_unit!(WattHours, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(AmpHours, from_amp_hours, as_amp_hours, "amp-hours");
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

impl WattHours {
    /// Create a watt-hour value from joules.
    pub const fn from_joules(value: f32) -> Self {
        Self::from_watt_hours(value / 3600.0)
    }

    /// Return this watt-hour value in joules.
    pub const fn as_joules(self) -> f32 {
        self.as_watt_hours() * 3600.0
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
