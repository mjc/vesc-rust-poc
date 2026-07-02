//! Refloat footpad support types.
//!
//! These types model the decoded footpad sensor state and sample values.
//! Raw ADC interpretation stays at the footpad/runtime boundary.
//!
//! Source anchors for the compatibility surface below are Refloat `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/refloat/src/footpad_sensor.c:28-31` stores raw ADC1/ADC2 readings.

use vescpkg_rs::prelude::{AdcDecodedLevel, Ratio, Voltage};

/// Refloat footpad sensor state.
///
/// C map: `third_party/refloat/src/footpad_sensor.h:22-27`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatFootpadState {
    /// No footpad sensor is active.
    None,
    /// Left footpad sensor is active.
    Left,
    /// Right footpad sensor is active.
    Right,
    /// Both footpad sensors are active.
    Both,
}

impl RefloatFootpadState {
    /// Return the Refloat `v1.2.1` footpad state ID.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.h:22-27`.
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Both => 3,
        }
    }

    /// Return the Refloat app-data switch compatibility value.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.c:63-73`.
    pub const fn switch_compat(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left | Self::Right => 1,
            Self::Both => 2,
        }
    }
}

/// Refloat footpad ADC sample and decoded state.
///
/// C map: `third_party/refloat/src/footpad_sensor.h:29-32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatFootpadSample {
    adc1: Voltage,
    adc2: Voltage,
    state: RefloatFootpadState,
}

impl RefloatFootpadSample {
    /// Build a footpad sensor sample from typed ADC levels and decoded state.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.c:22-32`.
    pub const fn new(
        adc1: AdcDecodedLevel,
        adc2: AdcDecodedLevel,
        state: RefloatFootpadState,
    ) -> Self {
        Self {
            adc1: Voltage::from_volts(adc1.ratio().as_ratio()),
            adc2: Voltage::from_volts(adc2.ratio().as_ratio()),
            state,
        }
    }

    /// Build a footpad sample from Refloat's raw ADC pin voltages.
    ///
    /// C map: Refloat v1.2.1 stores `VESC_IF->io_read_analog` results in
    /// `FootpadSensor.adc1/adc2` at `third_party/refloat/src/footpad_sensor.c:28-31`.
    pub const fn from_adc_volts(adc1: Voltage, adc2: Voltage, state: RefloatFootpadState) -> Self {
        Self { adc1, adc2, state }
    }

    /// Return the typed ADC1 level.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.h:29-32`.
    pub const fn adc1(self) -> AdcDecodedLevel {
        AdcDecodedLevel::new(Ratio::clamped(self.adc1.as_volts()))
    }

    /// Return the typed ADC2 level.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.h:29-32`.
    pub const fn adc2(self) -> AdcDecodedLevel {
        AdcDecodedLevel::new(Ratio::clamped(self.adc2.as_volts()))
    }

    /// Return Refloat's raw ADC1 voltage from `third_party/refloat/src/footpad_sensor.c:28-31`.
    pub const fn adc1_volts(self) -> f32 {
        self.adc1.as_volts()
    }

    /// Return Refloat's raw ADC2 voltage from `third_party/refloat/src/footpad_sensor.c:28-31`.
    pub const fn adc2_volts(self) -> f32 {
        self.adc2.as_volts()
    }

    /// Return the decoded footpad sensor state.
    ///
    /// C map: `third_party/refloat/src/footpad_sensor.h:29-32`.
    pub const fn state(self) -> RefloatFootpadState {
        self.state
    }
}

/// Backwards-compatible Refloat footpad state name.
///
/// C map: `third_party/refloat/src/footpad_sensor.h:22-27`.
pub type FootpadSensorState = RefloatFootpadState;

/// Backwards-compatible Refloat footpad sample name.
///
/// C map: `third_party/refloat/src/footpad_sensor.h:29-32`.
pub type FootpadSensorSample = RefloatFootpadSample;
