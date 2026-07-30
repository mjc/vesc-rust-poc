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
        message_age: VescSeconds => message_age,
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

bitflags::bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct FloatOutBoyBmsFaults: u8 {
        const CONNECTION = 1 << 0;
        const BMS_OVER_TEMPERATURE = 1 << 1;
        const CELL_OVER_VOLTAGE = 1 << 2;
        const CELL_UNDER_VOLTAGE = 1 << 3;
        const CELL_OVER_TEMPERATURE = 1 << 4;
        const CELL_UNDER_TEMPERATURE = 1 << 5;
        const CELL_BALANCE = 1 << 6;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyBmsStartupGrace {
    Active,
    Elapsed,
}

impl FloatOutBoyBmsStartupGrace {
    pub(crate) const fn from_elapsed(elapsed: bool) -> Self {
        if elapsed { Self::Elapsed } else { Self::Active }
    }
}

impl FloatOutBoyBmsFaults {
    pub(crate) fn evaluate(
        enabled: bool,
        sample: FloatOutBoyBmsSample,
        thresholds: FloatOutBoyBmsThresholds,
        startup_grace: FloatOutBoyBmsStartupGrace,
    ) -> Self {
        if !enabled {
            return Self::empty();
        }

        if sample.message_age() > VescSeconds::from_seconds(5.0) {
            return match startup_grace {
                FloatOutBoyBmsStartupGrace::Active => Self::empty(),
                FloatOutBoyBmsStartupGrace::Elapsed => Self::CONNECTION,
            };
        }

        let mut faults = Self::empty();
        if sample.cell_low_voltage() < thresholds.cell_low_voltage() {
            faults.insert(Self::CELL_UNDER_VOLTAGE);
        }
        if sample.cell_high_voltage() > thresholds.cell_high_voltage() {
            faults.insert(Self::CELL_OVER_VOLTAGE);
        }
        let zero_temperature = FloatOutBoyBmsTemperature::from_degrees_celsius(0);
        if thresholds.cell_high_temperature() > zero_temperature {
            if sample.cell_high_temperature() > thresholds.cell_high_temperature() {
                faults.insert(Self::CELL_OVER_TEMPERATURE);
            }
            if sample.cell_low_temperature() < thresholds.cell_low_temperature() {
                faults.insert(Self::CELL_UNDER_TEMPERATURE);
            }
        }
        if thresholds.bms_high_temperature() > zero_temperature
            && sample.bms_high_temperature() > thresholds.bms_high_temperature()
        {
            faults.insert(Self::BMS_OVER_TEMPERATURE);
        }
        if (sample.cell_low_voltage() - sample.cell_high_voltage()).abs()
            > thresholds.cell_balance_voltage()
        {
            faults.insert(Self::CELL_BALANCE);
        }
        faults
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
