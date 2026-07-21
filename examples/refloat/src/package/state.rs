use super::time::{refloat_ticks_elapsed, refloat_ticks_elapsed_seconds};
use crate::balance::{BalanceFilter, LoopInput, LoopState};
#[cfg(any(test, target_arch = "arm"))]
use crate::beeper::RefloatBeeperLevel;
use crate::beeper::{RefloatBeeper, RefloatBeeperAlert, RefloatBeeperCount};
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::RefloatBmsFaults;
use crate::bms::RefloatBmsSample;
use crate::config::*;
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude, RefloatAllDataBasePayload,
    RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatDarkRideState, RefloatFootpadState, RefloatMode, RefloatRealtimeBalanceCurrent,
    RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent, RefloatRealtimeRuntimeSetpoint,
    RefloatRealtimeRuntimeSetpoints, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};
use crate::motor_control::RefloatMotorControl;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{AdcVoltage, FirmwareVersion};
use vescpkg_rs::prelude::{
    AngleRadians, BatteryCellCount, BatteryVoltage, Current, DutyCycleLimit, MosfetTemperature,
    MotorCurrent, MotorCurrentLimit, MotorTemperature, Ratio, Rpm, Temperature,
    TemperatureLimitStart, TimestampTicks, Voltage,
};
use vescpkg_rs::{Imu, MotorOutput, MotorTelemetry};

#[cfg(test)]
mod balance_tests;
mod charging;
mod config_runtime;
mod config_storage;
#[cfg(any(test, target_arch = "arm"))]
mod footpad_runtime;
mod handtest;
mod imu_runtime;
mod limits;
mod motor_acceleration;
mod motor_runtime;
#[cfg(test)]
mod motor_telemetry_tests;
mod packet_response;
mod remote_control;
#[cfg(test)]
mod runtime_tests;
mod transition;
#[cfg(test)]
mod transition_tests;
mod tuning;
#[cfg(test)]
mod tuning_tests;

use motor_acceleration::MotorAccelerationTracker;
use remote_control::RemoteControlState;
use transition::{
    RefloatStateTransitionInput, RefloatStopEvent, refloat_first_stop_event,
    refloat_state_transition,
};

#[inline]
/// C map: `on_command_received` in `third_party/refloat/src/main.c:2143-2225` filters
/// app-data packets by package byte and command ID before dispatching to per-command handlers.
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
    serialized_config: RefloatConfigImage,
    beeper: RefloatBeeper,
    beeper_pin_configured: bool,
    bms_sample: RefloatBmsSample,
    #[cfg(any(test, target_arch = "arm"))]
    bms_faults: RefloatBmsFaults,
    #[cfg(any(test, target_arch = "arm"))]
    bms_start_ticks: Option<TimestampTicks>,
    #[cfg(any(test, target_arch = "arm"))]
    bms_alert_ticks: TimestampTicks,
    handtest_config_backup: Option<RefloatConfigImage>,
    motor_control: RefloatMotorControl,
    balance_filter: BalanceFilter,
    traction_control: bool,
    balance_loop: LoopState,
    reverse_total_erpm: Rpm,
    motor_acceleration: MotorAccelerationTracker,
    motor_current_filter: motor_runtime::RefloatMotorCurrentFilter,
    remote_control: RemoteControlState,
    charging_ticks: TimestampTicks,
    engage_ticks: TimestampTicks,
    disengage_ticks: TimestampTicks,
    idle_ticks: TimestampTicks,
    nag_ticks: TimestampTicks,
    idle_voltage: BatteryVoltage,
    fault_switch_ticks: TimestampTicks,
    fault_switch_half_ticks: TimestampTicks,
    reverse_ticks: TimestampTicks,
    fault_angle_pitch_ticks: TimestampTicks,
    fault_angle_roll_ticks: TimestampTicks,
    high_voltage_ticks: TimestampTicks,
    wheelslip_ticks: TimestampTicks,
    motor_duty_raw: Ratio,
    duty_max_with_margin: DutyCycleLimit,
    motor_current_max: MotorCurrentLimit,
    motor_current_min: MotorCurrentLimit,
    mosfet_temperature: MosfetTemperature,
    motor_temperature: MotorTemperature,
    mosfet_temperature_limit_start: TemperatureLimitStart,
    motor_temperature_limit_start: TemperatureLimitStart,
    battery_cell_count: Option<BatteryCellCount>,
    #[cfg(any(test, target_arch = "arm"))]
    firmware_version: Option<FirmwareVersion>,
}

impl RefloatPackageState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        let serialized_config = RefloatConfigImage::defaults();
        Self {
            all_data_payloads,
            // Upstream `data_init` reads EEPROM and falls back to generated
            // defaults at `third_party/refloat/src/main.c:1160-1185`; full EEPROM parity remains a
            // later source-backed slice.
            serialized_config,
            beeper: RefloatBeeper::new(serialized_config.beeper_enabled()),
            beeper_pin_configured: false,
            bms_sample: RefloatBmsSample::source_startup(),
            #[cfg(any(test, target_arch = "arm"))]
            bms_faults: RefloatBmsFaults::NONE,
            #[cfg(any(test, target_arch = "arm"))]
            bms_start_ticks: None,
            #[cfg(any(test, target_arch = "arm"))]
            bms_alert_ticks: TimestampTicks::from_ticks(0),
            handtest_config_backup: None,
            motor_control: RefloatMotorControl::new(),
            balance_filter: BalanceFilter::source_startup(),
            traction_control: false,
            balance_loop: LoopState::source_startup(),
            reverse_total_erpm: Rpm::ZERO,
            motor_acceleration: MotorAccelerationTracker::default(),
            motor_current_filter: motor_runtime::RefloatMotorCurrentFilter::source_startup(),
            remote_control: RemoteControlState::default(),
            charging_ticks: TimestampTicks::from_ticks(0),
            engage_ticks: TimestampTicks::from_ticks(0),
            disengage_ticks: TimestampTicks::from_ticks(0),
            idle_ticks: TimestampTicks::from_ticks(0),
            nag_ticks: TimestampTicks::from_ticks(0),
            idle_voltage: BatteryVoltage::new(Voltage::ZERO),
            fault_switch_ticks: TimestampTicks::from_ticks(0),
            fault_switch_half_ticks: TimestampTicks::from_ticks(0),
            reverse_ticks: TimestampTicks::from_ticks(0),
            fault_angle_pitch_ticks: TimestampTicks::from_ticks(0),
            fault_angle_roll_ticks: TimestampTicks::from_ticks(0),
            high_voltage_ticks: TimestampTicks::from_ticks(0),
            wheelslip_ticks: TimestampTicks::from_ticks(0),
            motor_duty_raw: Ratio::from_ratio_const(0.0),
            duty_max_with_margin: DutyCycleLimit::new(Ratio::from_ratio_const(0.0)),
            motor_current_max: MotorCurrentLimit::new(Current::ZERO),
            motor_current_min: MotorCurrentLimit::new(Current::ZERO),
            mosfet_temperature: MosfetTemperature::new(Temperature::ZERO),
            motor_temperature: MotorTemperature::new(Temperature::ZERO),
            mosfet_temperature_limit_start: TemperatureLimitStart::new(Temperature::ZERO),
            motor_temperature_limit_start: TemperatureLimitStart::new(Temperature::ZERO),
            battery_cell_count: None,
            #[cfg(any(test, target_arch = "arm"))]
            firmware_version: None,
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_firmware_version(&mut self, version: FirmwareVersion) {
        self.firmware_version = Some(version);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_bms_sample(&mut self, sample: RefloatBmsSample) {
        self.bms_sample = sample;
    }

    pub(crate) fn alert_beeper(&mut self, alert: RefloatBeeperAlert) {
        self.beeper.alert(alert);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn tick_beeper(&mut self) -> Option<RefloatBeeperLevel> {
        self.beeper.tick()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn take_beeper_configuration_request(&mut self) -> bool {
        let configure = self.serialized_config.beeper_enabled() && !self.beeper_pin_configured;
        self.beeper_pin_configured |= configure;
        configure
    }

    #[cfg(any(test, target_arch = "arm"))]
    /// Recompute the BMS fault mask before control-loop state selection.
    ///
    /// C map: `bms_update` runs immediately before state logic at
    /// `third_party/refloat/src/main.c:824-831`.
    pub(crate) fn refresh_bms_runtime_state(&mut self, system_time_ticks: TimestampTicks) {
        let bms = self.serialized_config.bms();
        let enabled = bms.enabled();
        let thresholds = bms.thresholds();
        let start_ticks = *self.bms_start_ticks.get_or_insert(system_time_ticks);
        let startup_timeout_elapsed = refloat_ticks_elapsed_seconds(
            system_time_ticks,
            start_ticks,
            vescpkg_rs::VescSeconds::from_seconds(5.0),
        );
        self.bms_faults = RefloatBmsFaults::evaluate(
            enabled,
            self.bms_sample,
            thresholds,
            startup_timeout_elapsed,
        );
    }

    #[cfg(test)]
    pub(crate) const fn bms_sample_for_test(&self) -> RefloatBmsSample {
        self.bms_sample
    }

    #[cfg(test)]
    pub(crate) const fn bms_faults_for_test(&self) -> RefloatBmsFaults {
        self.bms_faults
    }

    #[cfg(test)]
    pub(crate) const fn recorded_firmware_version(&self) -> Option<FirmwareVersion> {
        self.firmware_version
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    /// Apply and clear a pending motor-current request.
    pub fn apply_requested_motor_current(&mut self, motor: &impl MotorOutput) -> bool {
        self.motor_control.apply_requested_current(motor)
    }

    /// Apply motor control for the current run state.
    pub fn apply_motor_control(
        &mut self,
        motor: &impl MotorOutput,
        run_state: RefloatRunState,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let base = self.all_data_payloads.base();
        // Upstream `motor_control_configure` copies brake and parking config at
        // `third_party/refloat/src/motor_control.c:36-40`; this Rust state keeps
        // the serialized config as source of truth until full `Data` parity.
        self.motor_control.apply(
            motor,
            run_state,
            base.motor().electrical_speed().rpm().abs(),
            system_time_ticks,
            self.serialized_config.motor_control().parking_brake_mode(),
            self.serialized_config.motor_control().brake_current(),
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn update_balance_filter(&mut self, sample: vescpkg_rs::prelude::ImuReadSample) {
        self.balance_filter.update(sample);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn initialize_balance_filter(&mut self, orientation: vescpkg_rs::ImuOrientation) {
        // C map: `data_init` initializes the Refloat filter from VESC's live
        // quaternion through `balance_filter_init` before thread startup at
        // `third_party/refloat/src/main.c:1168-1171` and
        // `third_party/refloat/src/balance_filter.c:53-61`.
        self.balance_filter = BalanceFilter::from_orientation(orientation);
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    #[cfg(test)]
    pub(crate) fn set_balance_filter_for_test(&mut self, balance_filter: BalanceFilter) {
        self.balance_filter = balance_filter;
    }

    fn refresh_config_runtime_state(&mut self) {
        config_runtime::refresh(self);
    }

    /// Handle one app-data packet in the firmware callback context.
    ///
    /// Upstream `on_command_received` dispatches commands at
    /// `third_party/refloat/src/main.c:2143-2225`; the main
    /// `refloat_thd` owns `time_update`, `imu_update`, `motor_data_update`, and
    /// control-loop transitions at `third_party/refloat/src/main.c:772-1080`.
    pub fn handle_packet_with_runtime(
        &mut self,
        telemetry: &impl MotorTelemetry,
        _imu: &impl Imu,
        now: &mut impl FnMut() -> TimestampTicks,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        #[cfg(all(not(test), not(target_arch = "arm")))]
        self.refresh_runtime_state(telemetry, _imu, now());

        self.handle_packet_with_telemetry(telemetry, now, send, bytes)
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
    pub(crate) fn refresh_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_main_loop_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        footpad_adc1: AdcVoltage,
        footpad_adc2: AdcVoltage,
        system_time_ticks: TimestampTicks,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_footpad_runtime_state(footpad_adc1, footpad_adc2);
        self.refresh_charging_runtime_state(system_time_ticks);
        self.refresh_bms_runtime_state(system_time_ticks);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    fn handle_rc_move_packet(&mut self, bytes: &[u8]) -> bool {
        remote_control::handle_packet(self.all_data_payloads, &mut self.remote_control, bytes)
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    pub fn handle_packet_with_telemetry(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        self.handle_charging_state_packet(now, bytes)
            || self.handle_handtest_packet(bytes)
            || tuning::handle_runtime_tune_packet(self, bytes)
            || tuning::handle_booster_packet(self, bytes)
            || self.handle_rc_move_packet(bytes)
            || self.send_metadata_packet_response(send, bytes)
            || self.send_legacy_realtime_data_packet_response(send, bytes)
            || self.send_realtime_data_packet_response(telemetry, now, send, bytes)
            || self.send_all_data_packet_response(telemetry, send, bytes)
    }

    fn refresh_motor_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        motor_runtime::refresh(self, telemetry);
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: AdcVoltage, adc2: AdcVoltage) {
        footpad_runtime::refresh(self, adc1, adc2);
    }

    fn refresh_imu_runtime_state(&mut self, imu: &impl Imu, system_time_ticks: TimestampTicks) {
        imu_runtime::refresh(self, imu, system_time_ticks);
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn refresh_charging_runtime_state(&mut self, system_time_ticks: TimestampTicks) {
        self.all_data_payloads = charging::timeout(
            self.all_data_payloads,
            system_time_ticks,
            self.charging_ticks,
        );
    }

    fn handle_charging_state_packet(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> bool {
        match charging::handle_packet(self.all_data_payloads, bytes) {
            Some(payloads) => {
                self.all_data_payloads = payloads;
                self.charging_ticks = now();
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod footpad_tests;
#[cfg(test)]
mod motor_control_tests;
#[cfg(test)]
mod ready_darkride_tests;
#[cfg(test)]
mod ready_tests;
