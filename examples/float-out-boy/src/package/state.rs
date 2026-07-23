use super::time::{float_out_boy_ticks_elapsed, float_out_boy_ticks_elapsed_seconds};
use crate::balance::{BalanceFilter, LoopConfig, LoopInput, LoopState};
#[cfg(any(test, target_arch = "arm"))]
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::beeper::{FloatOutBoyBeeper, FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount};
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::FloatOutBoyBmsFaults;
use crate::bms::FloatOutBoyBmsSample;
use crate::config::*;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand,
    FloatOutBoyChargingState, FloatOutBoyDarkRideState, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};
use crate::motor_control::FloatOutBoyMotorControl;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{AdcVoltage, FirmwareVersion};
use vescpkg_rs::prelude::{
    AngleRadians, BatteryCellCount, BatteryVoltage, Current, DutyCycleLimit, InputCurrent,
    MosfetTemperature, MotorCurrent, MotorCurrentLimit, MotorTemperature, Ratio, Rpm, Temperature,
    TemperatureLimitStart, TimestampTicks, Voltage,
};
use vescpkg_rs::{Imu, MotorOutput, MotorTelemetry};

mod alert_tracker;
mod alerts;
#[cfg(test)]
mod balance_tests;
mod charging;
mod config_runtime;
mod config_storage;
mod flywheel;
#[cfg(any(test, target_arch = "arm"))]
mod footpad_runtime;
mod handtest;
#[cfg(any(test, target_arch = "arm"))]
mod haptic_feedback;
mod imu_runtime;
mod limits;
mod motor_acceleration;
mod motor_runtime;
#[cfg(test)]
mod motor_telemetry_tests;
mod packet_response;
mod remote_control;
mod ride_modifiers;
#[cfg(test)]
mod runtime_tests;
mod transition;
#[cfg(test)]
mod transition_tests;
mod tuning;
#[cfg(test)]
mod tuning_tests;

use alert_tracker::AlertTrackerState;
use flywheel::FloatOutBoyFlywheelOffsets;
#[cfg(any(test, target_arch = "arm"))]
use haptic_feedback::{HapticFeedbackInput, HapticFeedbackState};
use motor_acceleration::MotorAccelerationTracker;
use remote_control::RemoteControlState;
use ride_modifiers::{RideModifierInput, RideModifierState};
use transition::{
    FloatOutBoyStateTransitionInput, FloatOutBoyStopEvent, float_out_boy_first_stop_event,
    float_out_boy_state_transition,
};

#[inline]
/// C map: `on_command_received` in `third_party/float-out-boy/src/main.c:2143-2225` filters
/// app-data packets by package byte and command ID before dispatching to per-command handlers.
fn float_out_boy_command_payload(
    bytes: &[u8],
    command: FloatOutBoyAppDataCommand,
) -> Option<&[u8]> {
    match bytes {
        [package_id, command_id, payload @ ..]
            if *package_id == FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get()
                && *command_id == command.id() =>
        {
            Some(payload)
        }
        _ => None,
    }
}

/// Float Out Boy package state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyPackageState {
    all_data_payloads: FloatOutBoyAllDataPayloads,
    serialized_config: FloatOutBoyConfigImage,
    alert_tracker: AlertTrackerState,
    #[cfg(any(test, target_arch = "arm"))]
    haptic_feedback: HapticFeedbackState,
    beeper: FloatOutBoyBeeper,
    beeper_pin_configured: bool,
    duty_beeping: bool,
    bms_sample: FloatOutBoyBmsSample,
    #[cfg(any(test, target_arch = "arm"))]
    bms_faults: FloatOutBoyBmsFaults,
    #[cfg(any(test, target_arch = "arm"))]
    bms_start_ticks: Option<TimestampTicks>,
    #[cfg(any(test, target_arch = "arm"))]
    bms_alert_ticks: TimestampTicks,
    flywheel_offsets: FloatOutBoyFlywheelOffsets,
    flywheel_runtime_config: Option<FloatOutBoyFlywheelConfig>,
    flywheel_abort: bool,
    motor_control: FloatOutBoyMotorControl,
    balance_filter: BalanceFilter,
    traction_control: bool,
    balance_loop: LoopState,
    reverse_total_erpm: Rpm,
    motor_acceleration: MotorAccelerationTracker,
    motor_current_filter: motor_runtime::FloatOutBoyMotorCurrentFilter,
    remote_control: RemoteControlState,
    runtime_board_setpoint: vescpkg_rs::prelude::AngleDegrees,
    ride_modifiers: RideModifierState,
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
    upside_down_fault_ticks: TimestampTicks,
    upside_down_enabled: bool,
    upside_down_started: bool,
    motor_duty_raw: Ratio,
    duty_max_with_margin: DutyCycleLimit,
    motor_current_max: MotorCurrentLimit,
    motor_current_min: MotorCurrentLimit,
    battery_current_max: InputCurrent,
    battery_current_min: InputCurrent,
    mosfet_temperature: MosfetTemperature,
    motor_temperature: MotorTemperature,
    mosfet_temperature_limit_start: TemperatureLimitStart,
    motor_temperature_limit_start: TemperatureLimitStart,
    battery_cell_count: Option<BatteryCellCount>,
    #[cfg(any(test, target_arch = "arm"))]
    firmware_version: Option<FirmwareVersion>,
}

impl FloatOutBoyPackageState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let serialized_config = FloatOutBoyConfigImage::defaults();
        Self {
            all_data_payloads,
            serialized_config,
            alert_tracker: AlertTrackerState::default(),
            #[cfg(any(test, target_arch = "arm"))]
            haptic_feedback: HapticFeedbackState::new(),
            beeper: FloatOutBoyBeeper::new(serialized_config.beeper_enabled()),
            beeper_pin_configured: false,
            duty_beeping: false,
            bms_sample: FloatOutBoyBmsSample::source_startup(),
            #[cfg(any(test, target_arch = "arm"))]
            bms_faults: FloatOutBoyBmsFaults::NONE,
            #[cfg(any(test, target_arch = "arm"))]
            bms_start_ticks: None,
            #[cfg(any(test, target_arch = "arm"))]
            bms_alert_ticks: TimestampTicks::from_ticks(0),
            flywheel_offsets: FloatOutBoyFlywheelOffsets::source_startup(),
            flywheel_runtime_config: None,
            flywheel_abort: false,
            motor_control: FloatOutBoyMotorControl::new(),
            balance_filter: BalanceFilter::source_startup(),
            traction_control: false,
            balance_loop: LoopState::source_startup(),
            reverse_total_erpm: Rpm::ZERO,
            motor_acceleration: MotorAccelerationTracker::default(),
            motor_current_filter: motor_runtime::FloatOutBoyMotorCurrentFilter::source_startup(),
            remote_control: RemoteControlState::default(),
            runtime_board_setpoint: all_data_payloads.base().setpoints().board().angle(),
            ride_modifiers: RideModifierState::default(),
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
            upside_down_fault_ticks: TimestampTicks::from_ticks(0),
            upside_down_enabled: false,
            upside_down_started: false,
            motor_duty_raw: Ratio::from_ratio_const(0.0),
            duty_max_with_margin: DutyCycleLimit::new(Ratio::from_ratio_const(0.0)),
            motor_current_max: MotorCurrentLimit::new(Current::ZERO),
            motor_current_min: MotorCurrentLimit::new(Current::ZERO),
            battery_current_max: InputCurrent::new(Current::ZERO),
            battery_current_min: InputCurrent::new(Current::ZERO),
            mosfet_temperature: MosfetTemperature::new(Temperature::ZERO),
            motor_temperature: MotorTemperature::new(Temperature::ZERO),
            mosfet_temperature_limit_start: TemperatureLimitStart::new(Temperature::ZERO),
            motor_temperature_limit_start: TemperatureLimitStart::new(Temperature::ZERO),
            battery_cell_count: None,
            #[cfg(any(test, target_arch = "arm"))]
            firmware_version: None,
        }
    }

    #[cfg_attr(not(target_arch = "arm"), allow(dead_code))]
    pub(crate) fn refresh_controller_input(&mut self, input: &vescpkg_rs::ControllerInput) {
        // C map: Float Out Boy selects UART/PPM, rejects samples one second old,
        // applies deadband rescaling, then optional inversion at
        // `third_party/float-out-boy/src/remote.c:36-68`.
        let config = self.serialized_config;
        let value = match config.input_tilt_remote_type() {
            1 => {
                let remote = input.remote();
                (remote.age().duration() < vescpkg_rs::VescSeconds::from_seconds(1.0))
                    .then(|| remote.joystick_y().ratio().as_ratio())
            }
            2 => {
                let (ppm, age) = input.ppm();
                (age.duration() < vescpkg_rs::VescSeconds::from_seconds(1.0))
                    .then(|| ppm.ratio().as_ratio())
            }
            _ => None,
        }
        .unwrap_or(0.0);
        let deadband = config.input_tilt_deadband().as_ratio();
        let value = if value.abs() < deadband {
            0.0
        } else {
            value.signum() * (value.abs() - deadband) / (1.0 - deadband)
        };
        let value = if config.input_tilt_inverted() {
            -value
        } else {
            value
        };
        self.remote_control
            .set_input(crate::domain::FloatOutBoyRealtimeRemoteInput::new(
                vescpkg_rs::SignedRatio::clamped(value),
            ));
    }

    /// Build startup state and apply the config persisted by firmware.
    ///
    /// Upstream `data_init` reads EEPROM and falls back to generated defaults
    /// at `third_party/float-out-boy/src/main.c:1160-1185`.
    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn from_persisted_config(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let mut state = Self::new(all_data_payloads);
        state.load_persisted_config_on_startup();
        state
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_firmware_version(&mut self, version: FirmwareVersion) {
        self.firmware_version = Some(version);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_bms_sample(&mut self, sample: FloatOutBoyBmsSample) {
        self.bms_sample = sample;
    }

    pub(crate) fn alert_beeper(&mut self, alert: FloatOutBoyBeeperAlert) {
        self.beeper.alert(alert);
    }

    pub(crate) fn force_beeper_on(&mut self) {
        self.beeper.on(true);
    }

    pub(crate) fn release_beeper(&mut self) {
        self.beeper.off(false);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn tick_beeper(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.beeper.tick()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn take_beeper_level(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.beeper.take_level()
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
    /// `third_party/float-out-boy/src/main.c:824-831`.
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_bms_runtime_state(&mut self, system_time_ticks: TimestampTicks) {
        let bms = self.serialized_config.bms();
        let enabled = bms.enabled();
        let thresholds = bms.thresholds();
        let start_ticks = *self.bms_start_ticks.get_or_insert(system_time_ticks);
        let startup_timeout_elapsed = float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            start_ticks,
            vescpkg_rs::VescSeconds::from_seconds(5.0),
        );
        self.bms_faults = FloatOutBoyBmsFaults::evaluate(
            enabled,
            self.bms_sample,
            thresholds,
            startup_timeout_elapsed,
        );
    }

    #[cfg(test)]
    pub(crate) const fn bms_sample_for_test(&self) -> FloatOutBoyBmsSample {
        self.bms_sample
    }

    #[cfg(test)]
    pub(crate) const fn bms_faults_for_test(&self) -> FloatOutBoyBmsFaults {
        self.bms_faults
    }

    #[cfg(test)]
    pub(crate) const fn recorded_firmware_version(&self) -> Option<FirmwareVersion> {
        self.firmware_version
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> FloatOutBoyAllDataPayloads {
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
        run_state: FloatOutBoyRunState,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let base = self.all_data_payloads.base();
        // Upstream `motor_control_configure` copies brake and parking config at
        // `third_party/float-out-boy/src/motor_control.c:36-40`; this Rust state keeps
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
        // C map: `data_init` initializes the Float Out Boy filter from VESC's live
        // quaternion through `balance_filter_init` before thread startup at
        // `third_party/float-out-boy/src/main.c:1168-1171` and
        // `third_party/float-out-boy/src/balance_filter.c:53-61`.
        self.balance_filter = BalanceFilter::from_orientation(orientation);
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    #[cfg(test)]
    pub(crate) fn set_balance_filter_for_test(&mut self, balance_filter: BalanceFilter) {
        self.balance_filter = balance_filter;
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_config_runtime_state(&mut self) {
        config_runtime::refresh(self);
    }

    /// Handle one app-data packet in the firmware callback context.
    ///
    /// Upstream `on_command_received` dispatches commands at
    /// `third_party/float-out-boy/src/main.c:2143-2225`; the main
    /// `float_out_boy_thd` owns `time_update`, `imu_update`, `motor_data_update`, and
    /// control-loop transitions at `third_party/float-out-boy/src/main.c:772-1080`.
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

    /// Refresh the source-backed runtime slices that Float Out Boy updates near the
    /// top of `float_out_boy_thd`.
    ///
    /// C map: Float Out Boy v1.2.1 `imu_ref_callback` starts at `third_party/float-out-boy/src/main.c:760`.
    ///
    /// Upstream applies `configure(d)` before runtime work at
    /// `third_party/float-out-boy/src/main.c:184-191`, updates IMU at `third_party/float-out-boy/src/main.c:775`, motor data at
    /// `third_party/float-out-boy/src/main.c:796`, and performs the `STATE_STARTUP` -> `STATE_READY`
    /// gate at `third_party/float-out-boy/src/main.c:833-838`.
    pub(crate) fn refresh_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.alert_tracker.update_firmware_fault(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_main_loop_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        motor: &impl MotorOutput,
        footpad_adc1: AdcVoltage,
        footpad_adc2: AdcVoltage,
        system_time_ticks: TimestampTicks,
    ) {
        // Keep the ARM refresh phases in separate frames so LTO cannot merge
        // their independent stack use inside VESC's fixed thread working area.
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_haptic_runtime_state(motor, system_time_ticks);
        self.alert_tracker.update_firmware_fault(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        self.refresh_footpad_runtime_state(footpad_adc1, footpad_adc2);
        self.refresh_charging_runtime_state(system_time_ticks);
        self.refresh_bms_runtime_state(system_time_ticks);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    fn handle_rc_move_packet(&mut self, bytes: &[u8]) -> bool {
        remote_control::handle_packet(self.all_data_payloads, &mut self.remote_control, bytes)
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_haptic_runtime_state(
        &mut self,
        motor: &impl MotorOutput,
        system_time_ticks: TimestampTicks,
    ) {
        let config = self.serialized_config;
        let base = self.all_data_payloads.base();
        let ride_state = base.status().ride_state();
        let filtered_current = base.motor().filtered_motor_current().current().current();
        let braking = base.motor().motor_current().current().is_negative();
        let current_limit = if braking {
            self.motor_current_min
        } else {
            self.motor_current_max
        };
        let motor_saturation = if current_limit.current().is_positive() {
            filtered_current.abs().as_amps() / current_limit.current().as_amps()
        } else {
            0.0
        };
        let battery_current = base.motor().battery_current().current();
        let battery_limit = if battery_current.is_negative() {
            self.battery_current_min
        } else {
            self.battery_current_max
        };
        let battery_saturation = if battery_limit.current().is_positive() {
            battery_current.abs().as_amps() / battery_limit.current().as_amps()
        } else {
            0.0
        };
        self.haptic_feedback.update(
            config.haptic(),
            HapticFeedbackInput {
                run_state: ride_state.run_state(),
                mode: ride_state.mode(),
                setpoint_adjustment: ride_state.setpoint_adjustment(),
                duty_cycle: base.motor().duty_cycle().magnitude(),
                duty_solid_threshold: Ratio::clamped(
                    self.runtime_duty_pushback_threshold().as_ratio()
                        + config.haptic().duty_solid_offset().as_ratio(),
                ),
                speed: base.motor().vehicle_speed().speed(),
                current_saturation: Ratio::clamped(motor_saturation.max(battery_saturation)),
                fatal_error: matches!(
                    self.alert_tracker.fatal_error(),
                    crate::domain::FloatOutBoyFatalErrorState::Present
                ),
            },
            motor,
            &mut self.motor_control,
            system_time_ticks,
            config.startup().sample_rate(),
        );
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
            || self.handle_config_command(bytes)
            || self.handle_flywheel_packet(bytes)
            || tuning::handle_runtime_tune_packet(self, bytes)
            || tuning::handle_tilt_tune_packet(self, bytes)
            || tuning::handle_other_tune_packet(self, bytes)
            || tuning::handle_booster_packet(self, bytes)
            || self.handle_rc_move_packet(bytes)
            || self.handle_alert_packet(telemetry, send, bytes)
            || self.send_metadata_packet_response(send, bytes)
            || self.send_legacy_realtime_data_packet_response(send, bytes)
            || self.send_realtime_data_packet_response(telemetry, now, send, bytes)
            || self.send_all_data_packet_response(telemetry, send, bytes)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_motor_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        motor_runtime::refresh(self, telemetry);
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: AdcVoltage, adc2: AdcVoltage) {
        footpad_runtime::refresh(self, adc1, adc2);
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_imu_runtime_state(&mut self, imu: &impl Imu, system_time_ticks: TimestampTicks) {
        imu_runtime::refresh(self, imu, system_time_ticks);
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[cfg_attr(target_arch = "arm", inline(never))]
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
mod flywheel_tests;
#[cfg(test)]
mod footpad_tests;
#[cfg(test)]
mod motor_control_tests;
#[cfg(test)]
mod ready_darkride_tests;
#[cfg(test)]
mod ready_tests;
