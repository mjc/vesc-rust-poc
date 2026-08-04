use super::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use super::limits::traction_loss;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMotorPayload,
    FloatOutBoyRealtimeFilteredMotorCurrent, FloatOutBoyRealtimeMotorCurrents,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{
    BatteryCurrent, BatteryVoltage, Current, DirectionalMotorCurrent, DutyCycle, MotorCurrent,
    SignedRatio,
};

const CURRENT_FILTER_Q: f32 = 0.707;
const MOTOR_DATA_SMOOTHING_FACTOR: f32 = 0.01;

#[cfg(any(test, target_arch = "arm"))]
pub(super) fn refresh_config(state: &mut FloatOutBoyPackageState, telemetry: &impl MotorTelemetry) {
    state.duty_max_with_margin = telemetry
        .duty_cycle_limit()
        .reduced_by(traction_loss::DUTY_MARGIN);
    state.motor_current_max = telemetry.drive_current_limit();
    state.motor_current_min = telemetry.brake_current_limit();
    let settings = vescpkg_rs::FirmwareSettings;
    state.battery_current_max = settings.input_current_max();
    state.battery_current_min = settings.input_current_min();
    state.mosfet_temperature_limit_start = telemetry.mosfet_temperature_limit_start();
    state.motor_temperature_limit_start = telemetry.motor_temperature_limit_start();
    state.battery_cell_count = telemetry.battery_cell_count();
}

pub(super) fn refresh(state: &mut FloatOutBoyPackageState, telemetry: &impl MotorTelemetry) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let motor = base.motor();
    // C map: Float Out Boy v1.2.1 updates motor fields in `motor_data_update` at
    // `third_party/float-out-boy/src/motor_data.c:108-145`. Battery current uses the same first-order
    // smoothing expression from `third_party/float-out-boy/src/motor_data.c:140`; the package main
    // loop invokes this refresh before control aggregation like the source loop.
    let previous_battery_current = motor.battery_current().current();
    let next_battery_current = telemetry.battery_current().current();
    let previous_duty_cycle = motor.duty_cycle().ratio().as_ratio();
    let raw_duty_cycle = telemetry.duty_cycle().ratio().as_ratio().abs();
    state.motor_duty_raw = telemetry.duty_cycle().magnitude();
    state.mosfet_temperature = telemetry.mosfet_temperature();
    state.motor_temperature = telemetry.motor_temperature();
    state.motor_current_filter.configure(
        state.serialized_config.motor_current_filter_frequency(),
        state.serialized_config.startup().sample_rate(),
        CURRENT_FILTER_Q,
    );
    let directional_current = telemetry.directional_motor_current();
    let filtered_current = FloatOutBoyRealtimeFilteredMotorCurrent::new(
        DirectionalMotorCurrent::new(Current::from_amps(
            state
                .motor_current_filter
                .process(directional_current.current().as_amps()),
        )),
    );
    let electrical_speed = telemetry.electrical_speed();
    let motor_erpm = electrical_speed.rpm();
    // Upstream averages acceleration over `ACCEL_ARRAY_SIZE == 40` samples
    // in `third_party/float-out-boy/src/motor_data.c:128-133`.
    state
        .motor_kinematics
        .record(motor_erpm, super::motor_kinematics::ABS_ERPM_SMOOTHING);
    let motor = FloatOutBoyAllDataMotorPayload::new(
        BatteryVoltage::new(telemetry.input_voltage().voltage()),
        electrical_speed,
        telemetry.vehicle_speed(),
        FloatOutBoyRealtimeMotorCurrents::new(
            MotorCurrent::new(telemetry.motor_current().current()),
            directional_current,
            filtered_current,
            BatteryCurrent::new(
                previous_battery_current
                    + (next_battery_current - previous_battery_current)
                        * MOTOR_DATA_SMOOTHING_FACTOR,
            ),
        ),
        DutyCycle::new(SignedRatio::clamped(
            previous_duty_cycle
                + MOTOR_DATA_SMOOTHING_FACTOR * (raw_duty_cycle - previous_duty_cycle),
        )),
        // Upstream compact all-data reads optional `VESC_IF->foc_get_id` at
        // `third_party/float-out-boy/src/main.c:1364-1368` and writes 222 when the slot is absent.
        telemetry
            .d_axis_current()
            .map(|current| MotorCurrent::new(current.current())),
    );
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        motor,
    );
    state.all_data_payloads = payloads.with_base(base);
}
