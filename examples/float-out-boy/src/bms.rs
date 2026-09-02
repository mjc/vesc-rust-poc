//! Float Out Boy BMS support.
//!
//! This module owns Float Out Boy-specific BMS extension behavior.

use crate::package::FloatOutBoyPackageState;
use vescpkg_rs::LispArgs;
use vescpkg_rs::LispValue;
use vescpkg_rs::{VescSeconds, Voltage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct FloatOutBoyBmsTemperature(i32);

impl FloatOutBoyBmsTemperature {
    pub(crate) const fn from_degrees_celsius(degrees_celsius: i32) -> Self {
        Self(degrees_celsius)
    }

    pub(crate) fn from_config_byte(encoded: u8) -> Self {
        Self(i32::from(i8::from_be_bytes([encoded])))
    }
}

vescpkg_rs::typed_fields! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct FloatOutBoyBmsSample {
        cell_low_voltage: Voltage => cell_low_voltage,
        cell_high_voltage: Voltage => cell_high_voltage,
        cell_low_temperature: FloatOutBoyBmsTemperature => cell_low_temperature,
        cell_high_temperature: FloatOutBoyBmsTemperature => cell_high_temperature,
        bms_high_temperature: FloatOutBoyBmsTemperature => bms_high_temperature,
        message_age: VescSeconds => message_age => with_message_age,
    }
}

impl Default for FloatOutBoyBmsSample {
    fn default() -> Self {
        Self::source_startup()
    }
}

impl FloatOutBoyBmsSample {
    pub(crate) const fn source_startup() -> Self {
        Self::new(
            Voltage::ZERO,
            Voltage::ZERO,
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            VescSeconds::from_seconds(42.0),
        )
    }

    fn try_new(
        cell_low_voltage: f32,
        cell_high_voltage: f32,
        cell_low_temperature: i32,
        cell_high_temperature: i32,
        bms_high_temperature: i32,
        message_age: f32,
    ) -> Option<Self> {
        [cell_low_voltage, cell_high_voltage, message_age]
            .into_iter()
            .all(f32::is_finite)
            .then(|| {
                Self::new(
                    Voltage::from_volts(cell_low_voltage),
                    Voltage::from_volts(cell_high_voltage),
                    FloatOutBoyBmsTemperature::from_degrees_celsius(cell_low_temperature),
                    FloatOutBoyBmsTemperature::from_degrees_celsius(cell_high_temperature),
                    FloatOutBoyBmsTemperature::from_degrees_celsius(bms_high_temperature),
                    VescSeconds::from_seconds(message_age),
                )
            })
    }

    fn from_lisp_args(args: &LispArgs<'_>) -> Option<Self> {
        (args.len() > 5).then_some(())?;
        Self::try_new(
            args.get(0)?.decode_number_as_f32()?,
            args.get(1)?.decode_number_as_f32()?,
            args.get(2)?.decode_number_as_i32()?,
            args.get(3)?.decode_number_as_i32()?,
            args.get(4)?.decode_number_as_i32()?,
            args.get(5)?.decode_number_as_f32()?,
        )
    }
}

vescpkg_rs::typed_fields! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct FloatOutBoyBmsThresholds {
        cell_low_voltage: Voltage => cell_low_voltage,
        cell_high_voltage: Voltage => cell_high_voltage,
        cell_balance_voltage: Voltage => cell_balance_voltage,
        cell_low_temperature: FloatOutBoyBmsTemperature => cell_low_temperature,
        cell_high_temperature: FloatOutBoyBmsTemperature => cell_high_temperature,
        bms_high_temperature: FloatOutBoyBmsTemperature => bms_high_temperature,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FloatOutBoyBmsFault {
    Connection = 1 << 0,
    BmsOverTemperature = 1 << 1,
    CellOverVoltage = 1 << 2,
    CellUnderVoltage = 1 << 3,
    CellOverTemperature = 1 << 4,
    CellUnderTemperature = 1 << 5,
    CellBalance = 1 << 6,
}

impl FloatOutBoyBmsFault {
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the fault bit"
    )]
    const fn bit(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct FloatOutBoyBmsFaults(u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FloatOutBoyBmsIntegration {
    Disabled,
    Enabled(FloatOutBoyBmsThresholds),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyBmsConnectionMonitoring {
    Deferred,
    Armed,
}

impl FloatOutBoyBmsFaults {
    pub(crate) const NONE: Self = Self(0);

    pub(crate) const fn from_fault(fault: FloatOutBoyBmsFault) -> Self {
        Self(fault.bit())
    }

    pub(crate) const fn contains(self, fault: FloatOutBoyBmsFault) -> bool {
        self.0 & fault.bit() != 0
    }

    #[must_use]
    pub(crate) fn evaluate(
        integration: FloatOutBoyBmsIntegration,
        sample: FloatOutBoyBmsSample,
        connection_monitoring: FloatOutBoyBmsConnectionMonitoring,
    ) -> Self {
        let FloatOutBoyBmsIntegration::Enabled(thresholds) = integration else {
            return Self::NONE;
        };

        if sample.message_age > VescSeconds::from_seconds(5.0) {
            return match connection_monitoring {
                FloatOutBoyBmsConnectionMonitoring::Armed => {
                    Self::from_fault(FloatOutBoyBmsFault::Connection)
                }
                FloatOutBoyBmsConnectionMonitoring::Deferred => Self::NONE,
            };
        }

        let zero_temperature = FloatOutBoyBmsTemperature::from_degrees_celsius(0);
        let cell_temperature_faults_enabled = thresholds.cell_high_temperature() > zero_temperature;

        [
            (sample.cell_low_voltage() < thresholds.cell_low_voltage())
                .then_some(FloatOutBoyBmsFault::CellUnderVoltage),
            (sample.cell_high_voltage() > thresholds.cell_high_voltage())
                .then_some(FloatOutBoyBmsFault::CellOverVoltage),
            (cell_temperature_faults_enabled
                && sample.cell_high_temperature() > thresholds.cell_high_temperature())
            .then_some(FloatOutBoyBmsFault::CellOverTemperature),
            (cell_temperature_faults_enabled
                && sample.cell_low_temperature() < thresholds.cell_low_temperature())
            .then_some(FloatOutBoyBmsFault::CellUnderTemperature),
            (thresholds.bms_high_temperature() > zero_temperature
                && sample.bms_high_temperature() > thresholds.bms_high_temperature())
            .then_some(FloatOutBoyBmsFault::BmsOverTemperature),
            ((sample.cell_low_voltage() - sample.cell_high_voltage()).abs()
                > thresholds.cell_balance_voltage())
            .then_some(FloatOutBoyBmsFault::CellBalance),
        ]
        .into_iter()
        .flatten()
        .fold(Self::NONE, Self::with_fault)
    }

    const fn with_fault(self, fault: FloatOutBoyBmsFault) -> Self {
        Self(self.0 | fault.bit())
    }
}

/// Called from Float Out Boy's Lisp loader and BMS polling loop.
///
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/float-out-boy/src/main.c:2319-2331`.
pub(crate) struct ExtBms;

impl vescpkg_rs::StatefulLbmExtension for ExtBms {
    type State = FloatOutBoyPackageState;

    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
        let enabled = state.bms_enabled();
        if enabled && let Some(sample) = FloatOutBoyBmsSample::from_lisp_args(&args) {
            state.record_bms_sample(sample);
        }
        LispValue::boolean(enabled)
    }
}

#[cfg(test)]
mod tests;
