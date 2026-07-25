//! Battery and input semantic wrappers.

use crate::units::{Charge, Current, Energy, Voltage};

macro_rules! current_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Current);

        impl $name {
            /// Wrap a generic current with battery/input domain meaning.
            pub const fn new(current: Current) -> Self {
                Self(current)
            }

            /// Return the typed current without erasing it to a primitive.
            pub const fn current(self) -> Current {
                self.0
            }
        }
    };
}

macro_rules! voltage_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Voltage);

        impl $name {
            /// Wrap a generic voltage with battery/input domain meaning.
            pub const fn new(voltage: Voltage) -> Self {
                Self(voltage)
            }

            /// Return the typed voltage without erasing it to a primitive.
            pub const fn voltage(self) -> Voltage {
                self.0
            }
        }
    };
}

macro_rules! energy_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Energy);

        impl $name {
            /// Wrap a generic energy value with battery domain meaning.
            pub const fn new(energy: Energy) -> Self {
                Self(energy)
            }

            /// Return the typed energy without erasing it to a primitive.
            pub const fn energy(self) -> Energy {
                self.0
            }
        }
    };
}

macro_rules! charge_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Charge);

        impl $name {
            /// Wrap a generic charge value with battery domain meaning.
            pub const fn new(charge: Charge) -> Self {
                Self(charge)
            }

            /// Return the typed charge without erasing it to a primitive.
            pub const fn charge(self) -> Charge {
                self.0
            }
        }
    };
}

current_type!(BatteryCurrent, "Battery-side current.");
current_type!(InputCurrent, "Controller input current.");
voltage_type!(InputVoltage, "Controller input voltage.");
voltage_type!(BatteryVoltage, "Battery pack voltage.");
voltage_type!(CellVoltage, "Battery cell voltage.");
energy_type!(WattHoursDischarged, "Discharged watt-hours.");
energy_type!(WattHoursCharged, "Charged watt-hours.");
energy_type!(WattHoursRemaining, "Remaining watt-hours.");
charge_type!(AmpHoursDischarged, "Discharged amp-hours.");
charge_type!(AmpHoursCharged, "Charged amp-hours.");

/// Firmware-reported battery state-of-charge fraction.
///
/// VESC can report values above `1.0` when the measured voltage exceeds the
/// configured full-voltage threshold, so this is intentionally not a bounded
/// [`crate::Ratio`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BatteryLevel(f32);

impl BatteryLevel {
    /// Preserve the raw fraction reported by firmware.
    #[must_use]
    pub const fn from_fraction(fraction: f32) -> Self {
        Self(fraction)
    }

    /// Return the raw firmware fraction.
    #[must_use]
    pub const fn as_fraction(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::BatteryLevel;

    #[test]
    fn battery_level_preserves_firmware_fraction_above_one() {
        let level = BatteryLevel::from_fraction(1.1);

        assert_f32_eq!(level.as_fraction(), 1.1);
    }
}
