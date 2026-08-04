//! Float Out Boy footpad protocol support types.
//!
//! These types model the decoded footpad sensor state and sample values.
//! Raw ADC interpretation stays at the footpad/runtime boundary.
//!
//! Source anchors for the compatibility surface below are Float Out Boy `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/float-out-boy/src/footpad_sensor.c:28-31` stores raw ADC1/ADC2 readings.

use vescpkg_rs::prelude::Voltage;

vesc_protocol::wire_enum! {
    /// Float Out Boy footpad sensor state.
    ///
    /// C map: `third_party/float-out-boy/src/footpad_sensor.h:22-27`.
    #[derive(Default)]
    pub enum FloatOutBoyFootpadState {
        /// No footpad sensor is active.
        #[default]
        None = 0,
        /// Left footpad sensor is active.
        Left = 1,
        /// Right footpad sensor is active.
        Right = 2,
        /// Both footpad sensors are active.
        Both = 3,
    }
}

impl FloatOutBoyFootpadState {
    /// Return whether either footpad sensor is active.
    #[must_use]
    pub const fn is_pressed(self) -> bool {
        !matches!(self, Self::None)
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
/// C map: `third_party/float-out-boy/src/footpad_sensor.h:29-32`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FloatOutBoyFootpadSample {
    adc1: Voltage,
    adc2: Voltage,
    state: FloatOutBoyFootpadState,
}

impl FloatOutBoyFootpadSample {
    /// Build a footpad sample from Float Out Boy's raw ADC pin voltages.
    ///
    /// C map: Float Out Boy v1.2.1 stores `VESC_IF->io_read_analog` results in
    /// `FootpadSensor.adc1/adc2` at `third_party/float-out-boy/src/footpad_sensor.c:28-31`.
    #[must_use]
    pub const fn new(adc1: Voltage, adc2: Voltage, state: FloatOutBoyFootpadState) -> Self {
        Self { adc1, adc2, state }
    }

    /// Return Float Out Boy's raw ADC1 voltage from `third_party/float-out-boy/src/footpad_sensor.c:28-31`.
    #[must_use]
    pub const fn adc1_volts(self) -> f32 {
        self.adc1.as_volts()
    }

    /// Return Float Out Boy's raw ADC2 voltage from `third_party/float-out-boy/src/footpad_sensor.c:28-31`.
    #[must_use]
    pub const fn adc2_volts(self) -> f32 {
        self.adc2.as_volts()
    }

    /// Return the decoded footpad sensor state.
    ///
    /// C map: `third_party/float-out-boy/src/footpad_sensor.h:29-32`.
    #[must_use]
    pub const fn state(self) -> FloatOutBoyFootpadState {
        self.state
    }
}
