use super::FloatOutBoyPackageState;
use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyFootpadState};
use vescpkg_rs::prelude::{AdcVoltage, Voltage};

fn sample(
    state: &mut FloatOutBoyPackageState,
    adc1: f32,
    adc2: f32,
) -> crate::domain::FloatOutBoyFootpadSample {
    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(adc1)),
        AdcVoltage::new(Voltage::from_volts(adc2)),
    );
    state.all_data_payloads().footpad()
}

fn set_fault_thresholds(
    state: &mut FloatOutBoyPackageState,
    adc1_millivolts: u16,
    adc2_millivolts: u16,
) {
    let mut config = *state.serialized_config.as_bytes();
    config[44..46].copy_from_slice(&adc1_millivolts.to_be_bytes());
    config[46..48].copy_from_slice(&adc2_millivolts.to_be_bytes());
    let config = crate::config::FloatOutBoyConfigImage::from_serialized(&config).unwrap();
    state.replace_serialized_config_for_test(&config);
}

fn set_adc_swap(state: &mut FloatOutBoyPackageState, swapped: bool) {
    let mut config = *state.serialized_config.as_bytes();
    config[247] = u8::from(swapped);
    let config = crate::config::FloatOutBoyConfigImage::from_serialized(&config).unwrap();
    state.replace_serialized_config_for_test(&config);
}

#[test]
fn footpad_runtime_reports_logical_left_and_right_voltages() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    let footpad = sample(&mut state, 2.5, 0.0);
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Left);
    assert_f32_eq!(footpad.left_voltage().as_volts(), 2.5);
    assert_f32_eq!(footpad.right_voltage().as_volts(), 0.0);
}

#[test]
fn footpad_adc_swap_changes_logical_measurements_and_pressed_side_together() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    set_adc_swap(&mut state, true);

    let footpad = sample(&mut state, 2.5, 0.25);
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Right);
    assert_f32_eq!(footpad.left_voltage().as_volts(), 0.25);
    assert_f32_eq!(footpad.right_voltage().as_volts(), 2.5);
}

#[test]
fn footpad_threshold_comparison_is_strict_and_both_sides_compose() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    set_fault_thresholds(&mut state, 1_000, 2_000);

    assert_eq!(
        sample(&mut state, 1.0, 2.0).state(),
        FloatOutBoyFootpadState::None
    );
    assert_eq!(
        sample(&mut state, 1.01, 2.0).state(),
        FloatOutBoyFootpadState::Left
    );
    assert_eq!(
        sample(&mut state, 1.0, 2.01).state(),
        FloatOutBoyFootpadState::Right
    );
    assert_eq!(
        sample(&mut state, 1.01, 2.01).state(),
        FloatOutBoyFootpadState::Both
    );
}

#[test]
fn zero_threshold_single_sensor_reports_both_only_when_live_sensor_is_pressed() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    set_fault_thresholds(&mut state, 0, 2_000);
    assert_eq!(
        sample(&mut state, 0.0, 2.0).state(),
        FloatOutBoyFootpadState::None
    );
    assert_eq!(
        sample(&mut state, 0.0, 2.01).state(),
        FloatOutBoyFootpadState::Both
    );

    set_fault_thresholds(&mut state, 1_000, 0);
    assert_eq!(
        sample(&mut state, 1.0, 0.0).state(),
        FloatOutBoyFootpadState::None
    );
    assert_eq!(
        sample(&mut state, 1.01, 0.0).state(),
        FloatOutBoyFootpadState::Both
    );
}

#[test]
fn single_sensor_state_is_not_relabelled_when_adc_mapping_is_swapped() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    set_fault_thresholds(&mut state, 0, 2_000);
    set_adc_swap(&mut state, true);

    let footpad = sample(&mut state, 0.25, 2.5);
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Both);
    assert_f32_eq!(footpad.left_voltage().as_volts(), 2.5);
    assert_f32_eq!(footpad.right_voltage().as_volts(), 0.25);
}

#[test]
fn zero_thresholds_on_both_inputs_model_no_sensors_as_always_engaged() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    set_fault_thresholds(&mut state, 0, 0);

    assert_eq!(
        sample(&mut state, 0.0, 0.0).state(),
        FloatOutBoyFootpadState::Both
    );
}
