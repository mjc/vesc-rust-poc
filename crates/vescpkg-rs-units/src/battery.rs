//! Energy and charge unit newtypes.

use crate::scalar_unit;

scalar_unit!(Energy, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(Charge, from_amp_hours, as_amp_hours, "amp-hours");
scalar_unit!(WattHours, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(AmpHours, from_amp_hours, as_amp_hours, "amp-hours");

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
