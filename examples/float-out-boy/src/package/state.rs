use crate::balance::{BalanceFilter, LoopConfig, LoopInput, LoopState};
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::beeper::{FloatOutBoyBeeper, FloatOutBoyBeeperAlert};
use crate::bms::FloatOutBoyBmsSample;
use crate::config::FloatOutBoyConfigImage;
#[cfg(test)]
use crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID;
use crate::domain::{
    FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand, FloatOutBoyChargingState,
    FloatOutBoyDarkRideState, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRealtimeAtrAccelerationDiff, FloatOutBoyRealtimeAtrSpeedBoost,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterTorque, FloatOutBoyRealtimeControlFrequency,
    FloatOutBoyRealtimeControlPeriod, FloatOutBoyRealtimeLiveValues,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};
use crate::motor_control::FloatOutBoyMotorControl;
use crate::motor_torque::MotorTorqueConstant;
use vescpkg_rs::prelude::OdometerMeters;
use vescpkg_rs::prelude::{AdcVoltage, FirmwareVersion};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCellCount, BatteryVoltage, Current, DutyCycleLimit,
    InputCurrent, MosfetTemperature, MotorCurrent, MotorCurrentLimit, MotorTemperature, Ratio, Rpm,
    TemperatureLimitStart, TimestampTicks,
};
use vescpkg_rs::{
    Imu, ImuPitch, ImuReadSample, ImuRoll, MotorOutput, MotorTelemetry, WrappingTimer,
};

mod alert_tracker;
mod alerts;
#[cfg(test)]
mod balance_tests;
mod bms_runtime;
mod charging;
mod config_runtime;
mod config_storage;
mod data_recorder;
#[cfg(test)]
mod data_recorder_tests;
mod flywheel;
mod footpad_runtime;
mod frequency_tracker;
#[cfg(test)]
mod frequency_tracker_tests;
mod handtest;
mod haptic_feedback;
mod imu_runtime;
mod internal_leds;
mod konami;
mod lcm;
mod limits;
mod motor_kinematics;
mod motor_runtime;
#[cfg(test)]
mod motor_telemetry_tests;
mod packet_response;
mod remote_control;
mod reverse_stop;
#[cfg(test)]
mod reverse_stop_tests;
mod ride_modifiers;
#[cfg(test)]
mod runtime_tests;
mod smooth_setpoint;
mod transition;
#[cfg(test)]
mod transition_tests;
mod tuning;
#[cfg(test)]
mod tuning_tests;

use alert_tracker::AlertTrackerState;
use config_storage::DeferredConfigPersistence;
pub(in crate::package) use config_storage::{
    FirmwareImuMigration, FloatOutBoyConfigLoadOutcome, migrate_legacy_firmware_imu_settings,
    store_persisted_config,
};
pub(in crate::package) use config_storage::{FloatOutBoyPersistedConfig, load_persisted_config};
use data_recorder::DataRecorderState;
use flywheel::FloatOutBoyFlywheelRuntime;
use haptic_feedback::{HapticFeedbackInput, HapticFeedbackState, normalized_current_saturation};
#[cfg(test)]
use internal_leds::FloatOutBoyInternalLedRuntime;
#[cfg(test)]
type InternalLedRuntime = FloatOutBoyInternalLedRuntime;
#[cfg(target_arch = "arm")]
type InternalLedRuntime = internal_leds::RuntimeAllocation;
use konami::FloatOutBoyKonami;
use lcm::LcmState;
use motor_kinematics::MotorKinematicsTracker;
pub(in crate::package) use motor_runtime::{MotorConfigSnapshot, snapshot_motor_config};
use remote_control::RemoteControlState;
use reverse_stop::ReverseStop;
use ride_modifiers::{RideModifierInput, RideModifierState};
use transition::{
    FloatOutBoyStateTransitionInput, FloatOutBoyStopEvent, float_out_boy_first_stop_event,
    float_out_boy_state_transition,
};

// C map: `aux_thd` stores backup data after more than 200 m while not running
// at `third_party/float-out-boy/src/main.c:1142-1146`.
const FLOAT_OUT_BOY_AUX_BACKUP_DISTANCE_METERS: u64 = 200;

#[inline]
#[cfg(test)]
/// C map: `on_command_received` in `third_party/float-out-boy/src/main.c:2143-2225` filters
/// app-data packets by package byte and command ID before dispatching to per-command handlers.
fn float_out_boy_command_payload(
    bytes: &[u8],
    command: FloatOutBoyAppDataCommand,
) -> Option<&[u8]> {
    let (actual, payload) = vescpkg_rs::protocol_app_data::parse_app_data_command::<
        FloatOutBoyAppDataCommand,
    >(bytes, FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID)?;
    (actual == command).then_some(payload)
}

const fn float_out_boy_source_noop(command: FloatOutBoyAppDataCommand) -> bool {
    matches!(
        command,
        FloatOutBoyAppDataCommand::PrintInfo | FloatOutBoyAppDataCommand::Experiment
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KonamiRuntime {
    flywheel: FloatOutBoyKonami,
    headlights_on: FloatOutBoyKonami,
    headlights_off: FloatOutBoyKonami,
}

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
#[pin_init::pin_init]
#[derive(Debug, Default)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq))]
pub struct FloatOutBoyPackageState {
    all_data_payloads: FloatOutBoyAllDataPayloads,
    serialized_config: FloatOutBoyConfigImage,
    config_load_outcome: FloatOutBoyConfigLoadOutcome,
    deferred_config_persistence: DeferredConfigPersistence,
    startup_configured: bool,
    firmware_imu_migration: FirmwareImuMigration,
    data_recorder: DataRecorderState,
    alert_tracker: AlertTrackerState,
    lcm: LcmState,
    led_runtime_overrides: LedRuntimeOverrides,
    konami: KonamiRuntime,
    haptic_feedback: HapticFeedbackState,
    beeper: FloatOutBoyBeeper,
    beeper_flags: BeeperRuntimeFlags,
    bms: bms_runtime::BmsRuntimeState,
    flywheel: FloatOutBoyFlywheelRuntime,
    ride_flags: RideRuntimeFlags,
    motor_control: FloatOutBoyMotorControl,
    balance_filter: BalanceFilter,
    balance_loop: LoopState,
    frequency_trackers: frequency_tracker::FrequencyTrackers,
    reverse_stop: ReverseStop,
    motor_distance_meters: f32,
    #[pin]
    motor_kinematics: MotorKinematicsTracker,
    motor_current_filter: vescpkg_rs::BiquadLowPass,
    motor_torque_constant: MotorTorqueConstant,
    remote_control: RemoteControlState,
    runtime_board_setpoint: vescpkg_rs::prelude::AngleDegrees,
    ride_modifiers: RideModifierState,
    charging_ticks: WrappingTimer,
    engage_ticks: WrappingTimer,
    disengage_ticks: WrappingTimer,
    idle_ticks: WrappingTimer,
    nag_ticks: WrappingTimer,
    idle_voltage: BatteryVoltage,
    fault_switch_ticks: WrappingTimer,
    fault_switch_half_ticks: WrappingTimer,
    fault_angle_pitch_ticks: WrappingTimer,
    fault_angle_roll_ticks: WrappingTimer,
    high_voltage_ticks: WrappingTimer,
    wheelslip_ticks: WrappingTimer,
    upside_down_fault_ticks: WrappingTimer,
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
    motor_config_initialized: bool,
    aux_odometer: OdometerMeters,
    aux_backup_failures: u32,
    aux_motor_config_refresh_ticks: WrappingTimer,
    internal_leds: Option<InternalLedRuntime>,
    internal_led_refresh_pending: bool,
    internal_led_confirmation_pending: Option<TimestampTicks>,
    firmware_version: Option<FirmwareVersion>,
}

impl FloatOutBoyPackageState {
    #[expect(
        clippy::default_trait_access,
        reason = "the in-place initializer infers every field type without duplicating the state declaration"
    )]
    pub(crate) fn default_in_place() -> impl pin_init::Init<Self, core::convert::Infallible> {
        pin_init::init_pin!(FloatOutBoyPackageState {
            all_data_payloads: Default::default(),
            serialized_config: Default::default(),
            config_load_outcome: Default::default(),
            deferred_config_persistence: Default::default(),
            startup_configured: Default::default(),
            firmware_imu_migration: Default::default(),
            data_recorder: Default::default(),
            alert_tracker: Default::default(),
            lcm: Default::default(),
            led_runtime_overrides: Default::default(),
            konami: Default::default(),
            haptic_feedback: Default::default(),
            beeper: Default::default(),
            beeper_flags: Default::default(),
            bms: Default::default(),
            flywheel: Default::default(),
            ride_flags: Default::default(),
            motor_control: Default::default(),
            balance_filter: Default::default(),
            balance_loop: Default::default(),
            frequency_trackers: Default::default(),
            reverse_stop: Default::default(),
            motor_distance_meters: Default::default(),
            motor_kinematics: MotorKinematicsTracker::default_in_place(),
            motor_current_filter: Default::default(),
            motor_torque_constant: Default::default(),
            remote_control: Default::default(),
            runtime_board_setpoint: Default::default(),
            ride_modifiers: Default::default(),
            charging_ticks: Default::default(),
            engage_ticks: Default::default(),
            disengage_ticks: Default::default(),
            idle_ticks: Default::default(),
            nag_ticks: Default::default(),
            idle_voltage: Default::default(),
            fault_switch_ticks: Default::default(),
            fault_switch_half_ticks: Default::default(),
            fault_angle_pitch_ticks: Default::default(),
            fault_angle_roll_ticks: Default::default(),
            high_voltage_ticks: Default::default(),
            wheelslip_ticks: Default::default(),
            upside_down_fault_ticks: Default::default(),
            upside_down_flags: Default::default(),
            motor_duty_raw: Default::default(),
            duty_max_with_margin: Default::default(),
            motor_current_max: Default::default(),
            motor_current_min: Default::default(),
            battery_current_max: Default::default(),
            battery_current_min: Default::default(),
            mosfet_temperature: Default::default(),
            motor_temperature: Default::default(),
            mosfet_temperature_limit_start: Default::default(),
            motor_temperature_limit_start: Default::default(),
            battery_cell_count: Default::default(),
            motor_config_initialized: Default::default(),
            aux_odometer: Default::default(),
            aux_backup_failures: Default::default(),
            aux_motor_config_refresh_ticks: Default::default(),
            internal_leds: Default::default(),
            internal_led_refresh_pending: Default::default(),
            internal_led_confirmation_pending: Default::default(),
            firmware_version: Default::default(),
        })
    }

    fn realtime_live_values(&self) -> FloatOutBoyRealtimeLiveValues {
        FloatOutBoyRealtimeLiveValues::new(
            FloatOutBoyRealtimeControlPeriod::new(self.frequency_trackers.imu.elapsed()),
            FloatOutBoyRealtimeControlFrequency::new(self.frequency_trackers.imu.frequency()),
            self.remote_control.input(),
            FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(
                self.ride_modifiers.atr_accel_diff(),
            ),
            FloatOutBoyRealtimeAtrSpeedBoost::from_units(self.ride_modifiers.atr_speed_boost()),
            self.ride_modifiers.atr_transition_boost(),
        )
    }

    /// Build app-data state from the current all-data payload snapshot.
    #[must_use]
    pub fn new(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let mut state = Self::default();
        state.all_data_payloads = all_data_payloads;
        state.runtime_board_setpoint = state.all_data_payloads.setpoints().board().angle();
        state
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "the capability reference keeps the package input seam explicit"
    )]
    pub(crate) fn refresh_controller_input(
        &mut self,
        input: &vescpkg_rs::FirmwareInputs,
        now: TimestampTicks,
    ) {
        // C map: cutoff `remote_input` gives command input priority for 0.5 s,
        // selects UART/PPM, rejects samples at 0.5 s, then applies physical
        // deadband, move-idle, and tilt-inversion behavior.
        let config = self.serialized_config;
        let input = match config.input_tilt_remote_type() {
            1 => input.remote().ok().and_then(|remote| {
                (remote.age().duration() < vescpkg_rs::VescSeconds::from_seconds(0.5))
                    .then(|| remote.joystick_y().ratio())
            }),
            2 => input.ppm().ok().and_then(|ppm| {
                (ppm.age().duration() < vescpkg_rs::VescSeconds::from_seconds(0.5))
                    .then(|| ppm.value().ratio())
            }),
            _ => None,
        };
        self.remote_control
            .refresh_physical_input(remote_control::PhysicalRemoteInput {
                raw: input,
                now,
                disengage_epoch: self.disengage_ticks.started(),
                deadband: config.input_tilt_deadband(),
                inverted: config.input_tilt_inverted(),
                maximum_move_speed: config.remote().max_move_speed(),
                move_grace: config.remote().grace_period(),
            });
    }

    /// Build startup state and apply the config persisted by firmware.
    ///
    /// Upstream `data_init` reads EEPROM and falls back to generated defaults
    /// at `third_party/float-out-boy/src/main.c:1160-1185`.
    #[cfg(test)]
    pub(super) fn from_persisted_config(all_data_payloads: FloatOutBoyAllDataPayloads) -> Self {
        let mut state = Self::new(all_data_payloads);
        state.load_persisted_config_on_main_thread(vescpkg_rs::FirmwareClock::current_timestamp());
        state.configure_loaded_config_on_main_thread();
        state
    }

    /// Seed the auxiliary backup threshold from the firmware odometer at startup.
    pub(crate) fn initialize_aux_odometer(&mut self, odometer: OdometerMeters) {
        self.aux_odometer = odometer;
    }

    /// Return whether the source-backed auxiliary backup threshold has been crossed.
    pub(crate) fn aux_backup_due(&self, odometer: OdometerMeters) -> bool {
        self.all_data_payloads.ride_state().run_state() != FloatOutBoyRunState::Running
            && odometer.as_meters()
                > self
                    .aux_odometer
                    .as_meters()
                    .saturating_add(FLOAT_OUT_BOY_AUX_BACKUP_DISTANCE_METERS)
    }

    pub(crate) fn record_aux_backup_result(&mut self, odometer: OdometerMeters, stored: bool) {
        if stored {
            self.aux_odometer = odometer;
        } else {
            self.aux_backup_failures = self.aux_backup_failures.saturating_add(1);
        }
    }

    pub(crate) fn aux_motor_config_refresh_due(&self, now: TimestampTicks) -> bool {
        self.aux_motor_config_refresh_ticks
            .older_than(now, vescpkg_rs::VescSeconds::from_seconds(0.5))
    }

    pub(in crate::package) fn finish_aux_motor_config_refresh(
        &mut self,
        config: MotorConfigSnapshot,
        now: TimestampTicks,
    ) {
        motor_runtime::apply_motor_config(self, config);
        self.aux_motor_config_refresh_ticks.restart(now);
    }

    #[cfg(test)]
    pub(crate) const fn aux_backup_failures(&self) -> u32 {
        self.aux_backup_failures
    }

    pub(crate) fn record_firmware_version(&mut self, version: FirmwareVersion) {
        self.firmware_version = Some(version);
    }

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
    pub(crate) fn tick_beeper_at(&mut self, now: TimestampTicks) -> Option<FloatOutBoyBeeperLevel> {
        self.beeper.tick_at(now)
    }

    #[cfg(test)]
    pub(crate) fn tick_beeper(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.beeper.tick()
    }

    pub(crate) fn take_beeper_level(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.beeper.take_level()
    }

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

    #[cfg(test)]
    pub(in crate::package) const fn remote_input_for_test(
        &self,
    ) -> crate::domain::FloatOutBoyRealtimeRemoteInput {
        self.remote_control.input()
    }

    #[cfg(test)]
    pub(in crate::package) fn remote_move_target_for_test(&self) -> Option<vescpkg_rs::Speed> {
        self.remote_control.move_target_for_test()
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    #[cold]
    #[inline(never)]
    fn play_motor_click(&mut self) {
        let startup = self.serialized_config.startup();
        self.motor_control.play_click(
            startup.click_current(),
            self.frequency_trackers.imu.filter_frequency(),
        );
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

    pub(crate) fn update_balance_filter(&mut self, sample: vescpkg_rs::prelude::ImuReadSample) {
        self.frequency_trackers
            .imu
            .update(sample.period().duration());
        self.balance_filter
            .update(sample, Ratio::from_ratio_const(0.1), 0.02);
    }

    pub(crate) fn handle_imu_control_sample(
        &mut self,
        sample: ImuReadSample,
        imu: &impl Imu,
        motor: &impl MotorOutput,
        now: TimestampTicks,
    ) {
        self.update_balance_filter(sample);

        let payloads = self.all_data_payloads;
        let ride_state = payloads.ride_state();
        let (pitch, roll) = self.flywheel_attitude(
            ride_state.mode(),
            AngleDegrees::from(imu.pitch().angle()),
            AngleDegrees::from(imu.roll().angle()),
        );
        let balance_pitch = if ride_state.mode() == FloatOutBoyMode::Flywheel {
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from(pitch))
        } else {
            FloatOutBoyRealtimeBalancePitch::new(self.balance_filter.pitch())
        };

        if ride_state.run_state() == FloatOutBoyRunState::Running {
            let angular_rate = sample.angular_rate();
            let output = self.balance_loop.advance_balance_loop_elapsed(
                self.runtime_balance_loop_config(),
                LoopInput {
                    setpoint: payloads.setpoints().board(),
                    brake_tilt_setpoint: payloads.setpoints().brake_tilt(),
                    balance_pitch: balance_pitch.angle_degrees(),
                    raw_pitch: pitch,
                    roll: ImuRoll::new(AngleRadians::from(roll)),
                    gyro_pitch: angular_rate.pitch(),
                    gyro_yaw: angular_rate.yaw(),
                    motor_erpm: payloads.electrical_speed(),
                    motor_current: payloads.motor_current(),
                    motor_current_max: self.motor_current_max,
                    motor_current_min: self.motor_current_min,
                    mode: ride_state.mode(),
                    darkride: ride_state.darkride(),
                    traction_control: self.ride_flags.traction_control,
                    motor_torque_constant: self.motor_torque_constant,
                },
                sample.period().duration(),
            );
            self.balance_loop = output.state;
            self.request_motor_current(output.requested_current);
        }

        self.all_data_payloads = payloads
            .with_balance_current(FloatOutBoyRealtimeBalanceCurrent::new(
                self.balance_loop.balance_current,
            ))
            .with_balance_pitch(balance_pitch)
            .with_roll(ImuRoll::new(AngleRadians::from(roll)))
            .with_pitch(ImuPitch::new(AngleRadians::from(pitch)))
            .with_booster_torque(FloatOutBoyRealtimeBoosterTorque::new(
                self.balance_loop.booster_torque,
            ));

        self.apply_motor_control(motor, ride_state.run_state(), now);
        #[cfg(any(test, target_arch = "arm"))]
        self.sample_data_recorder(now);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn initialize_frequency_tracking(
        &mut self,
        imu_frequency: vescpkg_rs::prelude::SampleRate,
        now: TimestampTicks,
    ) {
        self.frequency_trackers.main = frequency_tracker::FrequencyTracker::new(
            self.serialized_config.startup().sample_rate(),
            now,
            frequency_tracker::TRACKING_POLICY,
        );
        self.frequency_trackers.imu = frequency_tracker::FrequencyTracker::new(
            frequency_tracker::imu_start_frequency(imu_frequency),
            now,
            frequency_tracker::TRACKING_POLICY,
        );
        self.initialize_data_recorder_sample_rate(imu_frequency);
    }

    pub(crate) fn check_frequency_tracking(&mut self, running: bool, now: TimestampTicks) {
        if let Some(frequency) =
            self.frequency_trackers
                .main
                .check(running, now, frequency_tracker::TRACKING_POLICY)
        {
            motor_runtime::reconfigure_filters(self, frequency);
        }
        if let Some(frequency) =
            self.frequency_trackers
                .imu
                .check(running, now, frequency_tracker::TRACKING_POLICY)
        {
            self.refresh_data_recorder_sample_rate(frequency);
        }
    }

    pub(crate) fn initialize_balance_filter(&mut self, orientation: vescpkg_rs::ImuOrientation) {
        // C map: `data_init` initializes the Float Out Boy filter from VESC's live
        // quaternion through `balance_filter_init` before thread startup at
        // `third_party/float-out-boy/src/main.c:1168-1171` and
        // `third_party/float-out-boy/src/balance_filter.c:53-61`.
        self.balance_filter = BalanceFilter::from_orientation(orientation);
        let filter = self.serialized_config.filter();
        self.balance_filter
            .configure(filter.mahony_kp(), filter.mahony_kp_roll());
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
    pub(crate) const fn lcm_hardware_mode_for_test(&self) -> crate::lcm::FloatOutBoyLedMode {
        self.lcm.hardware_mode()
    }

    pub(super) fn refresh_idle_epoch(&mut self, now: TimestampTicks) {
        self.idle_ticks.restart(now);
    }

    pub(super) fn refresh_running_epochs(&mut self, now: TimestampTicks) {
        self.retry_failed_config_persistence_after_ride();
        self.disengage_ticks.restart(now);
        self.refresh_idle_epoch(now);
    }

    pub(super) fn initialize_time_epochs(&mut self, now: TimestampTicks) {
        // Refloat fixed its 1.2.1 tick/second mismatch in `f727e1d` so the
        // startup disengage epoch is actually one minute old.
        self.engage_ticks.restart(now);
        self.disengage_ticks.expire_whole_seconds(now, 60);
        self.idle_ticks.restart(now);
        self.bms.initialize_start_epoch(now);
    }

    #[cfg(test)]
    pub(super) fn replace_idle_epoch_for_test(&mut self, now: TimestampTicks) {
        self.idle_ticks.restart(now);
    }

    #[cfg(test)]
    pub(super) const fn idle_epoch_for_test(&self) -> TimestampTicks {
        self.idle_ticks.started()
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

    #[cfg(test)]
    /// Parse and handle one raw app-data packet in host tests.
    pub fn handle_packet_with_runtime(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some((command, payload)) = vescpkg_rs::protocol_app_data::parse_app_data_command(
            bytes,
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        ) else {
            return false;
        };
        // The device's dedicated IMU callback already refreshed state.
        let _ = imu;
        self.handle_command_with_telemetry(telemetry, now, reply, command, payload)
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
    #[cfg(test)]
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

    #[cfg(test)]
    pub(crate) fn refresh_main_loop_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        motor: &impl MotorOutput,
        footpad_adc1: AdcVoltage,
        footpad_adc2: AdcVoltage,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let elapsed = self
            .serialized_config
            .startup()
            .sample_rate()
            .sample_period()
            .unwrap_or(vescpkg_rs::prelude::VescSeconds::ZERO);
        self.refresh_main_loop_runtime_state_elapsed(
            telemetry,
            imu,
            motor,
            (footpad_adc1, footpad_adc2),
            system_time_ticks,
            elapsed,
        )
    }

    pub(crate) fn refresh_main_loop_runtime_state_elapsed(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        motor: &impl MotorOutput,
        footpads: (AdcVoltage, AdcVoltage),
        system_time_ticks: TimestampTicks,
        elapsed: vescpkg_rs::prelude::VescSeconds,
    ) -> bool {
        self.frequency_trackers.main.update(elapsed);
        // Keep the ARM refresh phases in separate frames so LTO cannot merge
        // their independent stack use inside VESC's fixed thread working area.
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state_elapsed(telemetry, elapsed);
        self.refresh_haptic_runtime_state(motor, system_time_ticks);
        self.alert_tracker.update(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        self.refresh_footpad_runtime_state(footpads.0, footpads.1);
        let restore_flywheel_config =
            self.refresh_konami_runtime_state(imu.pitch(), system_time_ticks);
        self.refresh_charging_runtime_state(system_time_ticks);
        self.refresh_bms_runtime_state(system_time_ticks);
        self.refresh_imu_runtime_state_elapsed(imu, system_time_ticks, elapsed)
            || restore_flywheel_config
    }

    fn refresh_konami_runtime_state(
        &mut self,
        current_pitch: ImuPitch,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let payloads = self.all_data_payloads;
        let ride_state = payloads.ride_state();
        // C refreshes `d->imu.pitch` before entering the READY Konami branch at
        // `third_party/float-out-boy/src/main.c:775,947-953`.
        let footpad = payloads.footpad().state();

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
            self.prepare_flywheel_command(&[0x82, 0, 0, 0, 0, 1])
                .unwrap_or(false)
        } else {
            false
        };

        if self.serialized_config.hardware_led_mode() == crate::lcm::FloatOutBoyLedMode::Off {
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

    fn handle_remote_command(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        payload: &[u8],
    ) -> bool {
        self.remote_control.handle_command(
            now(),
            self.disengage_ticks.started(),
            self.serialized_config.remote().max_move_speed(),
            payload,
        )
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_haptic_runtime_state(
        &mut self,
        motor: &impl MotorOutput,
        system_time_ticks: TimestampTicks,
    ) {
        let config = self.serialized_config;
        let payloads = self.all_data_payloads;
        let ride_state = payloads.ride_state();
        let filtered_current = payloads.filtered_motor_current().current().current();
        let braking = payloads.motor_current().is_negative();
        let current_limit = if braking {
            self.motor_current_min
        } else {
            self.motor_current_max
        };
        let motor_saturation =
            normalized_current_saturation(filtered_current, current_limit.current());
        let battery_current = payloads.battery_current().current();
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
                duty_cycle: payloads.duty_cycle().magnitude(),
                duty_solid_threshold: Ratio::clamped(
                    self.runtime_duty_pushback_threshold().as_ratio()
                        + config.haptic().duty_solid_offset().as_ratio(),
                ),
                speed: payloads.vehicle_speed().speed(),
                current_saturation: Ratio::clamped(motor_saturation.max(battery_saturation)),
                fatal_error: self.alert_tracker.fatal_error(),
            },
            motor,
            &mut self.motor_control,
            system_time_ticks,
            self.frequency_trackers.imu.filter_frequency(),
        );
    }

    /// Handle one decoded app-data command after refreshing live telemetry fields.
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub fn handle_command_with_telemetry(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: FloatOutBoyAppDataCommand,
        payload: &[u8],
    ) -> bool {
        #[cfg(test)]
        if let Some(handled) = self.handle_effectful_packet_for_test(now, command, payload) {
            return handled;
        }
        if self.handle_control_command(now, command, payload)
            || self.handle_config_command_boundary(now, command)
        {
            return true;
        }
        #[cfg(test)]
        if self.handle_tuning_command(now, command, payload) {
            return true;
        }
        self.handle_query_command(telemetry, now, reply, command, payload)
            || self.reply_to_all_data_command(telemetry, reply, command, payload)
    }

    #[cfg(test)]
    /// Parse and handle one raw app-data packet in telemetry-only host tests.
    pub fn handle_packet_with_telemetry(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some((command, payload)) = vescpkg_rs::protocol_app_data::parse_app_data_command(
            bytes,
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        ) else {
            return false;
        };
        self.handle_command_with_telemetry(telemetry, now, reply, command, payload)
    }

    #[cfg(test)]
    fn handle_effectful_packet_for_test(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        command: FloatOutBoyAppDataCommand,
        payload: &[u8],
    ) -> Option<bool> {
        match command {
            FloatOutBoyAppDataCommand::ConfigSave => {
                let requested_at = now();
                if let Some(config) = self.begin_active_config_persistence(requested_at) {
                    let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
                        store_persisted_config(effects, &config)
                    });
                    self.finish_config_persistence(&config, stored, now());
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
                let Some(restore) = self.prepare_handtest_command(payload) else {
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
                let Some(restore) = self.prepare_flywheel_command(payload) else {
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
    fn handle_control_command(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        command: FloatOutBoyAppDataCommand,
        payload: &[u8],
    ) -> bool {
        float_out_boy_source_noop(command)
            || command == FloatOutBoyAppDataCommand::ChargingState
                && self.handle_charging_state_command(now, payload)
            || command == FloatOutBoyAppDataCommand::Remote
                && self.handle_remote_command(now, payload)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_config_command_boundary(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        command: FloatOutBoyAppDataCommand,
    ) -> bool {
        self.handle_config_command(command, now)
    }

    #[cfg(test)]
    fn handle_tuning_command(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        command: FloatOutBoyAppDataCommand,
        payload: &[u8],
    ) -> bool {
        let mut config = *self.serialized_config();
        let Some(commit) = Self::prepare_tune_config(&mut config, command, payload) else {
            return false;
        };
        self.commit_prepared_tune(&config, commit, now());
        true
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn handle_query_command(
        &mut self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: FloatOutBoyAppDataCommand,
        payload: &[u8],
    ) -> bool {
        self.handle_alert_packet(telemetry, reply, command, payload)
            || self.handle_lcm_command(telemetry, reply, command, payload)
            || command == FloatOutBoyAppDataCommand::DataRecordRequest
                && self.handle_data_recorder_packet(reply, payload)
            || self.reply_to_metadata_command(reply, command, payload)
            || self.reply_to_legacy_realtime_data_command(reply, command)
            || self.reply_to_realtime_data_command(telemetry, now, reply, command)
            || self.reply_to_realtime_selected_command(telemetry, now, reply, command, payload)
    }

    #[cfg(test)]
    pub(crate) fn refresh_motor_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        let elapsed = self
            .serialized_config
            .startup()
            .sample_rate()
            .sample_period()
            .unwrap_or(vescpkg_rs::prelude::VescSeconds::ZERO);
        self.refresh_motor_runtime_state_elapsed(telemetry, elapsed);
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_motor_runtime_state_elapsed(
        &mut self,
        telemetry: &impl MotorTelemetry,
        elapsed: vescpkg_rs::prelude::VescSeconds,
    ) {
        #[cfg(test)]
        {
            if !self.motor_config_initialized {
                self.refresh_motor_config_runtime_state(telemetry);
            }
        }
        motor_runtime::refresh(self, telemetry, elapsed);
    }

    pub(crate) fn refresh_motor_config_runtime_state(&mut self, telemetry: &impl MotorTelemetry) {
        motor_runtime::refresh_config(self, telemetry);
        #[cfg(test)]
        {
            self.motor_config_initialized = true;
        }
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: AdcVoltage, adc2: AdcVoltage) {
        footpad_runtime::refresh(self, adc1, adc2);
    }

    #[cfg(test)]
    fn refresh_imu_runtime_state(
        &mut self,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) -> bool {
        let elapsed = self
            .serialized_config
            .startup()
            .sample_rate()
            .sample_period()
            .unwrap_or(vescpkg_rs::prelude::VescSeconds::ZERO);
        self.refresh_imu_runtime_state_elapsed(imu, system_time_ticks, elapsed)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_imu_runtime_state_elapsed(
        &mut self,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
        elapsed: vescpkg_rs::prelude::VescSeconds,
    ) -> bool {
        imu_runtime::refresh(self, imu, system_time_ticks, elapsed)
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn refresh_charging_runtime_state(&mut self, system_time_ticks: TimestampTicks) {
        self.all_data_payloads = charging::timeout(
            self.all_data_payloads,
            system_time_ticks,
            self.charging_ticks,
        );
    }

    fn handle_charging_state_command(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        payload: &[u8],
    ) -> bool {
        match charging::handle_command(self.all_data_payloads, payload) {
            Some(payloads) => {
                self.all_data_payloads = payloads;
                self.charging_ticks.restart(now());
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
