//! Float Out Boy BMS support.
//!
//! This module owns Float Out Boy-specific BMS extension behavior.

#[cfg(any(test, target_arch = "arm"))]
use crate::package::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispArgs;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispValue;
use vescpkg_rs::{VescSeconds, Voltage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct FloatOutBoyBmsTemperature(i32);

impl FloatOutBoyBmsTemperature {
    pub(crate) const fn from_degrees_celsius(degrees_celsius: i32) -> Self {
        Self(degrees_celsius)
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_config_byte(encoded: u8) -> Self {
        Self(i32::from(i8::from_be_bytes([encoded])))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyBmsSample {
    cell_low_voltage: Voltage,
    cell_high_voltage: Voltage,
    cell_low_temperature: FloatOutBoyBmsTemperature,
    cell_high_temperature: FloatOutBoyBmsTemperature,
    bms_high_temperature: FloatOutBoyBmsTemperature,
    message_age: VescSeconds,
}

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
    fn from_lisp_args(args: &LispArgs<'_>) -> Option<Self> {
        (args.len() > 5).then_some(())?;
        Some(Self::new(
            Voltage::from_volts(args.get(0)?.decode_number_as_f32()?),
            Voltage::from_volts(args.get(1)?.decode_number_as_f32()?),
            FloatOutBoyBmsTemperature::from_degrees_celsius(args.get(2)?.decode_number_as_i32()?),
            FloatOutBoyBmsTemperature::from_degrees_celsius(args.get(3)?.decode_number_as_i32()?),
            FloatOutBoyBmsTemperature::from_degrees_celsius(args.get(4)?.decode_number_as_i32()?),
            VescSeconds::from_seconds(args.get(5)?.decode_number_as_f32()?),
        ))
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
mod tests {
    use super::{
        ExtBms, FloatOutBoyBmsFault, FloatOutBoyBmsFaults, FloatOutBoyBmsSample,
        FloatOutBoyBmsTemperature, FloatOutBoyBmsThresholds,
    };
    use crate::config::{FLOAT_OUT_BOY_DEFAULT_CONFIG, FloatOutBoyConfigImage};
    use crate::package::test_support::sample_all_data_payloads;
    use crate::package::{FloatOutBoyCustomConfig, FloatOutBoyPackageState};
    use vescpkg_rs::{
        ConfigBytes, LispArgs, LispValue, StatefulCustomConfigCallback, StatefulLbmExtension,
        TimestampTicks, VescSeconds, Voltage,
    };

    fn sample() -> FloatOutBoyBmsSample {
        FloatOutBoyBmsSample::new(
            Voltage::from_volts(2.8),
            Voltage::from_volts(4.1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(-2),
            FloatOutBoyBmsTemperature::from_degrees_celsius(43),
            FloatOutBoyBmsTemperature::from_degrees_celsius(55),
            VescSeconds::from_seconds(0.2),
        )
    }

    fn thresholds() -> FloatOutBoyBmsThresholds {
        FloatOutBoyBmsThresholds::new(
            Voltage::from_volts(2.7),
            Voltage::from_volts(4.3),
            Voltage::from_volts(0.2),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(45),
            FloatOutBoyBmsTemperature::from_degrees_celsius(60),
        )
    }

    #[test]
    fn default_bms_thresholds_decode_like_float_out_boy_generated_config() {
        let config = FloatOutBoyConfigImage::defaults();

        assert_eq!(config.bms().thresholds(), thresholds());
    }

    #[test]
    fn bms_thresholds_decode_exact_generated_offsets_and_signed_temperatures() {
        let mut bytes = FLOAT_OUT_BOY_DEFAULT_CONFIG;
        bytes[266..268].copy_from_slice(&[0x0c, 0x1c]);
        bytes[268..270].copy_from_slice(&[0x10, 0x68]);
        bytes[270..272].copy_from_slice(&[0x00, 0x96]);
        bytes[272] = 50;
        bytes[273] = 0xf6;
        bytes[274] = 70;
        let config =
            FloatOutBoyConfigImage::from_serialized(&bytes).expect("valid Float Out Boy config");

        assert_eq!(
            config.bms().thresholds(),
            FloatOutBoyBmsThresholds::new(
                Voltage::from_volts(3.1),
                Voltage::from_volts(4.2),
                Voltage::from_volts(0.15),
                FloatOutBoyBmsTemperature::from_degrees_celsius(-10),
                FloatOutBoyBmsTemperature::from_degrees_celsius(50),
                FloatOutBoyBmsTemperature::from_degrees_celsius(70),
            )
        );
    }

    #[test]
    fn disabled_bms_clears_every_fault_like_float_out_boy_bms_update() {
        let faults = FloatOutBoyBmsFaults::evaluate(false, sample(), thresholds(), true);

        assert_eq!(faults, FloatOutBoyBmsFaults::NONE);
    }

    #[test]
    fn stale_bms_after_startup_timeout_reports_connection_only() {
        let stale = FloatOutBoyBmsSample::new(
            Voltage::from_volts(2.6),
            Voltage::from_volts(4.4),
            FloatOutBoyBmsTemperature::from_degrees_celsius(-1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(46),
            FloatOutBoyBmsTemperature::from_degrees_celsius(61),
            VescSeconds::from_seconds(6.0),
        );

        let faults = FloatOutBoyBmsFaults::evaluate(true, stale, thresholds(), true);

        assert_eq!(
            faults,
            FloatOutBoyBmsFaults::from_fault(FloatOutBoyBmsFault::Connection)
        );
    }

    #[test]
    fn stale_bms_during_startup_grace_does_not_report_connection() {
        let stale = FloatOutBoyBmsSample::new(
            Voltage::from_volts(4.0),
            Voltage::from_volts(4.1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(40),
            FloatOutBoyBmsTemperature::from_degrees_celsius(50),
            VescSeconds::from_seconds(6.0),
        );

        let faults = FloatOutBoyBmsFaults::evaluate(true, stale, thresholds(), false);

        assert_eq!(faults, FloatOutBoyBmsFaults::NONE);
    }

    #[test]
    fn message_at_exact_timeout_is_not_stale_like_float_out_boy() {
        let at_timeout = FloatOutBoyBmsSample::new(
            Voltage::from_volts(4.0),
            Voltage::from_volts(4.1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(40),
            FloatOutBoyBmsTemperature::from_degrees_celsius(50),
            VescSeconds::from_seconds(5.0),
        );

        let faults = FloatOutBoyBmsFaults::evaluate(true, at_timeout, thresholds(), true);

        assert_eq!(faults, FloatOutBoyBmsFaults::NONE);
    }

    #[test]
    fn bms_threshold_crossings_set_every_float_out_boy_fault() {
        let sample = FloatOutBoyBmsSample::new(
            Voltage::from_volts(2.6),
            Voltage::from_volts(4.4),
            FloatOutBoyBmsTemperature::from_degrees_celsius(-1),
            FloatOutBoyBmsTemperature::from_degrees_celsius(46),
            FloatOutBoyBmsTemperature::from_degrees_celsius(61),
            VescSeconds::ZERO,
        );

        let faults = FloatOutBoyBmsFaults::evaluate(true, sample, thresholds(), false);

        for fault in [
            FloatOutBoyBmsFault::BmsOverTemperature,
            FloatOutBoyBmsFault::CellOverVoltage,
            FloatOutBoyBmsFault::CellUnderVoltage,
            FloatOutBoyBmsFault::CellOverTemperature,
            FloatOutBoyBmsFault::CellUnderTemperature,
            FloatOutBoyBmsFault::CellBalance,
        ] {
            assert!(faults.contains(fault));
        }
        assert!(!faults.contains(FloatOutBoyBmsFault::Connection));
    }

    #[test]
    fn equal_or_disabled_bms_thresholds_do_not_fault() {
        let sample = FloatOutBoyBmsSample::new(
            Voltage::from_volts(3.0),
            Voltage::from_volts(4.0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(-20),
            FloatOutBoyBmsTemperature::from_degrees_celsius(80),
            FloatOutBoyBmsTemperature::from_degrees_celsius(80),
            VescSeconds::ZERO,
        );
        let thresholds = FloatOutBoyBmsThresholds::new(
            Voltage::from_volts(3.0),
            Voltage::from_volts(4.0),
            Voltage::from_volts(1.0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
            FloatOutBoyBmsTemperature::from_degrees_celsius(0),
        );

        let faults = FloatOutBoyBmsFaults::evaluate(true, sample, thresholds, false);

        assert_eq!(faults, FloatOutBoyBmsFaults::NONE);
    }

    #[test]
    fn bms_state_starts_like_float_out_boy_bms_init() {
        let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

        assert_eq!(
            state.bms_sample_for_test(),
            FloatOutBoyBmsSample::source_startup()
        );
    }

    #[test]
    fn bms_state_records_one_typed_lisp_poll_sample() {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        let sample = sample();

        state.record_bms_sample(sample);

        assert_eq!(state.bms_sample_for_test(), sample);
    }

    #[test]
    fn ext_bms_returns_nil_when_bms_integration_is_disabled() {
        // Float Out Boy returns `d->float_conf.bms.enabled` at
        // `third_party/float-out-boy/src/main.c:2319-2331`.
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        let args = LispArgs::empty();
        let nil = LispValue::nil();
        let value = ExtBms::call(&mut state, args);

        assert!(value == nil);
    }

    #[test]
    fn ext_bms_returns_true_when_bms_integration_is_enabled() {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        let mut config = FLOAT_OUT_BOY_DEFAULT_CONFIG;
        // Generated Float Out Boy v1.2.1 order places `bms.enabled` after the final
        // haptic field and before the BMS thresholds at settings.xml:4076-4082.
        config[265] = 1;
        assert!(FloatOutBoyCustomConfig::set_config(&mut state, ConfigBytes::new(&config)).is_ok());

        let value = ExtBms::call(&mut state, LispArgs::empty());

        assert!(value == LispValue::true_value());
    }

    #[test]
    fn runtime_bms_connection_fault_uses_float_out_boy_startup_timer_boundary() {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        let mut config = FLOAT_OUT_BOY_DEFAULT_CONFIG;
        config[265] = 1;
        assert!(FloatOutBoyCustomConfig::set_config(&mut state, ConfigBytes::new(&config)).is_ok());

        state.refresh_bms_runtime_state(TimestampTicks::from_ticks(10_000));
        assert!(
            !state
                .bms_faults_for_test()
                .contains(FloatOutBoyBmsFault::Connection)
        );

        state.refresh_bms_runtime_state(TimestampTicks::from_ticks(60_000));
        assert!(
            !state
                .bms_faults_for_test()
                .contains(FloatOutBoyBmsFault::Connection)
        );

        state.refresh_bms_runtime_state(TimestampTicks::from_ticks(60_001));
        assert_eq!(
            state.bms_faults_for_test(),
            FloatOutBoyBmsFaults::from_fault(FloatOutBoyBmsFault::Connection)
        );
    }
}
