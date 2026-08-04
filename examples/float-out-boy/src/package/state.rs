#[cfg(any(test, target_arch = "arm"))]
use super::time::float_out_boy_expire_timer;
use super::time::{float_out_boy_ticks_elapsed, float_out_boy_ticks_elapsed_seconds};
use crate::balance::{BalanceFilter, LoopConfig, LoopInput, LoopState};
#[cfg(any(test, target_arch = "arm"))]
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::beeper::{FloatOutBoyBeeper, FloatOutBoyBeeperAlert};
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::FloatOutBoyBmsSample;
use crate::config::FloatOutBoyConfigImage;
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
use vescpkg_rs::ImuPitch;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::OdometerMeters;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{AdcVoltage, FirmwareVersion};
use vescpkg_rs::prelude::{
    AngleRadians, BatteryCellCount, BatteryVoltage, Current, DutyCycleLimit, InputCurrent,
    MosfetTemperature, MotorCurrent, MotorCurrentLimit, MotorTemperature, Ratio, Rpm,
    TemperatureLimitStart, TimestampTicks,
};
use vescpkg_rs::{Imu, MotorOutput, MotorTelemetry};

mod alert_tracker;
mod alerts;
#[cfg(test)]
mod balance_tests;
#[cfg(any(test, target_arch = "arm"))]
mod bms_runtime;
mod charging;
mod config_runtime;
mod config_storage;
mod data_recorder;
#[cfg(test)]
mod data_recorder_tests;
mod flywheel;
#[cfg(any(test, target_arch = "arm"))]
mod footpad_runtime;
mod handtest;
#[cfg(any(test, target_arch = "arm"))]
mod haptic_feedback;
mod imu_runtime;
#[cfg(any(test, target_arch = "arm"))]
mod internal_leds;
#[cfg(any(test, target_arch = "arm"))]
mod konami;
mod lcm;
mod limits;
mod motor_kinematics;
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
pub(in crate::package) use config_storage::{
    FirmwareImuMigration, FloatOutBoyConfigLoadOutcome, migrate_legacy_firmware_imu_settings,
    store_persisted_config,
};
#[cfg(any(test, target_arch = "arm"))]
pub(in crate::package) use config_storage::{FloatOutBoyPersistedConfig, load_persisted_config};
use data_recorder::DataRecorderState;
use flywheel::FloatOutBoyFlywheelRuntime;
#[cfg(any(test, target_arch = "arm"))]
use haptic_feedback::{HapticFeedbackInput, HapticFeedbackState, normalized_current_saturation};
#[cfg(test)]
use internal_leds::FloatOutBoyInternalLedRuntime;
#[cfg(any(test, target_arch = "arm"))]
use konami::FloatOutBoyKonami;
use lcm::LcmState;
use motor_kinematics::MotorKinematicsTracker;
use remote_control::RemoteControlState;
use ride_modifiers::{RideModifierInput, RideModifierState};
use transition::{
    FloatOutBoyStateTransitionInput, FloatOutBoyStopEvent, float_out_boy_first_stop_event,
    float_out_boy_state_transition,
};

// C map: `aux_thd` stores backup data after more than 200 m while not running
// at `third_party/float-out-boy/src/main.c:1142-1146`.
#[cfg(any(test, target_arch = "arm"))]
const FLOAT_OUT_BOY_AUX_BACKUP_DISTANCE_METERS: u64 = 200;

#[inline]
/// C map: `on_command_received` in `third_party/float-out-boy/src/main.c:2143-2225` filters
/// app-data packets by package byte and command ID before dispatching to per-command handlers.
fn float_out_boy_command_payload(
    bytes: &[u8],
    command: FloatOutBoyAppDataCommand,
) -> Option<&[u8]> {
    match bytes {
        [package_id, command_id, payload @ ..]
            if *package_id == FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID && *command_id == command.id() =>
        {
            Some(payload)
        }
        _ => None,
    }
}

fn float_out_boy_source_noop(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        [package_id, command_id, ..]
            if *package_id == FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID
                && (*command_id == FloatOutBoyAppDataCommand::PrintInfo.id()
                    || *command_id == FloatOutBoyAppDataCommand::Experiment.id())
    )
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct BeeperRuntimeFlags {
    pin_configured: bool,
    duty_warning_active: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct RideRuntimeFlags {
    traction_control: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LedRuntimeOverrides {
    enabled: Option<bool>,
    headlights_enabled: Option<bool>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct UpsideDownRuntimeFlags {
    enabled: bool,
    started: bool,
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KonamiRuntime {
    flywheel: FloatOutBoyKonami,
    headlights_on: FloatOutBoyKonami,
    headlights_off: FloatOutBoyKonami,
}

#[cfg(any(test, target_arch = "arm"))]
impl Default for KonamiRuntime {
    fn default() -> Self {
        Self {
            flywheel: FloatOutBoyKonami::flywheel(),
            headlights_on: FloatOutBoyKonami::headlights_on(),
            headlights_off: FloatOutBoyKonami::headlights_off(),
        }
    }
}

/// Float Out Boy package state.
#[derive(Debug, Default)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq))]
pub struct FloatOutBoyPackageState {
    all_data_payloads: FloatOutBoyAllDataPayloads,
    serialized_config: FloatOutBoyConfigImage,
    config_load_outcome: FloatOutBoyConfigLoadOutcome,
    startup_configured: bool,
    firmware_imu_migration: FirmwareImuMigration,
    data_recorder: DataRecorderState,
    alert_tracker: AlertTrackerState,
    lcm: LcmState,
    led_runtime_overrides: LedRuntimeOverrides,
    #[cfg(any(test, target_arch = "arm"))]
    konami: KonamiRuntime,
    #[cfg(any(test, target_arch = "arm"))]
    haptic_feedback: HapticFeedbackState,
    beeper: FloatOutBoyBeeper,
    beeper_flags: BeeperRuntimeFlags,
    #[cfg(any(test, target_arch = "arm"))]
    bms: bms_runtime::BmsRuntimeState,
    flywheel: FloatOutBoyFlywheelRuntime,
    ride_flags: RideRuntimeFlags,
    motor_control: FloatOutBoyMotorControl,
    balance_filter: BalanceFilter,
    balance_loop: LoopState,
    reverse_total_erpm: Rpm,
    motor_kinematics: MotorKinematicsTracker,
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
    upside_down_flags: UpsideDownRuntimeFlags,
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
    #[cfg(test)]
    motor_config_initialized: bool,
    #[cfg(any(test, target_arch = "arm"))]
    aux_odometer: OdometerMeters,
    #[cfg(any(test, target_arch = "arm"))]
    aux_backup_failures: u32,
    #[cfg(any(test, target_arch = "arm"))]
    aux_motor_config_refresh_ticks: TimestampTicks,
    #[cfg(test)]
    internal_leds: Option<FloatOutBoyInternalLedRuntime>,
    #[cfg(target_arch = "arm")]
    internal_leds: Option<internal_leds::RuntimeAllocation>,
    #[cfg(any(test, target_arch = "arm"))]
    internal_led_refresh_pending: bool,
    #[cfg(any(test, target_arch = "arm"))]
    internal_led_confirmation_pending: Option<TimestampTicks>,
    #[cfg(any(test, target_arch = "arm"))]
    firmware_version: Option<FirmwareVersion>,
}

impl FloatOutBoyPackageState {
    /// Build app-data state from the current all-data payload snapshot.
    #[must_use]
    pub fn new(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let mut state = Self::default();
        state.all_data_payloads = all_data_payloads;
        state.runtime_board_setpoint = state.all_data_payloads.base().setpoints().board().angle();
        state
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "the capability reference keeps the package input seam explicit"
    )]
    pub(crate) fn refresh_controller_input(&mut self, input: &vescpkg_rs::FirmwareInputs) {
        // C map: Float Out Boy selects UART/PPM, rejects samples one second old,
        // applies deadband rescaling, then optional inversion at
        // `third_party/float-out-boy/src/remote.c:36-68`.
        let config = self.serialized_config;
        let value = match config.input_tilt_remote_type() {
            1 => input.remote().ok().and_then(|remote| {
                (remote.age().duration() < vescpkg_rs::VescSeconds::from_seconds(1.0))
                    .then(|| remote.joystick_y().ratio().as_ratio())
            }),
            2 => input.ppm().ok().and_then(|ppm| {
                (ppm.age().duration() < vescpkg_rs::VescSeconds::from_seconds(1.0))
                    .then(|| ppm.value().ratio().as_ratio())
            }),
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
    #[cfg(test)]
    pub(super) fn from_persisted_config(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let mut state = Self::new(all_data_payloads);
        state.load_persisted_config_on_main_thread(vescpkg_rs::FirmwareClock::current_timestamp());
        state.configure_loaded_config_on_main_thread();
        state
    }

    /// Seed the auxiliary backup threshold from the firmware odometer at startup.
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn initialize_aux_odometer(&mut self, odometer: OdometerMeters) {
        self.aux_odometer = odometer;
    }

    /// Return whether the source-backed auxiliary backup threshold has been crossed.
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn aux_backup_due(&self, odometer: OdometerMeters) -> bool {
        self.all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state()
            != FloatOutBoyRunState::Running
            && odometer.as_meters()
                > self
                    .aux_odometer
                    .as_meters()
                    .saturating_add(FLOAT_OUT_BOY_AUX_BACKUP_DISTANCE_METERS)
    }

    /// Record a successful auxiliary backup so the same distance is not stored repeatedly.
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_aux_backup(&mut self, odometer: OdometerMeters) {
        self.aux_odometer = odometer;
    }

    /// Record an unsuccessful auxiliary backup for diagnostics and retry on the next tick.
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_aux_backup_failure(&mut self) {
        self.aux_backup_failures = self.aux_backup_failures.saturating_add(1);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_aux_motor_config_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: TimestampTicks,
    ) {
        if float_out_boy_ticks_elapsed_seconds(
            now,
            self.aux_motor_config_refresh_ticks,
            vescpkg_rs::VescSeconds::from_seconds(0.5),
        ) {
            self.refresh_motor_config_runtime_state(telemetry);
            self.aux_motor_config_refresh_ticks = now;
        }
    }

    #[cfg(test)]
    pub(crate) const fn aux_backup_failures(&self) -> u32 {
        self.aux_backup_failures
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_firmware_version(&mut self, version: FirmwareVersion) {
        self.firmware_version = Some(version);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn record_bms_sample(&mut self, sample: FloatOutBoyBmsSample) {
        self.bms.record_sample(sample);
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
        let uses_ppm_input = self.serialized_config.input_tilt_remote_type() == 2;
        // Refloat checks this only during startup, so enabling the beeper later
        // can write through a PPM pin that is still configured as an input.
        // Acquire it on the first configuration that actually needs the output.
        if self.beeper_flags.pin_configured
            || (!self.serialized_config.beeper_enabled() && uses_ppm_input)
        {
            return false;
        }
        self.beeper_flags.pin_configured = true;
        true
    }

    #[cfg(any(test, target_arch = "arm"))]
    /// Recompute the BMS fault mask before control-loop state selection.
    ///
    /// C map: `bms_update` runs immediately before state logic at
    /// `third_party/float-out-boy/src/main.c:824-831`.
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_bms_runtime_state(&mut self, system_time_ticks: TimestampTicks) {
        let bms = self.serialized_config.bms();
        self.bms
            .refresh(bms.enabled(), bms.thresholds(), system_time_ticks);
    }

    #[cfg(test)]
    pub(crate) const fn bms_sample_for_test(&self) -> FloatOutBoyBmsSample {
        self.bms.sample()
    }

    #[cfg(test)]
    pub(crate) const fn bms_faults_for_test(&self) -> crate::bms::FloatOutBoyBmsFaults {
        self.bms.faults()
    }

    #[cfg(test)]
    pub(crate) const fn recorded_firmware_version(&self) -> Option<FirmwareVersion> {
        self.firmware_version
    }

    /// Return the current all-data payload snapshot.
    #[must_use]
    pub const fn all_data_payloads(&self) -> FloatOutBoyAllDataPayloads {
        self.all_data_payloads
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    #[cold]
    #[inline(never)]
    fn play_motor_click(&mut self) {
        let startup = self.serialized_config.startup();
        self.motor_control
            .play_click(startup.click_current(), startup.sample_rate());
    }

    /// Apply and clear a pending motor-current request.
    #[cfg(test)]
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
        // Upstream `motor_control_configure` copies brake and parking config at
        // `third_party/float-out-boy/src/motor_control.c:36-40`; this Rust state keeps
        // the serialized config as source of truth until full `Data` parity.
        self.motor_control.apply(
            motor,
            run_state,
            self.motor_kinematics.smoothed_abs_erpm(),
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

    #[cfg(test)]
    pub(crate) const fn configured_mahony_gains_for_test(
        &self,
    ) -> (vescpkg_rs::MahonyPitchGain, vescpkg_rs::MahonyRollGain) {
        self.balance_filter.configured_gains()
    }

    #[cfg(test)]
    pub(crate) const fn lcm_hardware_mode_for_test(&self) -> u8 {
        self.lcm.hardware_mode()
    }

    pub(super) fn refresh_idle_epoch(&mut self, now: TimestampTicks) {
        self.idle_ticks = now;
    }

    pub(super) fn refresh_running_epochs(&mut self, now: TimestampTicks) {
        self.disengage_ticks = now;
        self.refresh_idle_epoch(now);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn initialize_time_epochs(&mut self, now: TimestampTicks) {
        // Refloat fixed its 1.2.1 tick/second mismatch in `f727e1d` so the
        // startup disengage epoch is actually one minute old.
        self.engage_ticks = now;
        self.disengage_ticks = float_out_boy_expire_timer(now, 60);
        self.idle_ticks = now;
        self.bms.initialize_start_epoch(now);
    }

    #[cfg(test)]
    pub(super) fn replace_idle_epoch_for_test(&mut self, now: TimestampTicks) {
        self.idle_ticks = now;
    }

    #[cfg(test)]
    pub(super) const fn idle_epoch_for_test(&self) -> TimestampTicks {
        self.idle_ticks
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_config_runtime_state(&mut self) {
        config_runtime::refresh(self);
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_led_config_runtime_state(&mut self) {
        config_runtime::refresh_leds(self);
    }

    fn led_runtime_status(&self) -> crate::leds::FloatOutBoyLedRuntimeStatus {
        crate::leds::FloatOutBoyLedRuntimeStatus {
            enabled: self
                .led_runtime_overrides
                .enabled
                .unwrap_or_else(|| self.serialized_config.leds_enabled()),
            headlights_enabled: self
                .led_runtime_overrides
                .headlights_enabled
                .unwrap_or_else(|| self.serialized_config.headlights_enabled()),
        }
    }

    fn effective_led_config(
        &self,
    ) -> Option<(
        crate::lcm::FloatOutBoyHardwareLedsConfig,
        crate::leds::FloatOutBoyLedsConfig,
    )> {
        self.serialized_config
            .led_configs()
            .map(|(hardware, mut config)| {
                let status = self.led_runtime_status();
                config.on = status.enabled;
                config.headlights_on = status.headlights_enabled;
                (hardware, config)
            })
    }

    fn set_led_runtime_overrides(
        &mut self,
        enabled: Option<bool>,
        headlights_enabled: Option<bool>,
    ) {
        if let Some(enabled) = enabled {
            self.led_runtime_overrides.enabled = Some(enabled);
        }
        if let Some(headlights_enabled) = headlights_enabled {
            self.led_runtime_overrides.headlights_enabled = Some(headlights_enabled);
        }
        config_runtime::refresh_led_effects(self);
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
        imu: &impl Imu,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        // Device callbacks keep the IMU parameter for one stable packet API;
        // the device's dedicated IMU callback already refreshed state.
        let _ = imu;
        #[cfg(all(not(test), not(target_arch = "arm")))]
        self.refresh_runtime_state(telemetry, imu, now());

        self.handle_packet_with_telemetry(telemetry, now, reply, bytes)
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
    #[cfg(not(target_arch = "arm"))]
    pub(crate) fn refresh_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.alert_tracker.update(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        let _ = self.refresh_imu_runtime_state(imu, system_time_ticks);
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
    ) -> bool {
        // Keep the ARM refresh phases in separate frames so LTO cannot merge
        // their independent stack use inside VESC's fixed thread working area.
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_haptic_runtime_state(motor, system_time_ticks);
        self.alert_tracker.update(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        self.refresh_footpad_runtime_state(footpad_adc1, footpad_adc2);
        let restore_flywheel_config =
            self.refresh_konami_runtime_state(imu.pitch(), system_time_ticks);
        self.refresh_charging_runtime_state(system_time_ticks);
        self.refresh_bms_runtime_state(system_time_ticks);
        self.refresh_imu_runtime_state(imu, system_time_ticks) || restore_flywheel_config
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn refresh_konami_runtime_state(
        &mut self,
        current_pitch: ImuPitch,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let base = self.all_data_payloads.base();
        let ride_state = base.status().ride_state();
        // C refreshes `d->imu.pitch` before entering the READY Konami branch at
        // `third_party/float-out-boy/src/main.c:775,947-953`.
        let footpad = base.footpad().state();

        let restore_flywheel_config = if ride_state.run_state() == FloatOutBoyRunState::Ready
            && ride_state.mode() != FloatOutBoyMode::Flywheel
            && self
                .konami
                .flywheel
                .check_flywheel(current_pitch, footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            // C map: `main.c:85-89` and `main.c:945-949`; this is the same
            // armed default flywheel command used by the native handler.
            let command = [
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
                FloatOutBoyAppDataCommand::Flywheel.id(),
                0x82,
                0,
                0,
                0,
                0,
                1,
            ];
            self.prepare_flywheel_packet(&command).unwrap_or(false)
        } else {
            false
        };

        if self.serialized_config.hardware_led_mode_id() == 0 {
            return restore_flywheel_config;
        }
        let status = self.led_runtime_status();
        if !status.headlights_enabled && self.konami.headlights_on.check(footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            self.set_led_runtime_overrides(None, Some(true));
        }
        if status.headlights_enabled && self.konami.headlights_off.check(footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            self.set_led_runtime_overrides(None, Some(false));
        }
        restore_flywheel_config
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
        let braking = base.motor().motor_current().is_negative();
        let current_limit = if braking {
            self.motor_current_min
        } else {
            self.motor_current_max
        };
        let motor_saturation =
            normalized_current_saturation(filtered_current, current_limit.current());
        let battery_current = base.motor().battery_current().current();
        let battery_limit = if battery_current.is_negative() {
            self.battery_current_min
        } else {
            self.battery_current_max
        };
        let battery_saturation =
            normalized_current_saturation(battery_current, battery_limit.current());
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
                fatal_error: self.alert_tracker.fatal_error(),
            },
            motor,
            &mut self.motor_control,
            system_time_ticks,
            config.startup().sample_rate(),
        );
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub fn handle_packet_with_telemetry(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        #[cfg(test)]
        if let Some(handled) = self.handle_effectful_packet_for_test(now, bytes) {
            return handled;
        }
        self.handle_control_packet(now, bytes)
            || self.handle_config_packet(now, bytes)
            || self.handle_tuning_packet(now, bytes)
            || self.handle_query_packet(telemetry, now, reply, bytes)
            || self.reply_to_all_data_packet(telemetry, reply, bytes)
    }

    #[cfg(test)]
    fn handle_effectful_packet_for_test(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> Option<bool> {
        let [package_id, command, payload @ ..] = bytes else {
            return None;
        };
        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID {
            return None;
        }
        let Ok(command) = FloatOutBoyAppDataCommand::try_from_id(*command) else {
            return None;
        };

        match command {
            FloatOutBoyAppDataCommand::ConfigSave => {
                let config = self.active_config_image();
                let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
                    store_persisted_config(effects, &config)
                });
                if stored {
                    self.acknowledge_command_config_write(now());
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::ConfigRestore => {
                let loaded = vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                self.begin_restore_persisted_config(&loaded, now());
                let migration = vescpkg_rs::test_support::with_firmware_effects(
                    migrate_legacy_firmware_imu_settings,
                );
                self.finish_configure_active(migration);
                Some(true)
            }
            FloatOutBoyAppDataCommand::Lock => {
                let Some(disabled) = payload.first() else {
                    return Some(false);
                };
                if !self.is_running() {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    if let Some(config) =
                        self.apply_lock_from_persisted(&loaded, *disabled != 0, now())
                    {
                        let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
                            store_persisted_config(effects, &config)
                        });
                        if stored {
                            self.acknowledge_command_config_write(now());
                        }
                        let migration = vescpkg_rs::test_support::with_firmware_effects(
                            migrate_legacy_firmware_imu_settings,
                        );
                        self.finish_configure_active(migration);
                    }
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::HandTest => {
                let Some(restore) = self.prepare_handtest_packet(bytes) else {
                    return Some(false);
                };
                if restore {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    if self.commit_handtest_restore(
                        &loaded,
                        vescpkg_rs::FirmwareClock::current_timestamp(),
                    ) {
                        let migration = vescpkg_rs::test_support::with_firmware_effects(
                            migrate_legacy_firmware_imu_settings,
                        );
                        self.finish_configure_active(migration);
                    }
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::Flywheel => {
                let Some(restore) = self.prepare_flywheel_packet(bytes) else {
                    return Some(false);
                };
                if restore {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    self.commit_flywheel_restore(
                        &loaded,
                        vescpkg_rs::FirmwareClock::current_timestamp(),
                    );
                    let migration = vescpkg_rs::test_support::with_firmware_effects(
                        migrate_legacy_firmware_imu_settings,
                    );
                    self.finish_configure_active(migration);
                }
                Some(true)
            }
            _ => None,
        }
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_control_packet(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> bool {
        float_out_boy_source_noop(bytes)
            || self.handle_charging_state_packet(now, bytes)
            || self.handle_rc_move_packet(bytes)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_config_packet(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> bool {
        self.handle_config_command(bytes, now)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_tuning_packet(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> bool {
        tuning::handle_runtime_tune_packet(self, now, bytes)
            || tuning::handle_tilt_tune_packet(self, bytes)
            || tuning::handle_other_tune_packet(self, now, bytes)
            || tuning::handle_booster_packet(self, bytes)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_query_packet(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        self.handle_alert_packet(telemetry, reply, bytes)
            || self.handle_lcm_packet(telemetry, reply, bytes)
            || self.handle_data_recorder_packet(reply, bytes)
            || self.reply_to_metadata_packet(reply, bytes)
            || self.reply_to_legacy_realtime_data_packet(reply, bytes)
            || self.reply_to_realtime_data_packet(telemetry, now, reply, bytes)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_motor_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        #[cfg(test)]
        {
            if !self.motor_config_initialized {
                self.refresh_motor_config_runtime_state(telemetry);
            }
        }
        motor_runtime::refresh(self, telemetry);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_motor_config_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        motor_runtime::refresh_config(self, telemetry);
        #[cfg(test)]
        {
            self.motor_config_initialized = true;
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: AdcVoltage, adc2: AdcVoltage) {
        footpad_runtime::refresh(self, adc1, adc2);
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_imu_runtime_state(
        &mut self,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        imu_runtime::refresh(self, imu, system_time_ticks)
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
