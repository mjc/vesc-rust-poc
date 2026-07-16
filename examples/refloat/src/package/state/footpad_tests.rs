use super::RefloatPackageState;
use crate::domain::{RefloatAllDataPayloads, RefloatFootpadState};
use vescpkg_rs::prelude::{AdcVoltage, Voltage};

#[test]
fn footpad_runtime_refresh_decodes_adc_like_refloat_sensor_update() {
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    state.refresh_footpad_runtime_state(
        AdcVoltage::new(Voltage::from_volts(2.5)),
        AdcVoltage::new(Voltage::ZERO),
    );

    let footpad = state.all_data_payloads().base().footpad();
    // C map: Refloat v1.2.1 `footpad_sensor_update` reads ADCs and decodes
    // the switch state at
    // `third_party/refloat/src/footpad_sensor.c:28-61`.
    assert_eq!(footpad.state(), RefloatFootpadState::Left);
    assert_eq!(footpad.adc1_volts(), 2.5);
    assert_eq!(footpad.adc2_volts(), 0.0);
}
