use super::FloatOutBoyPackageState;
use crate::domain::FloatOutBoyFootpadState;
use vescpkg_rs::prelude::{AdcVoltage, Voltage};

#[test]
fn footpad_runtime_refresh_decodes_adc_like_float_out_boy_sensor_update() {
    let mut state = FloatOutBoyPackageState::default();

    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(2.5)),
        AdcVoltage::new(Voltage::ZERO),
    );

    let footpad = state.all_data_payloads().base().footpad();
    // C map: Float Out Boy v1.2.1 `footpad_sensor_update` reads ADCs and decodes
    // the switch state at
    // `third_party/float-out-boy/src/footpad_sensor.c:28-61`.
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Left);
    assert_f32_eq!(footpad.left_voltage().as_volts(), 2.5);
    assert_f32_eq!(footpad.right_voltage().as_volts(), 0.0);
}

#[test]
fn footpad_adc_swap_maps_physical_inputs_to_logical_sides() {
    let mut state = FloatOutBoyPackageState::default();
    let mut config = *state.serialized_config.as_bytes();
    config[247] = 1;
    state.replace_serialized_config_for_test(
        &crate::config::FloatOutBoyConfigImage::from_serialized(&config).expect("valid image"),
    );

    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(2.5)),
        AdcVoltage::new(Voltage::from_volts(0.25)),
    );

    let footpad = state.all_data_payloads().base().footpad();
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Right);
    assert_f32_eq!(footpad.left_voltage().as_volts(), 0.25);
    assert_f32_eq!(footpad.right_voltage().as_volts(), 2.5);
}

#[test]
fn footpad_single_sensor_threshold_keeps_both_state_when_pressed() {
    let mut state = FloatOutBoyPackageState::default();
    let mut config = *state.serialized_config.as_bytes();
    config[44..46].copy_from_slice(&0_u16.to_be_bytes());
    config[46..48].copy_from_slice(&2_000_u16.to_be_bytes());
    state.replace_serialized_config_for_test(
        &crate::config::FloatOutBoyConfigImage::from_serialized(&config).expect("valid image"),
    );

    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(0.0)),
        AdcVoltage::new(Voltage::from_volts(2.01)),
    );

    assert_eq!(
        state.all_data_payloads().base().footpad().state(),
        FloatOutBoyFootpadState::Both
    );
}
