use super::{
    ExtBms, FloatOutBoyBmsFaults, FloatOutBoyBmsSample, FloatOutBoyBmsTemperature,
    FloatOutBoyBmsThresholds,
};
use crate::config::{FLOAT_OUT_BOY_DEFAULT_CONFIG, FloatOutBoyConfigImage};
use crate::package::test_support::sample_all_data_payloads;
use crate::package::{FloatOutBoyPackageState, set_float_out_boy_custom_config_for_test};
use vescpkg_rs::{LispArgs, LispValue, StatefulLbmExtension, TimestampTicks, VescSeconds, Voltage};

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

fn enabled_state() -> FloatOutBoyPackageState {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut config = FLOAT_OUT_BOY_DEFAULT_CONFIG;
    // Generated Float Out Boy v1.2.1 order places `bms.enabled` after the final
    // haptic field and before the BMS thresholds at settings.xml:4076-4082.
    config[265] = 1;
    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &config
    ));
    state
}

fn encoded_sample() -> [LispValue; 6] {
    [
        LispValue::from_i32(3),
        LispValue::from_i32(4),
        LispValue::from_i32(-2),
        LispValue::from_i32(43),
        LispValue::from_i32(55),
        LispValue::from_i32(1),
    ]
}

fn decoded_integer_sample() -> FloatOutBoyBmsSample {
    FloatOutBoyBmsSample::new(
        Voltage::from_volts(3.0),
        Voltage::from_volts(4.0),
        FloatOutBoyBmsTemperature::from_degrees_celsius(-2),
        FloatOutBoyBmsTemperature::from_degrees_celsius(43),
        FloatOutBoyBmsTemperature::from_degrees_celsius(55),
        VescSeconds::from_seconds(1.0),
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

    assert_eq!(faults, FloatOutBoyBmsFaults::empty());
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

    assert_eq!(faults, FloatOutBoyBmsFaults::CONNECTION);
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

    assert_eq!(faults, FloatOutBoyBmsFaults::empty());
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

    assert_eq!(faults, FloatOutBoyBmsFaults::empty());
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
        FloatOutBoyBmsFaults::BMS_OVER_TEMPERATURE,
        FloatOutBoyBmsFaults::CELL_OVER_VOLTAGE,
        FloatOutBoyBmsFaults::CELL_UNDER_VOLTAGE,
        FloatOutBoyBmsFaults::CELL_OVER_TEMPERATURE,
        FloatOutBoyBmsFaults::CELL_UNDER_TEMPERATURE,
        FloatOutBoyBmsFaults::CELL_BALANCE,
    ] {
        assert!(faults.contains(fault));
    }
    assert!(!faults.contains(FloatOutBoyBmsFaults::CONNECTION));
    assert_eq!(FloatOutBoyBmsFaults::all().bits(), 0x7f);
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

    assert_eq!(faults, FloatOutBoyBmsFaults::empty());
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

    assert_eq!(value, nil);
}

#[test]
fn ext_bms_returns_true_when_bms_integration_is_enabled() {
    let mut state = enabled_state();

    let value = ExtBms::call(&mut state, LispArgs::empty());

    assert_eq!(value, LispValue::true_value());
}

#[test]
fn ext_bms_complete_sample_replaces_every_field_atomically() {
    let mut state = enabled_state();
    let values = encoded_sample();

    let value = ExtBms::call(&mut state, LispArgs::from_values(&values));

    assert_eq!(value, LispValue::true_value());
    assert_eq!(state.bms_sample_for_test(), decoded_integer_sample());
}

#[test]
fn ext_bms_short_calls_do_not_partially_replace_the_sample() {
    let values = encoded_sample();
    for len in 0..6 {
        let mut state = enabled_state();
        let before = state.bms_sample_for_test();

        let value = ExtBms::call(&mut state, LispArgs::from_values(&values[..len]));

        assert_eq!(value, LispValue::true_value());
        assert_eq!(state.bms_sample_for_test(), before, "argument count {len}");
    }
}

#[test]
fn ext_bms_ignores_extra_arguments_like_refloat() {
    let mut state = enabled_state();
    let sample_values = encoded_sample();
    let mut values = [LispValue::nil(); 7];
    values[..6].copy_from_slice(&sample_values);

    let value = ExtBms::call(&mut state, LispArgs::from_values(&values));

    assert_eq!(value, LispValue::true_value());
    assert_eq!(state.bms_sample_for_test(), decoded_integer_sample());
}

#[test]
fn bms_sample_rejects_non_finite_numeric_inputs() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            FloatOutBoyBmsSample::try_new(invalid, 4.1, -2, 43, 55, 0.2),
            None
        );
        assert_eq!(
            FloatOutBoyBmsSample::try_new(2.8, invalid, -2, 43, 55, 0.2),
            None
        );
        assert_eq!(
            FloatOutBoyBmsSample::try_new(2.8, 4.1, -2, 43, 55, invalid),
            None
        );
    }
}

#[test]
fn bms_sample_accepts_finite_numeric_extremes() {
    assert_eq!(
        FloatOutBoyBmsSample::try_new(f32::MIN, f32::MAX, i32::MIN, i32::MAX, i32::MIN, f32::MAX,),
        Some(FloatOutBoyBmsSample::new(
            Voltage::from_volts(f32::MIN),
            Voltage::from_volts(f32::MAX),
            FloatOutBoyBmsTemperature::from_degrees_celsius(i32::MIN),
            FloatOutBoyBmsTemperature::from_degrees_celsius(i32::MAX),
            FloatOutBoyBmsTemperature::from_degrees_celsius(i32::MIN),
            VescSeconds::from_seconds(f32::MAX),
        ))
    );
}

#[test]
fn ext_bms_invalid_type_leaves_the_sample_unchanged() {
    let mut state = enabled_state();
    let before = state.bms_sample_for_test();
    let mut values = encoded_sample();
    values[0] = LispValue::nil();

    let value = ExtBms::call(&mut state, LispArgs::from_values(&values));

    assert_eq!(value, LispValue::true_value());
    assert_eq!(state.bms_sample_for_test(), before);
}

#[test]
fn ext_bms_disabled_complete_call_does_not_replace_the_sample() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let before = state.bms_sample_for_test();
    let values = encoded_sample();

    let value = ExtBms::call(&mut state, LispArgs::from_values(&values));

    assert_eq!(value, LispValue::nil());
    assert_eq!(state.bms_sample_for_test(), before);
}

#[test]
fn ext_bms_disable_and_reenable_preserve_the_last_sample_like_refloat() {
    let mut state = enabled_state();
    let values = encoded_sample();
    assert_eq!(
        ExtBms::call(&mut state, LispArgs::from_values(&values)),
        LispValue::true_value()
    );
    let recorded = state.bms_sample_for_test();

    let mut config = FLOAT_OUT_BOY_DEFAULT_CONFIG;
    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &config
    ));
    assert_eq!(
        ExtBms::call(&mut state, LispArgs::empty()),
        LispValue::nil()
    );
    assert_eq!(state.bms_sample_for_test(), recorded);

    config[265] = 1;
    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &config
    ));
    assert_eq!(
        ExtBms::call(&mut state, LispArgs::empty()),
        LispValue::true_value()
    );
    assert_eq!(state.bms_sample_for_test(), recorded);
}

#[test]
fn runtime_bms_connection_fault_uses_float_out_boy_startup_timer_boundary() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut config = FLOAT_OUT_BOY_DEFAULT_CONFIG;
    config[265] = 1;
    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &config
    ));

    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(10_000));
    assert!(
        !state
            .bms_faults_for_test()
            .contains(FloatOutBoyBmsFaults::CONNECTION)
    );

    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(60_000));
    assert!(
        !state
            .bms_faults_for_test()
            .contains(FloatOutBoyBmsFaults::CONNECTION)
    );

    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(60_001));
    assert_eq!(
        state.bms_faults_for_test(),
        FloatOutBoyBmsFaults::CONNECTION
    );
}
