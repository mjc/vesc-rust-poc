use super::FloatOutBoyPackageState;
use super::limits::traction_loss;
use crate::domain::{
    FloatOutBoyAllDataMotorPayload, FloatOutBoyRealtimeFilteredMotorCurrent,
    FloatOutBoyRealtimeMotorCurrents,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{
    BatteryCurrent, BatteryVoltage, Current, DirectionalMotorCurrent, DutyCycle, Frequency,
    MotorCurrent, SampleRate, SignedRatio,
};

const CURRENT_FILTER_Q: f32 = 0.707;
const MOTOR_DATA_SMOOTHING_FACTOR: f32 = 0.01;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct FloatOutBoyMotorCurrentFilter {
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    z1: f32,
    z2: f32,
    enabled: bool,
}

impl FloatOutBoyMotorCurrentFilter {
    fn configure(&mut self, frequency: Frequency, sample_rate: SampleRate) {
        self.enabled = frequency.is_positive();
        if self.enabled {
            let k = vescpkg_rs::tan(
                core::f32::consts::PI * frequency.as_hertz() / sample_rate.as_hertz(),
            );
            let norm = 1.0 / (1.0 + k / CURRENT_FILTER_Q + k * k);
            self.a0 = k * k * norm;
            self.a1 = 2.0 * self.a0;
            self.a2 = self.a0;
            self.b1 = 2.0 * (k * k - 1.0) * norm;
            self.b2 = (1.0 - k / CURRENT_FILTER_Q + k * k) * norm;
        }
    }

    fn process(
        &mut self,
        current: DirectionalMotorCurrent,
    ) -> FloatOutBoyRealtimeFilteredMotorCurrent {
        let input = current.current().as_amps();
        let output = if self.enabled {
            let output = input * self.a0 + self.z1;
            self.z1 = input * self.a1 + self.z2 - self.b1 * output;
            self.z2 = input * self.a2 - self.b2 * output;
            output
        } else {
            input
        };
        FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
            Current::from_amps(output),
        ))
    }

    pub(super) fn reset_runtime(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

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
        state.frequency_trackers.main.filter_frequency(),
    );
    let directional_current = telemetry.directional_motor_current();
    let filtered_current = state.motor_current_filter.process(directional_current);
    let electrical_speed = telemetry.electrical_speed();
    let motor_erpm = electrical_speed.rpm();
    // Upstream averages acceleration over `ACCEL_ARRAY_SIZE == 40` samples
    // in `third_party/float-out-boy/src/motor_data.c:128-133`.
    state.motor_kinematics.record(motor_erpm);
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
    state.all_data_payloads = payloads.with_base(base.with_motor(motor));
}

pub(super) fn reconfigure_current_filter(
    state: &mut FloatOutBoyPackageState,
    frequency: SampleRate,
) {
    state.motor_current_filter.configure(
        state.serialized_config.motor_current_filter_frequency(),
        frequency,
    );
}

#[cfg(test)]
mod tests;
