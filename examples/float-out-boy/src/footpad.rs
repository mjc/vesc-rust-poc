//! Float Out Boy footpad support types.
//!
//! These types model the decoded footpad sensor state and sample values.
//! Raw ADC interpretation stays at the footpad/runtime boundary.
//!
//! Source anchors for the compatibility surface below are Float Out Boy `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/float-out-boy/src/footpad_sensor.c:28-31` stores raw ADC1/ADC2 readings.

use vescpkg_rs::prelude::Voltage;

/// Float Out Boy footpad sensor state.
///
/// C map: `third_party/float-out-boy/src/footpad_sensor.h:22-27`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyFootpadState {
    /// No footpad sensor is active.
    #[default]
    None,
    /// Left footpad sensor is active.
    Left,
    /// Right footpad sensor is active.
    Right,
    /// Both footpad sensors are active.
    Both,
}

impl FloatOutBoyFootpadState {
    /// Return whether either footpad sensor is active.
    #[must_use]
    pub const fn is_pressed(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Return the Float Out Boy `v1.2.1` footpad state ID.
    ///
    /// C map: `third_party/float-out-boy/src/footpad_sensor.h:22-27`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Both => 3,
        }
    }

    /// Return the Float Out Boy app-data switch compatibility value.
    ///
    /// C map: `third_party/float-out-boy/src/footpad_sensor.c:63-73`.
    #[must_use]
    pub const fn switch_compat(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left | Self::Right => 1,
            Self::Both => 2,
        }
    }
}

/// Float Out Boy footpad ADC sample and decoded state.
///
/// C map: `adc_left`, `adc_right`, and `state` in
/// `third_party/float-out-boy/src/footpad_sensor.h:29-32`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FloatOutBoyFootpadSample {
    left_voltage: Voltage,
    right_voltage: Voltage,
    state: FloatOutBoyFootpadState,
}

impl FloatOutBoyFootpadSample {
    /// Build a footpad sample from Float Out Boy's logical left/right voltages.
    ///
    /// C map: the mapping-adjusted values are stored in
    /// `FootpadSensor.adc_left/adc_right`.
    #[must_use]
    pub const fn new(
        left_voltage: Voltage,
        right_voltage: Voltage,
        state: FloatOutBoyFootpadState,
    ) -> Self {
        Self {
            left_voltage,
            right_voltage,
            state,
        }
    }

    /// Return the logical left footpad voltage after ADC mapping.
    #[must_use]
    pub const fn left_voltage(self) -> Voltage {
        self.left_voltage
    }

    /// Return the logical right footpad voltage after ADC mapping.
    #[must_use]
    pub const fn right_voltage(self) -> Voltage {
        self.right_voltage
    }

    /// Return the decoded footpad sensor state.
    ///
    /// C map: `third_party/float-out-boy/src/footpad_sensor.h:29-32`.
    #[must_use]
    pub const fn state(self) -> FloatOutBoyFootpadState {
        self.state
    }
}

/// Mapping from physical ADC pins to logical footpad sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatOutBoyFootpadAdcMapping {
    /// ADC1 is left and ADC2 is right.
    #[default]
    Direct,
    /// ADC2 is left and ADC1 is right.
    Swapped,
}

impl FloatOutBoyFootpadAdcMapping {
    /// Decode the generated `hardware.swap_footpad_adcs` flag.
    #[must_use]
    pub const fn from_swapped(swapped: bool) -> Self {
        if swapped { Self::Swapped } else { Self::Direct }
    }

    /// Map physical ADC1/ADC2 voltages to logical left/right voltages.
    #[must_use]
    pub const fn logical_voltages(self, adc1: Voltage, adc2: Voltage) -> (Voltage, Voltage) {
        match self {
            Self::Direct => (adc1, adc2),
            Self::Swapped => (adc2, adc1),
        }
    }
}
