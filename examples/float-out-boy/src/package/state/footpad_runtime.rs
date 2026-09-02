use super::FloatOutBoyPackageState;
use crate::domain::{
    FloatOutBoyFootpadAdcMapping, FloatOutBoyFootpadSample, FloatOutBoyFootpadState,
};
use vescpkg_rs::prelude::AdcVoltage;

#[inline]
pub(super) fn refresh(state: &mut FloatOutBoyPackageState, adc1: AdcVoltage, adc2: AdcVoltage) {
    // C map: state derives footpad sensor state from raw ADC volts at
    // `third_party/float-out-boy/src/footpad_sensor.c:28-61`.
    let adc1 = adc1.voltage();
    let adc2 = adc2.voltage();
    let faults = state.serialized_config.faults();
    let mapping = state.serialized_config.footpad_adc_mapping();
    let (left_voltage, right_voltage) = mapping.logical_voltages(adc1, adc2);
    let sample = FloatOutBoyFootpadSample::new(
        left_voltage,
        right_voltage,
        sensor_state(
            adc1.as_volts(),
            adc2.as_volts(),
            faults.adc1_voltage().as_volts(),
            faults.adc2_voltage().as_volts(),
            mapping,
        ),
    );

    let payloads = state.all_data_payloads;
    state.all_data_payloads = payloads.with_base(payloads.base().with_footpad(sample));
}

#[inline]
fn sensor_state(
    adc1_volts: f32,
    adc2_volts: f32,
    fault_adc1: f32,
    fault_adc2: f32,
    mapping: FloatOutBoyFootpadAdcMapping,
) -> FloatOutBoyFootpadState {
    let adc1_on = fault_adc1 == 0.0 || adc1_volts > fault_adc1;
    let adc2_on = fault_adc2 == 0.0 || adc2_volts > fault_adc2;

    if fault_adc1 == 0.0 || fault_adc2 == 0.0 {
        return if adc1_on && adc2_on {
            FloatOutBoyFootpadState::Both
        } else {
            FloatOutBoyFootpadState::None
        };
    }

    match (mapping, adc1_on, adc2_on) {
        (_, true, true) => FloatOutBoyFootpadState::Both,
        (FloatOutBoyFootpadAdcMapping::Direct, true, false)
        | (FloatOutBoyFootpadAdcMapping::Swapped, false, true) => FloatOutBoyFootpadState::Left,
        (FloatOutBoyFootpadAdcMapping::Direct, false, true)
        | (FloatOutBoyFootpadAdcMapping::Swapped, true, false) => FloatOutBoyFootpadState::Right,
        (_, false, false) => FloatOutBoyFootpadState::None,
    }
}
