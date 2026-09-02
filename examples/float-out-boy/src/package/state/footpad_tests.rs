use super::FloatOutBoyPackageState;
use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyFootpadState};
use vescpkg_rs::prelude::{AdcVoltage, Voltage};

#[test]
fn footpad_runtime_refresh_decodes_adc_like_float_out_boy_sensor_update() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(2.5)),
        AdcVoltage::new(Voltage::ZERO),
    );

    let footpad = state.all_data_payloads().base().footpad();
    // C map: Float Out Boy v1.2.1 `footpad_sensor_update` reads ADCs and decodes
    // the switch state at
    // `third_party/float-out-boy/src/footpad_sensor.c:28-61`.
    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Left);
    assert_f32_eq!(footpad.adc1_volts(), 2.5);
    assert_f32_eq!(footpad.adc2_volts(), 0.0);
}
