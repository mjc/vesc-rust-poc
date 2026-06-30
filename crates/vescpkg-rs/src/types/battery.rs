//! Battery and input semantic wrappers.

use crate::units::{Charge, Current, Voltage};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
/// Battery-side current.
pub struct BatteryCurrent(Current);

impl BatteryCurrent {
    /// Wrap a generic current as battery-side current.
    pub const fn new(current: Current) -> Self {
        Self(current)
    }

    /// Return the typed current without erasing it to a primitive.
    pub const fn current(self) -> Current {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
/// Controller input voltage.
pub struct InputVoltage(Voltage);

impl InputVoltage {
    /// Wrap a generic voltage as controller input voltage.
    pub const fn new(voltage: Voltage) -> Self {
        Self(voltage)
    }

    /// Return the typed voltage without erasing it to a primitive.
    pub const fn voltage(self) -> Voltage {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
/// Discharged battery charge.
pub struct AmpHoursDischarged(Charge);

impl AmpHoursDischarged {
    /// Wrap generic charge as discharged amp-hours.
    pub const fn new(charge: Charge) -> Self {
        Self(charge)
    }

    /// Return the typed charge without erasing it to a primitive.
    pub const fn charge(self) -> Charge {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
/// Charged battery charge.
pub struct AmpHoursCharged(Charge);

impl AmpHoursCharged {
    /// Wrap generic charge as charged amp-hours.
    pub const fn new(charge: Charge) -> Self {
        Self(charge)
    }

    /// Return the typed charge without erasing it to a primitive.
    pub const fn charge(self) -> Charge {
        self.0
    }
}
