use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataMotorPayload, RefloatAllDataPayloads,
    RefloatFocIdCurrent,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, MotorCurrent};

pub(super) fn refresh(state: &mut RefloatPackageState, telemetry: &impl MotorTelemetry) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let motor = base.motor();
    // C map: Refloat v1.2.1 updates motor fields in `motor_data_update` at
    // `third_party/refloat/src/motor_data.c:108-145`. Battery current uses the same first-order
    // smoothing expression from `third_party/refloat/src/motor_data.c:140`; this app-data
    // refresh is still a runtime proxy until the real source main loop runs.
    let previous_battery_current = motor.battery_current().current().as_amps();
    let next_battery_current = telemetry.battery_current().current().as_amps();
    state.motor_current_max = MotorCurrent::new(telemetry.motor_current_max().current());
    state.motor_current_min = MotorCurrent::new(telemetry.motor_current_min().current());
    let electrical_speed = telemetry.electrical_speed();
    let motor_erpm = electrical_speed.rpm();
    // Upstream averages acceleration over `ACCEL_ARRAY_SIZE == 40` samples
    // in `third_party/refloat/src/motor_data.c:128-133`.
    state.motor_acceleration.record(motor_erpm);
    let motor = RefloatAllDataMotorPayload::new(
        BatteryVoltage::new(telemetry.input_voltage_filtered().voltage()),
        electrical_speed,
        telemetry.vehicle_speed(),
        telemetry.motor_current(),
        BatteryCurrent::new(Current::from_amps(
            previous_battery_current + 0.01 * (next_battery_current - previous_battery_current),
        )),
        telemetry.duty_cycle_now(),
        // Upstream compact all-data reads optional `VESC_IF->foc_get_id` at
        // `third_party/refloat/src/main.c:1364-1368` and writes 222 when the slot is absent.
        telemetry.foc_id_current().map_or(
            RefloatFocIdCurrent::unavailable(),
            RefloatFocIdCurrent::measured,
        ),
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        motor,
    );
    state.all_data_payloads =
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}
