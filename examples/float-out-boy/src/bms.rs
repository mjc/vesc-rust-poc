//! Float Out Boy BMS support.
//!
//! This module owns Float Out Boy-specific BMS extension behavior.

#[cfg(any(test, target_arch = "arm"))]
use crate::package::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispArgs;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispValue;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{VescSeconds, Voltage};

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct FloatOutBoyBmsTemperature(i32);

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBmsTemperature {
    pub(crate) const fn from_degrees_celsius(degrees_celsius: i32) -> Self {
        Self(degrees_celsius)
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_config_byte(encoded: u8) -> Self {
        Self(i32::from(i8::from_be_bytes([encoded])))
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyBmsSample {
    cell_low_voltage: Voltage,
    cell_high_voltage: Voltage,
    cell_low_temperature: FloatOutBoyBmsTemperature,
    cell_high_temperature: FloatOutBoyBmsTemperature,
    bms_high_temperature: FloatOutBoyBmsTemperature,
    message_age: VescSeconds,
}

#[cfg(any(test, target_arch = "arm"))]
impl Default for FloatOutBoyBmsSample {
    fn default() -> Self {
        Self::source_startup()
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBmsSample {
    pub(crate) const fn new(
        cell_low_voltage: Voltage,
        cell_high_voltage: Voltage,
        cell_low_temperature: FloatOutBoyBmsTemperature,
        cell_high_temperature: FloatOutBoyBmsTemperature,
        bms_high_temperature: FloatOutBoyBmsTemperature,
        message_age: VescSeconds,
    ) -> Self {
        Self {
            cell_low_voltage,
            cell_high_voltage,
            cell_low_temperature,
            cell_high_temperature,
            bms_high_temperature,
            message_age,
        }
    }

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

    #[cfg(any(test, target_arch = "arm"))]
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

    #[cfg(any(test, target_arch = "arm"))]
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

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyBmsThresholds {
    cell_low_voltage: Voltage,
    cell_high_voltage: Voltage,
    cell_balance_voltage: Voltage,
    cell_low_temperature: FloatOutBoyBmsTemperature,
    cell_high_temperature: FloatOutBoyBmsTemperature,
    bms_high_temperature: FloatOutBoyBmsTemperature,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBmsThresholds {
    pub(crate) const fn new(
        cell_low_voltage: Voltage,
        cell_high_voltage: Voltage,
        cell_balance_voltage: Voltage,
        cell_low_temperature: FloatOutBoyBmsTemperature,
        cell_high_temperature: FloatOutBoyBmsTemperature,
        bms_high_temperature: FloatOutBoyBmsTemperature,
    ) -> Self {
        Self {
            cell_low_voltage,
            cell_high_voltage,
            cell_balance_voltage,
            cell_low_temperature,
            cell_high_temperature,
            bms_high_temperature,
        }
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyBmsFault {
    Connection,
    BmsOverTemperature,
    CellOverVoltage,
    CellUnderVoltage,
    CellOverTemperature,
    CellUnderTemperature,
    CellBalance,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBmsFault {
    const fn bit(self) -> u8 {
        match self {
            Self::Connection => 1 << 0,
            Self::BmsOverTemperature => 1 << 1,
            Self::CellOverVoltage => 1 << 2,
            Self::CellUnderVoltage => 1 << 3,
            Self::CellOverTemperature => 1 << 4,
            Self::CellUnderTemperature => 1 << 5,
            Self::CellBalance => 1 << 6,
        }
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct FloatOutBoyBmsFaults(u8);

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBmsFaults {
    pub(crate) const NONE: Self = Self(0);

    pub(crate) const fn from_fault(fault: FloatOutBoyBmsFault) -> Self {
        Self(fault.bit())
    }

    pub(crate) const fn contains(self, fault: FloatOutBoyBmsFault) -> bool {
        self.0 & fault.bit() != 0
    }

    pub(crate) fn evaluate(
        enabled: bool,
        sample: FloatOutBoyBmsSample,
        thresholds: FloatOutBoyBmsThresholds,
        startup_timeout_elapsed: bool,
    ) -> Self {
        if !enabled {
            return Self::NONE;
        }

        if sample.message_age > VescSeconds::from_seconds(5.0) && startup_timeout_elapsed {
            return Self::from_fault(FloatOutBoyBmsFault::Connection);
        }

        let mut faults = Self::NONE;
        if sample.cell_low_voltage < thresholds.cell_low_voltage {
            faults.insert(FloatOutBoyBmsFault::CellUnderVoltage);
        }
        if sample.cell_high_voltage > thresholds.cell_high_voltage {
            faults.insert(FloatOutBoyBmsFault::CellOverVoltage);
        }
        let zero_temperature = FloatOutBoyBmsTemperature::from_degrees_celsius(0);
        if thresholds.cell_high_temperature > zero_temperature {
            if sample.cell_high_temperature > thresholds.cell_high_temperature {
                faults.insert(FloatOutBoyBmsFault::CellOverTemperature);
            }
            if sample.cell_low_temperature < thresholds.cell_low_temperature {
                faults.insert(FloatOutBoyBmsFault::CellUnderTemperature);
            }
        }
        if thresholds.bms_high_temperature > zero_temperature
            && sample.bms_high_temperature > thresholds.bms_high_temperature
        {
            faults.insert(FloatOutBoyBmsFault::BmsOverTemperature);
        }
        if (sample.cell_low_voltage - sample.cell_high_voltage).abs()
            > thresholds.cell_balance_voltage
        {
            faults.insert(FloatOutBoyBmsFault::CellBalance);
        }
        faults
    }

    fn insert(&mut self, fault: FloatOutBoyBmsFault) {
        self.0 |= fault.bit();
    }
}

/// Called from Float Out Boy's Lisp loader and BMS polling loop.
///
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/float-out-boy/src/main.c:2319-2331`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct ExtBms;

#[cfg(any(test, target_arch = "arm"))]
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
