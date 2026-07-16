use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataPayloads, RefloatFootpadSample, RefloatFootpadState,
};
use vescpkg_rs::prelude::{AdcVoltage, Voltage};

#[inline(always)]
pub(super) fn refresh(state: &mut RefloatPackageState, adc1: AdcVoltage, adc2: AdcVoltage) {
    // C map: state derives footpad sensor state from raw ADC volts at
    // `third_party/refloat/src/footpad_sensor.c:28-61`.
    let adc1 = adc1.voltage();
    let adc2 = adc2_zero_floor(adc2.voltage());
    let faults = state.serialized_config.faults();
    let sample = RefloatFootpadSample::new(
        adc1,
        adc2,
        sensor_state(
            adc1.as_volts(),
            adc2.as_volts(),
            faults.adc1_voltage().as_volts(),
            faults.adc2_voltage().as_volts(),
        ),
    );

    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        sample,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    state.all_data_payloads =
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}

#[inline(always)]
fn adc2_zero_floor(adc2: Voltage) -> Voltage {
    // C map: `footpad_sensor_update` clamps a missing ADC2 read to zero at
    // `third_party/refloat/src/footpad_sensor.c:28-61`.
    if adc2.as_volts() < 0.0 {
        Voltage::from_volts(0.0)
    } else {
        adc2
    }
}

#[inline(always)]
fn sensor_state(
    adc1_volts: f32,
    adc2_volts: f32,
    fault_adc1: f32,
    fault_adc2: f32,
) -> RefloatFootpadState {
    // C map: Refloat v1.2.1 `footpad_sensor_update` decodes the switch
    // state from raw ADC volts at `third_party/refloat/src/footpad_sensor.c:28-61`.
    let mut state = RefloatFootpadState::None;
    if fault_adc1 == 0.0 && fault_adc2 == 0.0 {
        state = RefloatFootpadState::Both;
    } else if fault_adc2 == 0.0 {
        if adc1_volts > fault_adc1 {
            state = RefloatFootpadState::Both;
        }
    } else if fault_adc1 == 0.0 {
        if adc2_volts > fault_adc2 {
            state = RefloatFootpadState::Both;
        }
    } else if adc1_volts > fault_adc1 {
        state = if adc2_volts > fault_adc2 {
            RefloatFootpadState::Both
        } else {
            RefloatFootpadState::Left
        };
    } else if adc2_volts > fault_adc2 {
        state = RefloatFootpadState::Right;
    }
    state
}
