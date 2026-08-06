use super::FloatOutBoyPackageState;
use super::limits::traction_loss;
use crate::domain::{FloatOutBoyRealtimeFilteredMotorCurrent, FloatOutBoyRealtimeMotorCurrents};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{
    BatteryCellCount, BatteryCurrent, BatteryVoltage, Current, DirectionalMotorCurrent, DutyCycle,
    DutyCycleLimit, Frequency, InputCurrent, MotorCurrent, MotorCurrentLimit, SampleRate,
    SignedRatio, TemperatureLimitStart,
};

const CURRENT_FILTER_Q: f32 = 0.707;
const DEFAULT_CURRENT_FILTER_FREQUENCY: Frequency = Frequency::from_hertz(20.0);
const MOTOR_DATA_EMA_CUTOFF: Frequency = Frequency::from_hertz(1.0);

#[derive(Clone, Copy)]
pub(in crate::package) struct MotorConfigSnapshot {
    duty_max_with_margin: DutyCycleLimit,
    motor_current_max: MotorCurrentLimit,
    motor_current_min: MotorCurrentLimit,
    battery_current_max: InputCurrent,
    battery_current_min: InputCurrent,
    mosfet_temperature_limit_start: TemperatureLimitStart,
    motor_temperature_limit_start: TemperatureLimitStart,
    battery_cell_count: Option<BatteryCellCount>,
    motor_torque_constant: crate::motor_torque::MotorTorqueConstant,
}

pub(super) fn current_filter_frequency(configured: Frequency) -> Frequency {
    if configured.as_hertz() < 1.0 {
        DEFAULT_CURRENT_FILTER_FREQUENCY
    } else {
        configured
    }
}

pub(in crate::package) fn snapshot_motor_config(
    telemetry: &impl MotorTelemetry,
) -> MotorConfigSnapshot {
    let settings = vescpkg_rs::FirmwareSettings;
    MotorConfigSnapshot {
        duty_max_with_margin: telemetry
            .duty_cycle_limit()
            .reduced_by(traction_loss::DUTY_MARGIN),
        motor_current_max: telemetry.drive_current_limit(),
        motor_current_min: telemetry.brake_current_limit(),
        battery_current_max: settings.input_current_max(),
        battery_current_min: settings.input_current_min(),
        mosfet_temperature_limit_start: telemetry.mosfet_temperature_limit_start(),
        motor_temperature_limit_start: telemetry.motor_temperature_limit_start(),
        battery_cell_count: telemetry.battery_cell_count(),
        motor_torque_constant: crate::motor_torque::motor_torque_constant_from_firmware_config(
            settings.foc_motor_flux_linkage(),
            settings.motor_pole_count().ok(),
        ),
    }
}

pub(super) fn apply_motor_config(state: &mut FloatOutBoyPackageState, config: MotorConfigSnapshot) {
    state.duty_max_with_margin = config.duty_max_with_margin;
    state.motor_current_max = config.motor_current_max;
    state.motor_current_min = config.motor_current_min;
    state.battery_current_max = config.battery_current_max;
    state.battery_current_min = config.battery_current_min;
    state.mosfet_temperature_limit_start = config.mosfet_temperature_limit_start;
    state.motor_temperature_limit_start = config.motor_temperature_limit_start;
    state.battery_cell_count = config.battery_cell_count;
    state.motor_torque_constant = Some(config.motor_torque_constant);
}

pub(super) fn refresh_config(state: &mut FloatOutBoyPackageState, telemetry: &impl MotorTelemetry) {
    apply_motor_config(state, snapshot_motor_config(telemetry));
}

pub(super) fn refresh(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    elapsed: vescpkg_rs::prelude::VescSeconds,
) {
    let payloads = state.all_data_payloads;
    // C map: Float Out Boy v1.2.1 updates motor fields in `motor_data_update` at
    // `third_party/float-out-boy/src/motor_data.c:108-145`. Battery current uses the same first-order
    // smoothing expression from `third_party/float-out-boy/src/motor_data.c:140`; the package main
    // loop invokes this refresh before control aggregation like the source loop.
    let previous_battery_current = payloads.battery_current().current();
    let next_battery_current = telemetry.battery_current().current();
    let previous_duty_cycle = payloads.duty_cycle().ratio().as_ratio();
    let raw_duty_cycle = telemetry.duty_cycle().ratio().as_ratio().abs();
    let smoothing = vescpkg_rs::ema_alpha(
        MOTOR_DATA_EMA_CUTOFF,
        state.frequency_trackers.main.filter_frequency(),
    );
    state.motor_duty_raw = telemetry.duty_cycle().magnitude();
    state.motor_distance_meters = telemetry.signed_trip_distance().distance().as_meters();
    state.mosfet_temperature = telemetry.mosfet_temperature();
    state.motor_temperature = telemetry.motor_temperature();
    state.motor_current_filter.configure(
        current_filter_frequency(state.serialized_config.motor_current_filter_frequency()),
        state.frequency_trackers.main.filter_frequency(),
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
    state.motor_kinematics.record(motor_erpm, elapsed);
    state.all_data_payloads = payloads
        .with_motor_battery_voltage(BatteryVoltage::new(telemetry.input_voltage().voltage()))
        .with_electrical_speed(electrical_speed)
        .with_vehicle_speed(telemetry.vehicle_speed())
        .with_currents(FloatOutBoyRealtimeMotorCurrents::new(
            MotorCurrent::new(telemetry.motor_current().current()),
            directional_current,
            filtered_current,
            BatteryCurrent::new(
                previous_battery_current
                    + (next_battery_current - previous_battery_current) * smoothing,
            ),
        ))
        .with_duty_cycle(DutyCycle::new(SignedRatio::clamped(
            previous_duty_cycle + smoothing * (raw_duty_cycle - previous_duty_cycle),
        )))
        // Upstream compact all-data reads optional `VESC_IF->foc_get_id` at
        // `third_party/float-out-boy/src/main.c:1364-1368` and writes 222 when the slot is absent.
        .with_foc_id_current(
            telemetry
                .d_axis_current()
                .map(|current| MotorCurrent::new(current.current())),
        );
}

pub(super) fn reconfigure_filters(state: &mut FloatOutBoyPackageState, frequency: SampleRate) {
    state.motor_current_filter.configure(
        current_filter_frequency(state.serialized_config.motor_current_filter_frequency()),
        frequency,
        CURRENT_FILTER_Q,
    );
    state.motor_kinematics.configure(frequency);
}
