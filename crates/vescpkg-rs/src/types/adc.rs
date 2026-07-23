//! ADC semantic wrappers.

use crate::units::{Ratio, Voltage};

/// Firmware ADC pin voltage.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct AdcVoltage(Voltage);

impl AdcVoltage {
    /// Wrap a generic voltage as a firmware ADC pin voltage.
    #[must_use]
    pub const fn new(voltage: Voltage) -> Self {
        Self(voltage)
    }

    /// Return the typed voltage without erasing it to a primitive.
    #[must_use]
    pub const fn voltage(self) -> Voltage {
        self.0
    }
}

/// Firmware decoded ADC level normalized to 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct AdcDecodedLevel(Ratio);

impl AdcDecodedLevel {
    /// Wrap a checked normalized ratio as a decoded ADC level.
    #[must_use]
    pub const fn new(level: Ratio) -> Self {
        Self(level)
    }

    /// Return the typed ratio without erasing it to a primitive.
    #[must_use]
    pub const fn ratio(self) -> Ratio {
        self.0
    }
}

/// Brake lever/input level decoded from ADC or app input.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BrakeLeverLevel(Ratio);

impl BrakeLeverLevel {
    /// Wrap a checked normalized ratio as a brake lever/input level.
    #[must_use]
    pub const fn new(level: Ratio) -> Self {
        Self(level)
    }

    /// Return the typed input level without erasing it to a primitive.
    #[must_use]
    pub const fn ratio(self) -> Ratio {
        self.0
    }
}

/// Brake switch/button input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrakeSwitch {
    /// Brake switch/button is inactive.
    Released,
    /// Brake switch/button is active.
    Pressed,
}
