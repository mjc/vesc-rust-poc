use super::lifecycle::RefloatPackageLifecycle;
use super::protocol::encode_refloat_realtime_data_response;
use super::threads::RefloatRuntimeThreads;
use super::{refloat_ticks_elapsed, refloat_ticks_elapsed_f32, refloat_ticks_elapsed_ms};
use crate::balance::{
    RefloatBalanceFilter, RefloatBalanceLoopConfig, RefloatBalanceLoopInput,
    RefloatBalanceLoopState, refloat_balance_loop_step,
};
use crate::config::*;
use crate::domain::{
    FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude,
    RefloatAllDataBasePayload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
    RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataRequest,
    RefloatAllDataResponse, RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatDarkRideState, RefloatFirmwareFaultCode, RefloatFocIdCurrent, RefloatMode,
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
    RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};
use crate::motor_control::RefloatMotorControl;
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, BatteryVoltage, Current, MotorCurrent, SampleRate,
    SystemTimestamp, TimestampTicks, Voltage,
};
use vescpkg_rs::{
    AppDataBindings, ImuApi, ImuBindings, MotorControlApi, MotorControlBindings, MotorTelemetryApi,
    MotorTelemetryBindings, ffi,
};

mod handtest;
mod transition;

use transition::{
    RefloatStateTransitionInput, RefloatStopEvent, refloat_first_stop_event,
    refloat_state_transition,
};

#[inline]
fn refloat_command_payload(bytes: &[u8], command: RefloatAppDataCommand) -> Option<&[u8]> {
    match bytes {
        [package_id, command_id, payload @ ..]
            if *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get() && *command_id == command.id() =>
        {
            Some(payload)
        }
        _ => None,
    }
}

/// Refloat package state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatPackageState {
    all_data_payloads: RefloatAllDataPayloads,
    serialized_config: [u8; 276],
    handtest_config_backup: Option<[u8; 276]>,
    runtime_threads: RefloatRuntimeThreads,
    motor_control: RefloatMotorControl,
    pub(super) balance_filter: RefloatBalanceFilter,
    traction_control: bool,
    pid_integral_current: f32,
    pid_kp_brake_scale: f32,
    pid_kp2_brake_scale: f32,
    pid_kp_accel_scale: f32,
    pid_kp2_accel_scale: f32,
    softstart_pid_limit: f32,
    reverse_total_erpm: f32,
    motor_last_erpm: f32,
    motor_acceleration: f32,
    motor_accel_history: [f32; 40],
    motor_accel_idx: usize,
    remote_input: f32,
    rc_current: f32,
    rc_steps: u16,
    rc_counter: u16,
    rc_current_target_deciamps: i16,
    engage_ticks: u32,
    disengage_ticks: u32,
    fault_switch_ticks: u32,
    fault_switch_half_ticks: u32,
    reverse_ticks: u32,
    fault_angle_pitch_ticks: u32,
    fault_angle_roll_ticks: u32,
    motor_current_max: MotorCurrent,
    motor_current_min: MotorCurrent,
}

impl RefloatPackageState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        Self {
            all_data_payloads,
            // Upstream `data_init` reads EEPROM and falls back to generated
            // defaults at `third_party/refloat/src/main.c:1160-1185`; full EEPROM parity remains a
            // later source-backed slice.
            serialized_config: REFLOAT_DEFAULT_CONFIG,
            handtest_config_backup: None,
            // Upstream stores these in `Data` after spawning at
            // `third_party/refloat/src/main.c:2439-2445`; this Rust state only tracks the handles
            // until the full `Data` layout is ported.
            runtime_threads: RefloatRuntimeThreads::empty(),
            motor_control: RefloatMotorControl::new(),
            balance_filter: RefloatBalanceFilter::source_startup(),
            traction_control: false,
            pid_integral_current: 0.0,
            pid_kp_brake_scale: 1.0,
            pid_kp2_brake_scale: 1.0,
            pid_kp_accel_scale: 1.0,
            pid_kp2_accel_scale: 1.0,
            softstart_pid_limit: 100.0,
            reverse_total_erpm: 0.0,
            motor_last_erpm: 0.0,
            motor_acceleration: 0.0,
            motor_accel_history: [0.0; 40],
            motor_accel_idx: 0,
            remote_input: 0.0,
            rc_current: 0.0,
            rc_steps: 0,
            rc_counter: 0,
            rc_current_target_deciamps: 0,
            engage_ticks: 0,
            disengage_ticks: 0,
            fault_switch_ticks: 0,
            fault_switch_half_ticks: 0,
            reverse_ticks: 0,
            fault_angle_pitch_ticks: 0,
            fault_angle_roll_ticks: 0,
            motor_current_max: MotorCurrent::new(Current::from_amps(100.0)),
            motor_current_min: MotorCurrent::new(Current::from_amps(100.0)),
        }
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    /// Return the runtime thread handles currently owned by this package state.
    pub const fn runtime_threads(self) -> RefloatRuntimeThreads {
        self.runtime_threads
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    #[cfg(test)]
    fn set_remote_input_for_test(&mut self, remote_input: f32) {
        self.remote_input = remote_input;
    }

    /// Apply and clear a pending motor-current request.
    pub fn apply_requested_motor_current<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
    ) -> bool {
        self.motor_control.apply_requested_current(motor)
    }

    /// Apply motor control for the current run state.
    pub fn apply_motor_control<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
        run_state: RefloatRunState,
        system_time_ticks: u32,
    ) -> bool {
        let base = self.all_data_payloads.base();
        // Upstream `motor_control_configure` copies brake and parking config at
        // `third_party/refloat/src/motor_control.c:36-40`; this Rust state keeps
        // the serialized config as source of truth until full `Data` parity.
        self.motor_control.apply(
            motor,
            run_state,
            base.motor()
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute()
                .abs(),
            system_time_ticks,
            self.config_byte(REFLOAT_CONFIG_PARKING_BRAKE_MODE_OFFSET),
            MotorCurrent::new(Current::from_amps(
                self.config_scaled_i16(REFLOAT_CONFIG_BRAKE_CURRENT_OFFSET, 100.0),
            )),
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn set_runtime_threads(&mut self, runtime_threads: RefloatRuntimeThreads) {
        self.runtime_threads = runtime_threads;
    }

    pub(super) fn serialized_config(&self) -> &[u8; 276] {
        &self.serialized_config
    }

    fn config_byte(&self, offset: usize) -> u8 {
        let Some(byte) = self.serialized_config.get(offset) else {
            return 0;
        };
        *byte
    }

    fn config_be_u16(&self, offset: usize) -> u16 {
        u16::from_be_bytes([self.config_byte(offset), self.config_byte(offset + 1)])
    }

    fn config_scaled_i16(&self, offset: usize, scale: f32) -> f32 {
        refloat_read_scaled_i16(self.config_be_u16(offset).to_be_bytes(), scale)
    }

    fn config_scaled_field(&self, field: RefloatScaledConfigField) -> f32 {
        self.config_scaled_i16(field.offset.get(), field.scale.get())
    }

    fn set_config_byte(config: &mut [u8; 276], offset: usize, value: u8) -> bool {
        let Some(byte) = config.get_mut(offset) else {
            return false;
        };
        *byte = value;
        true
    }

    // HANDTEST writes mirror `third_party/refloat/src/main.c:1431-1446`; keep the serialized
    // u16 store checked so corrupt offsets cannot panic on target.
    fn set_config_be_u16(config: &mut [u8; 276], offset: usize, value: u16) -> bool {
        let Some(bytes) = offset
            .checked_add(2)
            .and_then(|end| config.get_mut(offset..end))
            .and_then(|bytes| <&mut [u8; 2]>::try_from(bytes).ok())
        else {
            return false;
        };
        *bytes = value.to_be_bytes();
        true
    }

    pub(super) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Ok(config) = <&[u8; 276]>::try_from(config) else {
            return false;
        };
        if !config.starts_with(&REFLOAT_CONFIG_SIGNATURE_BYTES) {
            return false;
        }

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `third_party/refloat/src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        let mut config = *config;
        // Upstream clears `d->float_conf.disabled` while running at
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is
        // serialized from `third_party/refloat/src/conf/settings.xml:3890-3902`
        // at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            Self::set_config_byte(&mut config, REFLOAT_CONFIG_DISABLED_OFFSET, 0);
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `third_party/refloat/src/main.c:2375-2377`; `meta.is_default`
        // is serialized from `third_party/refloat/src/conf/settings.xml:3903-3914`
        // at byte 275.
        Self::set_config_byte(&mut config, REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET, 0);
        self.serialized_config = config;
        // After a successful write, C calls `configure(d)` at
        // `third_party/refloat/src/main.c:2380-2382`, which refreshes the balance filter KP at
        // `third_party/refloat/src/main.c:158-160`.
        self.refresh_balance_filter_config();
        true
    }

    fn refresh_balance_filter_config(&mut self) {
        let mahony_kp = self.config_scaled_i16(REFLOAT_CONFIG_MAHONY_KP_OFFSET, 10000.0);
        let mahony_kp_roll = self.config_scaled_i16(REFLOAT_CONFIG_MAHONY_KP_ROLL_OFFSET, 10000.0);
        self.balance_filter.configure(mahony_kp, mahony_kp_roll);
    }

    fn refresh_config_runtime_state(&mut self) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let disabled = self.config_byte(REFLOAT_CONFIG_DISABLED_OFFSET) != 0;
        let run_state = match (ride_state.run_state(), disabled) {
            // Refloat applies `float_conf.disabled` from `configure(d)` at
            // `third_party/refloat/src/main.c:184-190`; `state_set_disabled`
            // keeps RUNNING alive and toggles DISABLED/STARTUP at
            // `third_party/refloat/src/state.c:41-47`.
            (RefloatRunState::Running, true) => RefloatRunState::Running,
            (RefloatRunState::Disabled, false) => RefloatRunState::Startup,
            (_, true) => RefloatRunState::Disabled,
            (run_state, false) => run_state,
        };
        if run_state == ride_state.run_state() {
            return;
        }

        let ride_state = RefloatRideState::new(
            run_state,
            ride_state.mode(),
            ride_state.setpoint_adjustment(),
            ride_state.stop_condition(),
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(ride_state.wheelslip())
        .with_darkride(ride_state.darkride());
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        let hertz = self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET);
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/refloat/src/main.c:190-191`, then `refloat_thd`
        // sleeps that value at `third_party/refloat/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        1_000_000 / u32::from(hertz.max(1))
    }

    /// Recover typed app-data state from VESC loader metadata.
    ///
    pub fn from_info_arg(info: &mut ffi::LibInfo) -> Option<&mut Self> {
        vescpkg_rs::loader_state_mut(info)
    }

    /// Handle one app-data packet through the supplied lifecycle transport.
    pub fn handle_packet<B: AppDataBindings>(
        &mut self,
        lifecycle: &RefloatPackageLifecycle<B>,
        bytes: &[u8],
    ) -> bool {
        self.handle_charging_state_packet(bytes)
            || lifecycle.send_response(&self.all_data_payloads, bytes)
    }

    /// Handle one app-data packet in the firmware callback context.
    ///
    /// Upstream `on_command_received` dispatches commands at
    /// `third_party/refloat/src/main.c:2143-2225`; the main
    /// `refloat_thd` owns `time_update`, `imu_update`, `motor_data_update`, and
    /// control-loop transitions at `third_party/refloat/src/main.c:772-1080`.
    pub fn handle_packet_with_runtime<
        B: AppDataBindings,
        M: MotorTelemetryBindings,
        I: ImuBindings,
    >(
        &mut self,
        lifecycle: &RefloatPackageLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        _imu: &ImuApi<I>,
        bytes: &[u8],
    ) -> bool {
        #[cfg(all(not(test), not(target_arch = "arm")))]
        self.refresh_runtime_state(telemetry, _imu, lifecycle.bindings().system_time_ticks());

        self.handle_packet_with_telemetry(lifecycle, telemetry, bytes)
    }

    /// Refresh the source-backed runtime slices that Refloat updates near the
    /// top of `refloat_thd`.
    ///
    /// C map: Refloat v1.2.1 `imu_ref_callback` starts at `third_party/refloat/src/main.c:760`.
    ///
    /// Upstream applies `configure(d)` before runtime work at
    /// `third_party/refloat/src/main.c:184-191`, updates IMU at `third_party/refloat/src/main.c:775`, motor data at
    /// `third_party/refloat/src/main.c:796`, and performs the `STATE_STARTUP` -> `STATE_READY`
    /// gate at `third_party/refloat/src/main.c:833-838`.
    pub(crate) fn refresh_runtime_state<M: MotorTelemetryBindings, I: ImuBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        system_time_ticks: u32,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    /// Refresh the runtime slices in the target main-loop order.
    ///
    /// C map: Refloat v1.2.1 `refloat_thd` updates motor data at
    /// `third_party/refloat/src/main.c:796`, footpad ADC state at
    /// `third_party/refloat/src/main.c:802`, then uses that state in the
    /// later control/fault path.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    pub(crate) fn refresh_main_loop_runtime_state<M: MotorTelemetryBindings, I: ImuBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        footpad_adc1: f32,
        footpad_adc2: f32,
        system_time_ticks: u32,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_footpad_runtime_state(footpad_adc1, footpad_adc2);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    fn handle_rc_move_packet(&mut self, bytes: &[u8]) -> bool {
        match refloat_command_payload(bytes, RefloatAppDataCommand::RcMove) {
            Some([direction, current, time, sum, ..]) => {
                if self
                    .all_data_payloads
                    .base()
                    .status()
                    .ride_state()
                    .run_state()
                    == RefloatRunState::Ready
                {
                    self.rc_counter = 0;
                    self.rc_current_target_deciamps =
                        match (*sum == time.wrapping_add(*current), *direction) {
                            (false, _) => 0,
                            (true, 0) => -i16::from(*current),
                            (true, _) => i16::from(*current),
                        };
                    match self.rc_current_target_deciamps {
                        0 => {
                            self.rc_steps = 1;
                            self.rc_current = 0.0;
                        }
                        target if target > 80 => {
                            self.rc_steps = u16::from(*time) * 100;
                            self.rc_current_target_deciamps = 20;
                        }
                        _ => {
                            self.rc_steps = u16::from(*time) * 100;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn send_metadata_packet_response<B: AppDataBindings>(
        &self,
        lifecycle: &RefloatPackageLifecycle<B>,
        bytes: &[u8],
    ) -> bool {
        refloat_command_payload(bytes, RefloatAppDataCommand::Info)
            .or_else(|| refloat_command_payload(bytes, RefloatAppDataCommand::RealtimeDataIds))
            .is_some()
            && lifecycle.send_response(&self.all_data_payloads, bytes)
    }

    fn send_realtime_data_packet_response<B: AppDataBindings, M: MotorTelemetryBindings>(
        &self,
        lifecycle: &RefloatPackageLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        match refloat_command_payload(bytes, RefloatAppDataCommand::RealtimeData) {
            Some(_) => {
                let payloads = self
                    .all_data_payloads
                    .with_base_battery_voltage(BatteryVoltage::new(
                        telemetry.input_voltage_filtered().voltage(),
                    ))
                    .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                        telemetry.mosfet_temperature(),
                        telemetry.motor_temperature(),
                    ));
                // Refloat's main loop updates `d->time.now` before app-data reads it
                // in `cmd_realtime_data` at `third_party/refloat/src/main.c:1931`.
                let system_timestamp = SystemTimestamp::new(TimestampTicks::from_ticks(
                    lifecycle.bindings().system_time_ticks(),
                ));
                let response = encode_refloat_realtime_data_response(&payloads, system_timestamp);
                lifecycle.send_response_bytes(response.as_bytes())
            }
            None => false,
        }
    }

    fn send_all_data_packet_response<B: AppDataBindings, M: MotorTelemetryBindings>(
        &self,
        lifecycle: &RefloatPackageLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        match (
            RefloatAllDataRequest::parse(bytes),
            telemetry.firmware_fault(),
        ) {
            (Err(_), _) => false,
            (Ok(_), fault) if !fault.is_none() => fault.compat_code().is_some_and(|fault_code| {
                let response = RefloatAllDataResponse::fault(
                    RefloatFirmwareFaultCode::from_compat_code(fault_code),
                );
                lifecycle.send_response_bytes(response.as_bytes())
            }),
            (Ok(request), _) => {
                let mode = request.mode();
                let payloads =
                    self.all_data_payloads
                        .with_base_battery_voltage(BatteryVoltage::new(
                            telemetry.input_voltage_filtered().voltage(),
                        ));
                let payloads = if mode.includes_mode2() {
                    self.runtime_all_data_payloads(payloads, telemetry, mode.includes_mode3())
                } else {
                    payloads
                };
                lifecycle.send_all_data_response(&payloads, request)
            }
        }
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    pub fn handle_packet_with_telemetry<B: AppDataBindings, M: MotorTelemetryBindings>(
        &mut self,
        lifecycle: &RefloatPackageLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        self.handle_charging_state_packet(bytes)
            || self.handle_handtest_packet(bytes)
            || self.handle_rc_move_packet(bytes)
            || self.send_metadata_packet_response(lifecycle, bytes)
            || self.send_realtime_data_packet_response(lifecycle, telemetry, bytes)
            || self.send_all_data_packet_response(lifecycle, telemetry, bytes)
    }

    fn refresh_motor_runtime_state<M: MotorTelemetryBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
    ) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let motor = base.motor();
        // Refloat v1.2.1 updates motor fields in `motor_data_update` at
        // `third_party/refloat/src/motor_data.c:108-145`. Battery current uses the same first-order
        // smoothing expression from `third_party/refloat/src/motor_data.c:140`; this app-data
        // refresh is still a runtime proxy until the real source main loop runs.
        let previous_battery_current = motor.battery_current().current().as_amps();
        let next_battery_current = telemetry.battery_current().current().as_amps();
        self.motor_current_max = telemetry.motor_current_max();
        self.motor_current_min = telemetry.motor_current_min();
        let electrical_speed = telemetry.electrical_speed();
        let motor_erpm = electrical_speed.rpm().as_revolutions_per_minute();
        let current_acceleration = motor_erpm - self.motor_last_erpm;
        self.motor_last_erpm = motor_erpm;
        // Upstream averages acceleration over `ACCEL_ARRAY_SIZE == 40` samples
        // in `third_party/refloat/src/motor_data.c:128-133`.
        let accel_idx = self.motor_accel_idx.min(self.motor_accel_history.len() - 1);
        let Some(previous_acceleration) = self.motor_accel_history.get(accel_idx).copied() else {
            return;
        };
        self.motor_acceleration += (current_acceleration - previous_acceleration) / 40.0;
        let Some(history) = self.motor_accel_history.get_mut(accel_idx) else {
            return;
        };
        *history = current_acceleration;
        self.motor_accel_idx = (accel_idx + 1) % 40;
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
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: f32, adc2: f32) {
        let adc2 = if adc2 < 0.0 { 0.0 } else { adc2 };
        let fault_adc1 = f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC1_OFFSET)) / 1000.0;
        let fault_adc2 = f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC2_OFFSET)) / 1000.0;
        // C map: Refloat v1.2.1 `footpad_sensor_update` decodes the switch
        // state from raw ADC volts at `third_party/refloat/src/footpad_sensor.c:28-61`.
        let mut state = FootpadSensorState::None;
        if fault_adc1 == 0.0 && fault_adc2 == 0.0 {
            state = FootpadSensorState::Both;
        } else if fault_adc2 == 0.0 {
            if adc1 > fault_adc1 {
                state = FootpadSensorState::Both;
            }
        } else if fault_adc1 == 0.0 {
            if adc2 > fault_adc2 {
                state = FootpadSensorState::Both;
            }
        } else if adc1 > fault_adc1 {
            state = if adc2 > fault_adc2 {
                FootpadSensorState::Both
            } else {
                FootpadSensorState::Left
            };
        } else if adc2 > fault_adc2 {
            state = FootpadSensorState::Right;
        }
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            crate::domain::FootpadSensorSample::from_adc_volts(adc1, adc2, state),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn refresh_imu_runtime_state<I: ImuBindings>(
        &mut self,
        imu: &ImuApi<I>,
        system_time_ticks: u32,
    ) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let resets_runtime_vars =
            matches!(ride_state.run_state(), RefloatRunState::Startup) && imu.startup_done();
        let run_state = match (ride_state.run_state(), imu.startup_done()) {
            (RefloatRunState::Startup, true) => RefloatRunState::Ready,
            (run_state, _) => run_state,
        };
        let flywheel_both_footpads_fault = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Running,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let reverse_stop_no_footpads_fault = matches!(
            (
                run_state,
                ride_state.setpoint_adjustment(),
                base.footpad().state()
            ),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop,
                FootpadSensorState::None
            )
        );
        let reverse_stop_pitch_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && imu.pitch().angle().as_radians().abs()
            > 18.0_f32.to_radians();
        let reverse_stop_timer_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && {
            let pitch = imu.pitch().angle().as_radians().abs();
            (pitch > 10.0_f32.to_radians()
                && refloat_ticks_elapsed(system_time_ticks, self.reverse_ticks, 1))
                || (pitch > 5.0_f32.to_radians()
                    && refloat_ticks_elapsed(system_time_ticks, self.reverse_ticks, 2))
        };
        let reverse_stop_total_erpm_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && self.reverse_total_erpm.abs() > 200_000.0;
        let motor_erpm = base
            .motor()
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute();
        let pitch = imu.pitch().angle().as_radians();
        // C updates `imu.balance_pitch` from the Refloat-owned balance filter
        // before control at `third_party/refloat/src/main.c:760-775`, `third_party/refloat/src/imu.c:35-41`, and
        // `third_party/refloat/src/balance_filter.c:145-154`; FLYWHEEL then overrides it with raw
        // pitch at `third_party/refloat/src/imu.c:56-58`.
        let balance_pitch = if matches!(ride_state.mode(), RefloatMode::Flywheel) {
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(pitch))
        } else {
            self.balance_filter.balance_pitch()
        };
        let balance_pitch_radians = balance_pitch.angle().as_radians();
        let balance_pitch_degrees = balance_pitch_radians * 180.0 / core::f32::consts::PI;
        let quickstop_fault = matches!(
            (run_state, base.footpad().state(), ride_state.mode()),
            (
                RefloatRunState::Running,
                FootpadSensorState::None,
                mode
            ) if !matches!(mode, RefloatMode::Flywheel)
        ) && self.config_byte(REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET) != 0
            && motor_erpm.abs() < 200.0
            && pitch.abs() > 14.0_f32.to_radians()
            && base.setpoints().remote().angle().as_degrees().abs() < 30.0
            && (pitch >= 0.0) == (motor_erpm >= 0.0);
        let single_footpad = matches!(
            base.footpad().state(),
            FootpadSensorState::Left | FootpadSensorState::Right
        );
        let dual_switch = self.config_byte(REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET) != 0;
        let simple_start = self.config_byte(REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET) != 0
            && (refloat_ticks_elapsed(system_time_ticks, self.disengage_ticks, 2)
                || !refloat_ticks_elapsed(system_time_ticks, self.engage_ticks, 1));
        let can_engage = matches!(ride_state.charging(), RefloatChargingState::NotCharging)
            && (matches!(base.footpad().state(), FootpadSensorState::Both)
                || single_footpad && (dual_switch || simple_start)
                || matches!(ride_state.mode(), RefloatMode::Flywheel));
        let fault_adc_half_erpm =
            f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC_HALF_ERPM_OFFSET));
        let fault_delay_switch_half =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_SWITCH_HALF_OFFSET));
        let fault_delay_switch_full =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_SWITCH_FULL_OFFSET));
        let switch_faults_disabled =
            self.config_byte(REFLOAT_CONFIG_FAULT_MOVING_FAULT_DISABLED_OFFSET) != 0
                && motor_erpm > fault_adc_half_erpm * 2.0
                && imu.roll().angle().as_radians().abs() < 40.0_f32.to_radians();
        let full_switch_pending = matches!(run_state, RefloatRunState::Running)
            && matches!(base.footpad().state(), FootpadSensorState::None)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel);
        let full_switch_fault = full_switch_pending
            && !switch_faults_disabled
            && (refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_switch_ticks,
                fault_delay_switch_full,
            ) || motor_erpm.abs() < fault_adc_half_erpm * 6.0
                && refloat_ticks_elapsed_ms(
                    system_time_ticks,
                    self.fault_switch_ticks,
                    fault_delay_switch_half,
                ));
        let half_switch_pending = matches!(run_state, RefloatRunState::Running)
            && !dual_switch
            && !can_engage
            && motor_erpm.abs() < fault_adc_half_erpm;
        let half_switch_fault = half_switch_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_switch_half_ticks,
                fault_delay_switch_half,
            );
        let fault_roll = self.config_scaled_i16(REFLOAT_CONFIG_FAULT_ROLL_OFFSET, 10.0);
        let fault_delay_roll =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET));
        let roll_fault_pending = matches!(run_state, RefloatRunState::Running)
            && imu.roll().angle().as_radians().abs() > fault_roll.to_radians();
        let roll_fault = roll_fault_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_angle_roll_ticks,
                fault_delay_roll,
            );
        let fault_pitch = self.config_scaled_i16(REFLOAT_CONFIG_FAULT_PITCH_OFFSET, 10.0);
        let fault_delay_pitch =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET));
        let pitch_fault_pending = matches!(run_state, RefloatRunState::Running)
            && imu.pitch().angle().as_radians().abs() > fault_pitch.to_radians()
            && base.setpoints().remote().angle().as_degrees().abs() < 30.0;
        let pitch_fault = pitch_fault_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_angle_pitch_ticks,
                fault_delay_pitch,
            );
        let ready_flywheel_stop = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Ready,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let darkride_high_erpm_fault = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Active)
        ) && motor_erpm > 2000.0;
        let darkride_can_engage_fault = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Active)
        ) && can_engage;
        let darkride_roll_fault =
            matches!(
                (run_state, ride_state.darkride()),
                (RefloatRunState::Running, RefloatDarkRideState::Upright)
            ) && self.config_byte(REFLOAT_CONFIG_FAULT_DARKRIDE_ENABLED_OFFSET) != 0
                && {
                    let roll = imu.roll().angle().as_radians().abs();
                    roll > 100.0_f32.to_radians() && roll < 135.0_f32.to_radians()
                };
        let startup_pitch_tolerance =
            self.config_scaled_i16(REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET, 100.0);
        let startup_roll_tolerance =
            self.config_scaled_i16(REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET, 100.0);
        let ready_engage = matches!(run_state, RefloatRunState::Ready)
            && !ready_flywheel_stop
            && can_engage
            && balance_pitch_radians.abs() < startup_pitch_tolerance.to_radians()
            && imu.roll().angle().as_radians().abs() < startup_roll_tolerance.to_radians();
        let ready_darkride_engage = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Ready, RefloatDarkRideState::Active)
        ) && balance_pitch_radians.abs()
            < startup_pitch_tolerance.to_radians()
            && !refloat_ticks_elapsed(system_time_ticks, self.disengage_ticks, 1)
            && !matches!(
                ride_state.stop_condition(),
                RefloatStopCondition::ReverseStop
            );
        let ready_push_start = matches!(run_state, RefloatRunState::Ready)
            && self.config_byte(REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET) != 0
            && motor_erpm.abs() > 1000.0
            && can_engage
            && balance_pitch_radians.abs() < core::f32::consts::FRAC_PI_4
            && imu.roll().angle().as_radians().abs() < core::f32::consts::FRAC_PI_4
            && !(self.config_byte(REFLOAT_CONFIG_FAULT_REVERSESTOP_ENABLED_OFFSET) != 0
                && motor_erpm < 0.0);
        let state_engage = ready_engage || ready_darkride_engage || ready_push_start;
        let traction_loss_detected = matches!(run_state, RefloatRunState::Running)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel)
            && self.motor_acceleration.abs() > 15.0
            && self.motor_acceleration.signum() == motor_erpm.signum()
            && base.motor().duty_cycle().ratio().as_ratio() > 0.3
            && motor_erpm.abs() > 2000.0;
        if traction_loss_detected {
            self.traction_control = matches!(ride_state.darkride(), RefloatDarkRideState::Active);
        } else if matches!(ride_state.wheelslip(), RefloatWheelSlipState::Detected)
            && self.motor_acceleration.abs() < 10.0
        {
            self.traction_control = false;
        }
        // Upstream `check_faults(d)` returns immediately after each stop branch
        // in `third_party/refloat/src/main.c:357-509`; this call preserves the
        // same Rust condition priority before `state_stop` writes READY and
        // clears wheelslip at `third_party/refloat/src/state.c:29-33`.
        let stop_event = refloat_first_stop_event(&[
            (
                RefloatStopEvent::FlywheelBothFootpads,
                flywheel_both_footpads_fault,
            ),
            (
                RefloatStopEvent::ReverseStopNoFootpads,
                reverse_stop_no_footpads_fault,
            ),
            (RefloatStopEvent::ReverseStopPitch, reverse_stop_pitch_fault),
            (RefloatStopEvent::ReverseStopTimer, reverse_stop_timer_fault),
            (
                RefloatStopEvent::ReverseStopTotalErpm,
                reverse_stop_total_erpm_fault,
            ),
            (RefloatStopEvent::FullSwitch, full_switch_fault),
            (RefloatStopEvent::QuickStop, quickstop_fault),
            (RefloatStopEvent::HalfSwitch, half_switch_fault),
            (RefloatStopEvent::DarkrideHighErpm, darkride_high_erpm_fault),
            (
                RefloatStopEvent::DarkrideCanEngage,
                darkride_can_engage_fault,
            ),
            (RefloatStopEvent::Roll, roll_fault),
            (RefloatStopEvent::Pitch, pitch_fault),
            (RefloatStopEvent::DarkrideRoll, darkride_roll_fault),
        ]);
        let state_transition = refloat_state_transition(RefloatStateTransitionInput {
            previous: ride_state,
            run_state,
            ready_flywheel_stop,
            state_engage,
            traction_loss_detected,
            stop_event,
        });
        let state_stop_fault = state_transition.state_stopped;
        if state_transition.state_stopped {
            self.disengage_ticks = system_time_ticks;
        } else if state_transition.state_engaged {
            self.engage_ticks = system_time_ticks;
        }
        if !full_switch_pending {
            self.fault_switch_ticks = system_time_ticks;
        }
        if !half_switch_pending {
            self.fault_switch_half_ticks = system_time_ticks;
        }
        if !matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) || imu.pitch().angle().as_radians().abs() < 5.0_f32.to_radians()
        {
            self.reverse_ticks = system_time_ticks;
        }
        if !roll_fault_pending {
            self.fault_angle_roll_ticks = system_time_ticks;
        }
        if !pitch_fault_pending {
            self.fault_angle_pitch_ticks = system_time_ticks;
        }
        // Upstream READY engages at `third_party/refloat/src/main.c:1033-1067`;
        // `state_engage` writes RUNNING/CENTERING/STOP_NONE at
        // `third_party/refloat/src/state.c:36-39`; READY flywheel abort returns
        // to NORMAL before startup checks at `third_party/refloat/src/main.c:957-963`.
        let mut ride_state = state_transition.ride_state;
        let reset_runtime_vars = resets_runtime_vars || state_engage;
        let (mut balance_current, mut setpoints, mut booster_current) = if reset_runtime_vars {
            // Upstream `STATE_STARTUP` calls `reset_runtime_vars(d)` before
            // `STATE_READY` at `third_party/refloat/src/main.c:833-837`, and
            // `engage(d)` calls it before `state_engage(d)` at
            // `third_party/refloat/src/main.c:263-270`; reset clears
            // `balance_current` at `third_party/refloat/src/main.c:246`,
            // resets module setpoints at `third_party/refloat/src/main.c:239-244`,
            // and seeds only the board setpoint from `d->imu.balance_pitch` at
            // `third_party/refloat/src/main.c:249-252`.
            self.pid_integral_current = 0.0;
            self.softstart_pid_limit = 0.0;
            self.reverse_total_erpm = 0.0;
            self.traction_control = false;
            self.rc_current = 0.0;
            self.rc_steps = 0;
            let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
                balance_pitch_degrees,
            ));
            let zero_setpoint =
                RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
            (
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                RefloatRealtimeRuntimeSetpoints::new(
                    setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                ),
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            )
        } else {
            (
                base.balance_current(),
                base.setpoints(),
                base.booster_current(),
            )
        };
        if matches!(run_state, RefloatRunState::Running) && !state_engage {
            if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::Centering
            ) {
                let board_setpoint_degrees = setpoints.board().angle().as_degrees();
                if board_setpoint_degrees == 0.0 {
                    // Upstream `calculate_setpoint_target(d)` exits
                    // `SAT_CENTERING` when `setpoint_target_interpolated`
                    // already equals target zero at
                    // `third_party/refloat/src/main.c:517-520`.
                    ride_state = RefloatRideState::new(
                        ride_state.run_state(),
                        ride_state.mode(),
                        RefloatSetpointAdjustment::None,
                        ride_state.stop_condition(),
                    )
                    .with_charging(ride_state.charging())
                    .with_wheelslip(ride_state.wheelslip())
                    .with_darkride(ride_state.darkride());
                } else {
                    let startup_step = self
                        .config_scaled_i16(REFLOAT_CONFIG_STARTUP_SPEED_OFFSET, 100.0)
                        / f32::from(self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET).max(1));
                    let centered_board_degrees = if board_setpoint_degrees.abs() < startup_step {
                        0.0
                    } else {
                        board_setpoint_degrees - startup_step * board_setpoint_degrees.signum()
                    };
                    // Upstream stores `startup_speed / hertz` at
                    // `third_party/refloat/src/main.c:172`, selects it for
                    // `SAT_CENTERING` at `third_party/refloat/src/main.c:304-310`,
                    // applies `rate_limitf` at
                    // `third_party/refloat/src/utils.c:25-33`, and assigns the
                    // centered setpoint before PID at
                    // `third_party/refloat/src/main.c:869-875`.
                    let centered_board = RefloatRealtimeRuntimeSetpoint::new(
                        AngleDegrees::from_degrees(centered_board_degrees),
                    );
                    setpoints = RefloatRealtimeRuntimeSetpoints::new(
                        centered_board,
                        setpoints.atr(),
                        setpoints.brake_tilt(),
                        setpoints.torque_tilt(),
                        setpoints.turn_tilt(),
                        setpoints.remote(),
                    );
                }
            }
            if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::ReverseStop
            ) {
                // Upstream `calculate_setpoint_target(d)` accumulates ERPM
                // while SAT_REVERSESTOP is active at `third_party/refloat/src/main.c:522-525`.
                self.reverse_total_erpm += motor_erpm;
            }
            let [_, gyro_pitch, gyro_yaw] = imu.angular_rate().xyz();
            // Upstream RUNNING executes this exact balance-current pipeline at
            // `third_party/refloat/src/main.c:918-956`; the helper keeps the
            // PID, booster, pitch-rate, soft-start, limit, darkride, and
            // traction branches unit-testable while this method preserves the
            // surrounding state-machine order.
            let balance_loop = refloat_balance_loop_step(
                RefloatBalanceLoopConfig {
                    kp: self.config_scaled_field(REFLOAT_CONFIG_KP_FIELD),
                    kp2: self.config_scaled_field(REFLOAT_CONFIG_KP2_FIELD),
                    ki: self.config_scaled_field(REFLOAT_CONFIG_KI_FIELD),
                    kp_brake: self.config_scaled_field(REFLOAT_CONFIG_KP_BRAKE_FIELD),
                    kp2_brake: self.config_scaled_field(REFLOAT_CONFIG_KP2_BRAKE_FIELD),
                    ki_limit: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_KI_LIMIT_OFFSET, 10.0),
                    )),
                    booster_angle: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET, 100.0),
                    ),
                    booster_ramp: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET, 100.0),
                    ),
                    booster_current: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET, 100.0),
                    )),
                    brkbooster_angle: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET, 100.0),
                    ),
                    brkbooster_ramp: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET, 100.0),
                    ),
                    brkbooster_current: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET, 100.0),
                    )),
                    hertz: SampleRate::from_hertz(f32::from(
                        self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET),
                    )),
                },
                RefloatBalanceLoopInput {
                    setpoint: setpoints.board(),
                    brake_tilt_setpoint: setpoints.brake_tilt(),
                    balance_pitch,
                    raw_pitch: imu.pitch(),
                    roll: imu.roll(),
                    gyro_pitch,
                    gyro_yaw,
                    motor_erpm: base.motor().electrical_speed(),
                    motor_current: base.motor().motor_current(),
                    motor_current_max: self.motor_current_max,
                    motor_current_min: self.motor_current_min,
                    mode: ride_state.mode(),
                    darkride: ride_state.darkride(),
                    traction_control: self.traction_control,
                },
                RefloatBalanceLoopState {
                    balance_current: balance_current.current(),
                    booster_current: booster_current.current(),
                    pid_integral_current: MotorCurrent::new(Current::from_amps(
                        self.pid_integral_current,
                    )),
                    pid_kp_brake_scale: self.pid_kp_brake_scale,
                    pid_kp2_brake_scale: self.pid_kp2_brake_scale,
                    pid_kp_accel_scale: self.pid_kp_accel_scale,
                    pid_kp2_accel_scale: self.pid_kp2_accel_scale,
                    softstart_pid_limit: MotorCurrent::new(Current::from_amps(
                        self.softstart_pid_limit,
                    )),
                },
            );
            let balance_loop_state = balance_loop.state;
            self.pid_integral_current = balance_loop_state.pid_integral_current.current().as_amps();
            self.pid_kp_brake_scale = balance_loop_state.pid_kp_brake_scale;
            self.pid_kp2_brake_scale = balance_loop_state.pid_kp2_brake_scale;
            self.pid_kp_accel_scale = balance_loop_state.pid_kp_accel_scale;
            self.pid_kp2_accel_scale = balance_loop_state.pid_kp2_accel_scale;
            self.softstart_pid_limit = balance_loop_state.softstart_pid_limit.current().as_amps();
            booster_current =
                RefloatRealtimeBoosterCurrent::new(balance_loop_state.booster_current);
            balance_current =
                RefloatRealtimeBalanceCurrent::new(balance_loop_state.balance_current);
            self.request_motor_current(balance_loop.requested_current);
        } else if matches!(run_state, RefloatRunState::Ready) && !state_stop_fault {
            if self.rc_steps != 0 {
                self.rc_current =
                    self.rc_current * 0.95 + f32::from(self.rc_current_target_deciamps) * 0.005;
                if motor_erpm.abs() > 800.0 {
                    self.rc_current = 0.0;
                }
                self.rc_steps -= 1;
                self.rc_counter += 1;
                if self.rc_counter == 500 && self.rc_current_target_deciamps > 20 {
                    self.rc_current_target_deciamps /= 2;
                }
                // Upstream READY falls through to `do_rc_move(d)` at
                // `third_party/refloat/src/main.c:1069`, where active RC move steps filter/request
                // `rc_current` at `third_party/refloat/src/main.c:276-286`.
                self.request_motor_current(MotorCurrent::new(Current::from_amps(self.rc_current)));
            } else {
                let remote_throttle_current_max =
                    self.config_scaled_i16(REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET, 10.0);
                let remote_throttle_grace_period = self
                    .config_scaled_i16(REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET, 10.0);
                if remote_throttle_current_max > 0.0
                    && refloat_ticks_elapsed_f32(
                        system_time_ticks,
                        self.disengage_ticks,
                        remote_throttle_grace_period,
                    )
                    && self.remote_input.abs() > 0.02
                {
                    let servo_val =
                        if self.config_byte(REFLOAT_CONFIG_INPUTTILT_INVERT_THROTTLE_OFFSET) != 0 {
                            -self.remote_input
                        } else {
                            self.remote_input
                        };
                    self.rc_current =
                        self.rc_current * 0.95 + remote_throttle_current_max * servo_val * 0.05;
                    // Upstream READY falls through to `do_rc_move(d)` at
                    // `third_party/refloat/src/main.c:1069`, where the remote-throttle idle branch
                    // filters and requests `rc_current` at `third_party/refloat/src/main.c:291-298`.
                    self.request_motor_current(MotorCurrent::new(Current::from_amps(
                        self.rc_current,
                    )));
                } else {
                    self.rc_current = 0.0;
                }
            }
        }
        // C publishes the just-refreshed `imu.balance_pitch` through app-data;
        // normal mode comes from the balance filter at `third_party/refloat/src/imu.c:35-41`, while
        // FLYWHEEL mirrors raw pitch at `third_party/refloat/src/imu.c:56-58`.
        let base = RefloatAllDataBasePayload::new(
            balance_current,
            RefloatAllDataAttitude::new(balance_pitch, imu.roll(), imu.pitch()),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            setpoints,
            booster_current,
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn handle_charging_state_packet(&mut self, bytes: &[u8]) -> bool {
        // Refloat v1.2.1 routes COMMAND_CHARGING_STATE at `third_party/refloat/src/main.c:2267-2269`;
        // the command ID is defined in `third_party/refloat/src/charging.h:25`.
        match refloat_command_payload(bytes, RefloatAppDataCommand::ChargingState) {
            Some(
                [
                    151,
                    charging,
                    voltage_hi,
                    voltage_lo,
                    current_hi,
                    current_lo,
                    ..,
                ],
            ) => {
                let (voltage, current) = match *charging {
                    0 => (0.0, 0.0),
                    _ => (
                        refloat_read_scaled_i16([*voltage_hi, *voltage_lo], 10.0),
                        refloat_read_scaled_i16([*current_hi, *current_lo], 10.0),
                    ),
                };
                self.all_data_payloads =
                    self.all_data_payloads
                        .with_mode4_charging(RefloatAllDataMode4Payload::new(
                            RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(
                                Current::from_amps(current),
                            )),
                            RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(
                                Voltage::from_volts(voltage),
                            )),
                        ));
                true
            }
            _ => false,
        }
    }

    fn runtime_all_data_payloads<M: MotorTelemetryBindings>(
        self,
        payloads: RefloatAllDataPayloads,
        telemetry: &MotorTelemetryApi<M>,
        include_mode3: bool,
    ) -> RefloatAllDataPayloads {
        let payloads = payloads
            .with_mode2_distance_abs(telemetry.distance_abs())
            .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads.with_mode3_ride_totals(RefloatAllDataMode3Payload::new(
                telemetry.odometer(),
                telemetry.amp_hours_discharged(),
                telemetry.amp_hours_charged(),
                telemetry.watt_hours_discharged(),
                telemetry.watt_hours_charged(),
                telemetry.battery_level(),
            ))
        } else {
            payloads
        }
    }
}

#[cfg(test)]
mod tests;
