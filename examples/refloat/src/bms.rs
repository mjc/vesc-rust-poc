//! Refloat BMS support.
//!
//! This module owns Refloat-specific BMS extension behavior.

#[cfg(any(test, target_arch = "arm"))]
use crate::package::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispArgs;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispValue;
use vescpkg_rs::{VescSeconds, Voltage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct RefloatBmsTemperature(i32);

impl RefloatBmsTemperature {
    pub(crate) const fn from_degrees_celsius(degrees_celsius: i32) -> Self {
        Self(degrees_celsius)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBmsSample {
    cell_low_voltage: Voltage,
    cell_high_voltage: Voltage,
    cell_low_temperature: RefloatBmsTemperature,
    cell_high_temperature: RefloatBmsTemperature,
    bms_high_temperature: RefloatBmsTemperature,
    message_age: VescSeconds,
}

impl RefloatBmsSample {
    pub(crate) const fn new(
        cell_low_voltage: Voltage,
        cell_high_voltage: Voltage,
        cell_low_temperature: RefloatBmsTemperature,
        cell_high_temperature: RefloatBmsTemperature,
        bms_high_temperature: RefloatBmsTemperature,
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
            RefloatBmsTemperature::from_degrees_celsius(0),
            RefloatBmsTemperature::from_degrees_celsius(0),
            RefloatBmsTemperature::from_degrees_celsius(0),
            VescSeconds::from_seconds(42.0),
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn from_lisp_args(args: &LispArgs<'_>) -> Option<Self> {
        (args.len() > 5).then_some(())?;
        Some(Self::new(
            Voltage::from_volts(args.get(0)?.as_f32()?),
            Voltage::from_volts(args.get(1)?.as_f32()?),
            RefloatBmsTemperature::from_degrees_celsius(args.get(2)?.as_i32()?),
            RefloatBmsTemperature::from_degrees_celsius(args.get(3)?.as_i32()?),
            RefloatBmsTemperature::from_degrees_celsius(args.get(4)?.as_i32()?),
            VescSeconds::from_seconds(args.get(5)?.as_f32()?),
        ))
    }
}

/// Called from Refloat's Lisp loader and BMS polling loop.
///
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/refloat/src/main.c:2319-2331`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct ExtBms;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::StatefulLbmExtension for ExtBms {
    type State = RefloatPackageState;

    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
        let enabled = state.bms_enabled();
        if enabled && let Some(sample) = RefloatBmsSample::from_lisp_args(&args) {
            state.record_bms_sample(sample);
        }
        LispValue::boolean(enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtBms, RefloatBmsSample, RefloatBmsTemperature};
    use crate::config::REFLOAT_DEFAULT_CONFIG;
    use crate::package::test_support::sample_all_data_payloads;
    use crate::package::{RefloatCustomConfig, RefloatPackageState};
    use vescpkg_rs::{
        ConfigBytes, LispArgs, LispValue, StatefulCustomConfigCallback, StatefulLbmExtension,
        VescSeconds, Voltage,
    };

    fn sample() -> RefloatBmsSample {
        RefloatBmsSample::new(
            Voltage::from_volts(2.8),
            Voltage::from_volts(4.1),
            RefloatBmsTemperature::from_degrees_celsius(-2),
            RefloatBmsTemperature::from_degrees_celsius(43),
            RefloatBmsTemperature::from_degrees_celsius(55),
            VescSeconds::from_seconds(0.2),
        )
    }

    #[test]
    fn bms_state_starts_like_refloat_bms_init() {
        let state = RefloatPackageState::new(sample_all_data_payloads());

        assert_eq!(
            state.bms_sample_for_test(),
            RefloatBmsSample::source_startup()
        );
    }

    #[test]
    fn bms_state_records_one_typed_lisp_poll_sample() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let sample = sample();

        state.record_bms_sample(sample);

        assert_eq!(state.bms_sample_for_test(), sample);
    }

    #[test]
    fn ext_bms_returns_nil_when_bms_integration_is_disabled() {
        // Refloat returns `d->float_conf.bms.enabled` at
        // `third_party/refloat/src/main.c:2319-2331`.
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let args = LispArgs::empty();
        let nil = LispValue::nil();
        let value = ExtBms::call(&mut state, args);

        assert!(value == nil);
    }

    #[test]
    fn ext_bms_returns_true_when_bms_integration_is_enabled() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut config = REFLOAT_DEFAULT_CONFIG;
        // Generated Refloat v1.2.1 order places `bms.enabled` after the final
        // haptic field and before the BMS thresholds at settings.xml:4076-4082.
        config[265] = 1;
        assert!(RefloatCustomConfig::set_config(&mut state, ConfigBytes::new(&config)).is_ok());

        let value = ExtBms::call(&mut state, LispArgs::empty());

        assert!(value == LispValue::true_value());
    }
}
