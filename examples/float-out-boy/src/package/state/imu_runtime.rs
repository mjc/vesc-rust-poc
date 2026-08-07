#[cfg(any(test, target_arch = "arm"))]
use super::BatteryVoltage;
use super::limits::{
    DarkrideLimits, MovingFaultLimits, PushStartLimits, QuickStopLimits, RemoteSetpointFaultLimit,
    ReverseStopLimits, TractionLossLimits,
};
use super::transition::FloatOutBoyStateTransitionOutput;
use super::{
    AngleRadians, BatteryCellCount, Current, FloatOutBoyAllDataAttitude,
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataStatus, FloatOutBoyBeeperAlert,
    FloatOutBoyBeeperCount, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyEngagementDecision, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyPackageState, FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStateTransitionInput, FloatOutBoyStopCondition, FloatOutBoyStopEvent,
    FloatOutBoyTransitionEffect, FloatOutBoyWheelSlipState, Imu, LoopInput, MotorCurrent,
    RideModifierInput, Rpm, TimestampTicks, float_out_boy_state_transition,
    float_out_boy_ticks_elapsed, float_out_boy_ticks_elapsed_seconds,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::FloatOutBoyBmsFault;
use crate::domain::{FloatOutBoyAllDataMotorPayload, FloatOutBoyBeepReason, FloatOutBoyRideState};
use vescpkg_rs::prelude::{
    AngleDegrees, DutyCycle, SignedRatio, SystemTicks, Temperature, VescSeconds, Voltage,
};
use vescpkg_rs::{ImuPitch, ImuRoll};

fn rate_limit_angle(
    current: AngleDegrees,
    target: AngleDegrees,
    step: AngleDegrees,
) -> AngleDegrees {
    let difference = target - current;
    if difference.abs() < step {
        target
    } else if difference > AngleDegrees::ZERO {
        current + step
    } else {
        current - step
    }
}

fn pack_voltage_threshold(
    configured: Voltage,
    battery_cell_count: Option<BatteryCellCount>,
) -> Voltage {
    if configured.as_volts() < 10.0 {
        battery_cell_count.map_or(configured, |count| configured * count)
    } else {
        configured
    }
}

pub(super) fn startup_ready_beep_count(
    warning_threshold: Voltage,
    battery_voltage: Voltage,
) -> FloatOutBoyBeeperCount {
    if battery_voltage + Voltage::from_volts(6.0) <= warning_threshold {
        FloatOutBoyBeeperCount::SEVEN
    } else if battery_voltage + Voltage::from_volts(5.0) <= warning_threshold {
        FloatOutBoyBeeperCount::SIX
    } else if battery_voltage + Voltage::from_volts(4.0) <= warning_threshold {
        FloatOutBoyBeeperCount::FIVE
    } else if battery_voltage + Voltage::from_volts(3.0) <= warning_threshold {
        FloatOutBoyBeeperCount::FOUR
    } else if battery_voltage + Voltage::from_volts(2.0) <= warning_threshold {
        FloatOutBoyBeeperCount::THREE
    } else if battery_voltage + Voltage::from_volts(1.0) <= warning_threshold {
        FloatOutBoyBeeperCount::TWO
    } else {
        FloatOutBoyBeeperCount::ONE
    }
}

struct RefreshStart {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    startup_became_ready: bool,
}

fn begin_refresh(
    state: &mut FloatOutBoyPackageState,
    base: FloatOutBoyAllDataBasePayload,
    imu_ready: bool,
    system_time_ticks: TimestampTicks,
) -> RefreshStart {
    let status = base.status();
    let ride_state = status.ride_state();
    let startup_became_ready =
        matches!(ride_state.run_state(), FloatOutBoyRunState::Startup) && imu_ready;
    let run_state = if startup_became_ready {
        FloatOutBoyRunState::Ready
    } else {
        ride_state.run_state()
    };
    if matches!(run_state, FloatOutBoyRunState::Running) {
        // `time_update` refreshes Float Out Boy's disengage and idle timers on every RUNNING loop
        // at `third_party/float-out-boy/src/time.c:38-43`.
        state.refresh_running_epochs(system_time_ticks);
    }

    let mut beep_reason = status.beep_reason();
    let mut beeper_alert = None;
    if startup_became_ready {
        let low_voltage_threshold = pack_voltage_threshold(
            state.serialized_config.low_voltage_threshold(),
            state.battery_cell_count,
        );
        let warning_threshold = low_voltage_threshold + Voltage::from_volts(5.0);
        let battery_voltage = base.motor().battery_voltage().voltage();
        if battery_voltage < warning_threshold {
            beep_reason = FloatOutBoyBeepReason::LowBattery;
        }
        beeper_alert = Some(FloatOutBoyBeeperAlert::Long(startup_ready_beep_count(
            warning_threshold,
            battery_voltage,
        )));
    }

    RefreshStart {
        ride_state,
        run_state,
        beep_reason,
        beeper_alert,
        startup_became_ready,
    }
}

fn refresh_darkride_state(
    state: &mut FloatOutBoyPackageState,
    mut ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    roll_abs: AngleDegrees,
    system_time_ticks: TimestampTicks,
) -> (FloatOutBoyRideState, Option<FloatOutBoyBeeperAlert>) {
    // C map: Float Out Boy activates darkride above 150 degrees only after a prior
    // RUNNING tick enables it, retains it through the hysteresis band, and
    // clears below 120 degrees at `third_party/float-out-boy/src/main.c:781-794`.
    if state.serialized_config.faults().darkride_enabled() {
        match ride_state.darkride() {
            FloatOutBoyDarkRideState::Active if roll_abs < AngleDegrees::from_degrees(120.0) => {
                ride_state = ride_state.with_darkride(FloatOutBoyDarkRideState::Upright);
            }
            FloatOutBoyDarkRideState::Upright
                if state.upside_down_flags.enabled
                    && roll_abs > AngleDegrees::from_degrees(150.0) =>
            {
                ride_state = ride_state.with_darkride(FloatOutBoyDarkRideState::Active);
                state.upside_down_flags.started = false;
            }
            _ => {}
        }
    }

    let reset_after_disengage = matches!(run_state, FloatOutBoyRunState::Ready)
        && float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 10);
    if !reset_after_disengage {
        return (ride_state, None);
    }

    // Float Out Boy removes the post-flip darkride grace after updating the
    // roll transition at `third_party/float-out-boy/src/main.c:781-794,984-992`.
    let alert = matches!(ride_state.darkride(), FloatOutBoyDarkRideState::Active)
        .then_some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::ONE));
    state.upside_down_flags.enabled = false;
    (
        ride_state.with_darkride(FloatOutBoyDarkRideState::Upright),
        alert,
    )
}

struct RuntimeValues {
    balance_current: FloatOutBoyRealtimeBalanceCurrent,
    setpoints: FloatOutBoyRealtimeRuntimeSetpoints,
    booster_current: FloatOutBoyRealtimeBoosterCurrent,
    motor: FloatOutBoyAllDataMotorPayload,
}

fn runtime_values(
    state: &mut FloatOutBoyPackageState,
    base: FloatOutBoyAllDataBasePayload,
    balance_pitch: AngleDegrees,
    reset: bool,
) -> RuntimeValues {
    if !reset {
        return RuntimeValues {
            balance_current: base.balance_current(),
            setpoints: base.setpoints(),
            booster_current: base.booster_current(),
            motor: base.motor(),
        };
    }

    // Upstream `reset_runtime_vars` clears control-loop history and seeds only
    // the board setpoint from the current balance pitch.
    state.balance_loop.reset_pid();
    state.balance_loop.softstart_pid_limit = MotorCurrent::new(Current::ZERO);
    state.reverse_total_erpm = Rpm::ZERO;
    state.motor_kinematics.reset_acceleration();
    state.motor_current_filter.reset_runtime();
    state.ride_flags.traction_control = false;
    state.remote_control.reset_runtime_vars();
    state.ride_modifiers.reset();
    state.runtime_board_setpoint = balance_pitch;
    let board_setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(balance_pitch);
    let zero_setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::ZERO);

    RuntimeValues {
        balance_current: FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::ZERO)),
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints::new(
            board_setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
        ),
        booster_current: FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::ZERO)),
        motor: base
            .motor()
            .with_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0))),
    }
}

#[cfg(any(test, target_arch = "arm"))]
fn refresh_ready_alert(
    state: &mut FloatOutBoyPackageState,
    base: FloatOutBoyAllDataBasePayload,
    run_state: FloatOutBoyRunState,
    ready_flywheel_stop: bool,
    system_time_ticks: TimestampTicks,
) -> Option<(FloatOutBoyBeepReason, FloatOutBoyBeeperAlert)> {
    if !matches!(run_state, FloatOutBoyRunState::Ready) || ready_flywheel_stop {
        return None;
    }

    let mut alert = None;
    if let Some(fault) = state
        .bms
        .take_ready_alert_fault(system_time_ticks, state.disengage_ticks)
    {
        let reason = match fault {
            super::bms_runtime::BmsReadyAlertFault::Connection => {
                FloatOutBoyBeepReason::BmsConnection
            }
            super::bms_runtime::BmsReadyAlertFault::CellBalance => {
                FloatOutBoyBeepReason::CellBalance
            }
        };
        alert = Some((
            reason,
            FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::FOUR),
        ));
    }

    // READY nags after 30 idle minutes, at most once per minute, and suppresses
    // the alert while pack voltage rises.
    if float_out_boy_ticks_elapsed(system_time_ticks, state.idle_ticks, 1_800) {
        if float_out_boy_ticks_elapsed(system_time_ticks, state.nag_ticks, 60) {
            state.nag_ticks = system_time_ticks;
            let battery_voltage = base.motor().battery_voltage();
            if battery_voltage > state.idle_voltage {
                state.idle_voltage = battery_voltage;
            } else {
                alert = Some((
                    FloatOutBoyBeepReason::Idle,
                    FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::TWO),
                ));
            }
        }
    } else {
        state.nag_ticks = system_time_ticks;
        state.idle_voltage = BatteryVoltage::new(Voltage::ZERO);
    }
    alert
}

struct TransitionEvents {
    startup_became_ready: bool,
    effect: FloatOutBoyTransitionEffect,
}

struct ControlConditions {
    decision: ControlDecision,
    darkride_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlDecision {
    Preserve,
    EnterReverseStop,
    DetectTractionLoss,
}

impl ControlDecision {
    #[must_use]
    const fn enters_reverse_stop(self) -> bool {
        matches!(self, Self::EnterReverseStop)
    }

    #[must_use]
    const fn detects_traction_loss(self) -> bool {
        matches!(self, Self::DetectTractionLoss)
    }
}

struct TransitionPhase {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    events: TransitionEvents,
    #[cfg(any(test, target_arch = "arm"))]
    ready_flywheel_stop: bool,
    balance_pitch: FloatOutBoyRealtimeBalancePitch,
    pitch_degrees: AngleDegrees,
    imu_pitch: ImuPitch,
    imu_roll: ImuRoll,
    motor_erpm: Rpm,
    control: ControlConditions,
    motor_acceleration: Rpm,
    startup_centering_step: AngleDegrees,
}

struct AttitudeInput {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    system_time_ticks: TimestampTicks,
}

struct AttitudeSnapshot {
    ride_state: FloatOutBoyRideState,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    imu_pitch: ImuPitch,
    imu_roll: ImuRoll,
    pitch: AngleRadians,
    pitch_degrees: AngleDegrees,
    pitch_abs: AngleDegrees,
    roll_abs: AngleDegrees,
}

fn transition_attitude(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    input: &AttitudeInput,
) -> AttitudeSnapshot {
    let (imu_pitch, imu_roll) = if matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        let (pitch, roll) = state.flywheel_attitude(
            input.ride_state.mode(),
            AngleDegrees::from(imu.pitch().angle()),
            AngleDegrees::from(imu.roll().angle()),
        );
        (
            ImuPitch::new(AngleRadians::from(pitch)),
            ImuRoll::new(AngleRadians::from(roll)),
        )
    } else {
        (imu.pitch(), imu.roll())
    };
    let pitch = imu_pitch.angle();
    let pitch_degrees = AngleDegrees::from(pitch);
    let pitch_abs = pitch_degrees.abs();
    let roll_abs = AngleDegrees::from(imu_roll.angle()).abs();
    state
        .ride_modifiers
        .aggregate_yaw(AngleDegrees::from(imu.yaw().angle()));
    let (ride_state, darkride_alert) = refresh_darkride_state(
        state,
        input.ride_state,
        input.run_state,
        roll_abs,
        input.system_time_ticks,
    );

    AttitudeSnapshot {
        ride_state,
        beeper_alert: darkride_alert.or(input.beeper_alert),
        imu_pitch,
        imu_roll,
        pitch,
        pitch_degrees,
        pitch_abs,
        roll_abs,
    }
}

fn refresh_footpad_warning(
    state: &mut FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    motor_erpm: Rpm,
    beep_reason: FloatOutBoyBeepReason,
) -> FloatOutBoyBeepReason {
    let switch_warning_erpm = if state.serialized_config.foot_beep_enabled() {
        Rpm::from_revolutions_per_minute(2_000.0)
    } else {
        Rpm::from_revolutions_per_minute(100_000.0)
    };
    let warning = matches!(run_state, FloatOutBoyRunState::Running)
        && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && !base.footpad().state().is_pressed()
        && motor_erpm.abs() > switch_warning_erpm;
    if warning {
        state.force_beeper_on();
        FloatOutBoyBeepReason::Sensors
    } else {
        state.release_beeper();
        beep_reason
    }
}

struct FlywheelReadiness {
    run_state: FloatOutBoyRunState,
    balance_pitch: FloatOutBoyRealtimeBalancePitch,
    ready_stop: bool,
}

fn refresh_flywheel_readiness(
    state: &mut FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    pitch: AngleRadians,
) -> FlywheelReadiness {
    // Float Out Boy normally uses its balance filter, while FLYWHEEL uses raw
    // pitch (`src/imu.c:35-41,56-58`).
    let balance_pitch = if matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
        FloatOutBoyRealtimeBalancePitch::new(pitch)
    } else {
        state.balance_filter.balance_pitch()
    };
    let ready_stop = matches!(run_state, FloatOutBoyRunState::Ready)
        && matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && state
            .flywheel
            .should_stop(base.footpad().state().is_pressed());
    let run_state = if ready_stop {
        state.prepare_flywheel_restore();
        state
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state()
    } else {
        run_state
    };
    FlywheelReadiness {
        run_state,
        balance_pitch,
        ready_stop,
    }
}

struct FaultInputs {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    pitch: AngleRadians,
    pitch_abs: AngleDegrees,
    roll_abs: AngleDegrees,
    balance_pitch_abs: AngleDegrees,
    remote_setpoint_abs: AngleDegrees,
    motor_erpm: Rpm,
    startup_became_ready: bool,
    ready_flywheel_stop: bool,
}

impl FaultInputs {
    #[must_use]
    const fn darkride_active(&self) -> bool {
        matches!(self.run_state, FloatOutBoyRunState::Running)
            && matches!(self.ride_state.darkride(), FloatOutBoyDarkRideState::Active)
    }
}

struct FaultContext<'a> {
    state: &'a FloatOutBoyPackageState,
    base: &'a FloatOutBoyAllDataBasePayload,
    now: TimestampTicks,
    input: &'a FaultInputs,
}

/// A timed check either refreshes its epoch, holds it, or requests a stop.
///
/// This preserves the C loop's distinction between a pending condition and a
/// triggered fault without coupling timer updates to positional booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimedFault {
    Clear,
    Pending,
    Stop(FloatOutBoyStopEvent),
}

impl TimedFault {
    #[must_use]
    const fn stop(self) -> Option<FloatOutBoyStopEvent> {
        match self {
            Self::Stop(event) => Some(event),
            Self::Clear | Self::Pending => None,
        }
    }
}

#[must_use]
fn full_switch_applicable(context: &FaultContext<'_>) -> bool {
    let input = context.input;
    matches!(input.run_state, FloatOutBoyRunState::Running)
        && !input.darkride_active()
        && !context.base.footpad().state().is_pressed()
        && !matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel)
}

#[must_use]
fn full_switch_stop_delay_held(context: &FaultContext<'_>) -> bool {
    let faults = context.state.serialized_config.faults();
    let input = context.input;
    let half_erpm = faults.adc_half_erpm().rpm();
    faults.moving_faults_disabled()
        && input.motor_erpm > half_erpm * 2.0
        && input.roll_abs < MovingFaultLimits::FLOAT_OUT_BOY.roll
}

#[must_use]
fn full_switch_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    let faults = context.state.serialized_config.faults();
    let input = context.input;
    let half_erpm = faults.adc_half_erpm().rpm();

    let full_delay_elapsed = float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_switch_ticks,
        faults.switch_full_delay(),
    );

    let slow_delay_elapsed = input.motor_erpm.abs() < half_erpm * 6.0
        && float_out_boy_ticks_elapsed_seconds(
            context.now,
            context.state.fault_switch_ticks,
            faults.switch_half_delay(),
        );

    full_delay_elapsed || slow_delay_elapsed
}

#[must_use]
fn classify_full_switch_fault(context: &FaultContext<'_>) -> TimedFault {
    if !full_switch_applicable(context) {
        return TimedFault::Clear;
    }
    if full_switch_stop_delay_held(context) {
        return TimedFault::Pending;
    }
    if full_switch_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::FullSwitch)
    } else {
        TimedFault::Pending
    }
}

#[must_use]
fn quick_stop_applicable(context: &FaultContext<'_>) -> bool {
    let input = context.input;
    matches!(input.run_state, FloatOutBoyRunState::Running)
        && !context.base.footpad().state().is_pressed()
        && !matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel)
        && context.state.serialized_config.faults().quickstop_enabled()
}

#[must_use]
fn quick_stop_thresholds_met(context: &FaultContext<'_>) -> bool {
    let input = context.input;
    let limits = QuickStopLimits::FLOAT_OUT_BOY;
    let below_stop_speed = input.motor_erpm.abs() < limits.stopped_erpm;
    let beyond_stop_pitch = input.pitch_abs > limits.pitch;
    let remote_setpoint_clear =
        input.remote_setpoint_abs < RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle();
    let pitch_matches_direction =
        (input.pitch >= AngleRadians::ZERO) == (input.motor_erpm >= Rpm::ZERO);

    below_stop_speed && beyond_stop_pitch && remote_setpoint_clear && pitch_matches_direction
}

#[must_use]
fn classify_quick_stop(context: &FaultContext<'_>) -> Option<FloatOutBoyStopEvent> {
    if !quick_stop_applicable(context) {
        return None;
    }
    quick_stop_thresholds_met(context).then_some(FloatOutBoyStopEvent::QuickStop)
}

#[must_use]
fn half_switch_applicable(context: &FaultContext<'_>, can_engage: EngagementPermission) -> bool {
    let faults = context.state.serialized_config.faults();
    let input = context.input;
    let below_half_speed = input.motor_erpm.abs() < faults.adc_half_erpm().rpm();

    matches!(input.run_state, FloatOutBoyRunState::Running)
        && !input.darkride_active()
        && !faults.dual_switch()
        && matches!(can_engage, EngagementPermission::Blocked)
        && below_half_speed
}

#[must_use]
fn half_switch_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_switch_half_ticks,
        context.state.serialized_config.faults().switch_half_delay(),
    )
}

#[must_use]
fn classify_half_switch_fault(
    context: &FaultContext<'_>,
    can_engage: EngagementPermission,
) -> TimedFault {
    if !half_switch_applicable(context, can_engage) {
        return TimedFault::Clear;
    }
    if half_switch_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::HalfSwitch)
    } else {
        TimedFault::Pending
    }
}

#[must_use]
fn roll_fault_applicable(context: &FaultContext<'_>) -> bool {
    let faults = context.state.serialized_config.faults();
    let input = context.input;
    matches!(input.run_state, FloatOutBoyRunState::Running)
        && !input.darkride_active()
        && input.roll_abs > faults.roll_angle()
}

#[must_use]
fn roll_fault_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_angle_roll_ticks,
        context.state.serialized_config.faults().roll_delay(),
    )
}

#[must_use]
fn classify_roll_fault(context: &FaultContext<'_>) -> TimedFault {
    if !roll_fault_applicable(context) {
        return TimedFault::Clear;
    }
    if roll_fault_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::Roll)
    } else {
        TimedFault::Pending
    }
}

#[must_use]
fn pitch_fault_applicable(context: &FaultContext<'_>) -> bool {
    let faults = context.state.serialized_config.faults();
    let input = context.input;
    matches!(input.run_state, FloatOutBoyRunState::Running)
        && input.pitch_abs > faults.pitch_angle()
        && input.remote_setpoint_abs < RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle()
}

#[must_use]
fn pitch_fault_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_angle_pitch_ticks,
        context.state.serialized_config.faults().pitch_delay(),
    )
}

#[must_use]
fn classify_pitch_fault(context: &FaultContext<'_>) -> TimedFault {
    if !pitch_fault_applicable(context) {
        return TimedFault::Clear;
    }
    if pitch_fault_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::Pitch)
    } else {
        TimedFault::Pending
    }
}

#[derive(Clone, Copy)]
enum EngagementPermission {
    Allowed,
    Blocked,
}

struct NormalFaultEvaluation {
    before_darkride: Option<FloatOutBoyStopEvent>,
    after_darkride: Option<FloatOutBoyStopEvent>,
    switch_angle: SwitchAngleFaultEvaluation,
    can_engage: EngagementPermission,
    flywheel_footpad_pressed: bool,
}

struct SwitchAngleFaultEvaluation {
    before_darkride: Option<FloatOutBoyStopEvent>,
    after_darkride: Option<FloatOutBoyStopEvent>,
    full_switch: TimedFault,
    half_switch: TimedFault,
    roll: TimedFault,
    pitch: TimedFault,
}

#[must_use]
fn evaluate_switch_angle_faults(
    context: &FaultContext<'_>,
    can_engage: EngagementPermission,
) -> SwitchAngleFaultEvaluation {
    let full_switch = classify_full_switch_fault(context);
    let quick_stop = classify_quick_stop(context);
    let half_switch = classify_half_switch_fault(context, can_engage);
    let roll = classify_roll_fault(context);
    let pitch = classify_pitch_fault(context);
    SwitchAngleFaultEvaluation {
        before_darkride: full_switch.stop().or(quick_stop).or(half_switch.stop()),
        after_darkride: roll.stop().or(pitch.stop()),
        full_switch,
        half_switch,
        roll,
        pitch,
    }
}

pub(super) struct ActiveReverseStopFaultInput {
    pub(super) footpad: FloatOutBoyFootpadState,
    pub(super) darkride: FloatOutBoyDarkRideState,
    pub(super) pitch: AngleDegrees,
    pub(super) elapsed: SystemTicks,
    pub(super) total_erpm: Rpm,
}

impl ActiveReverseStopFaultInput {
    #[must_use]
    pub(super) fn stop_event(self) -> Option<FloatOutBoyStopEvent> {
        let limits = ReverseStopLimits::FLOAT_OUT_BOY;
        if !self.footpad.is_pressed() {
            return Some(FloatOutBoyStopEvent::ReverseStopNoFootpads);
        }
        if matches!(self.darkride, FloatOutBoyDarkRideState::Active) {
            return None;
        }
        // CodeRabbit suggested an inclusive 5-degree slow-timer boundary;
        // the C map (`third_party/float-out-boy/src/main.c:538-552`) is
        // strict (`pitch > 10`/`pitch > 5`) here, while Refloat refreshes
        // below `TARGET_STOP_ANGLE / 2` (8.5 degrees) separately.
        if self.pitch > limits.pitch {
            return Some(FloatOutBoyStopEvent::ReverseStopPitch);
        }
        let fast_timer_expired = self.pitch > limits.timer_fast_pitch
            && self.elapsed > VescSeconds::from_seconds(1.0).to_system_ticks_saturating();
        let slow_timer_expired = self.pitch > limits.timer_slow_pitch
            && self.elapsed > VescSeconds::from_seconds(2.0).to_system_ticks_saturating();
        if fast_timer_expired {
            return Some(FloatOutBoyStopEvent::ReverseStopTimer);
        }
        if slow_timer_expired {
            return Some(FloatOutBoyStopEvent::ReverseStopTimer);
        }
        if self.total_erpm.abs() > limits.total_erpm {
            return Some(FloatOutBoyStopEvent::ReverseStopTotalErpm);
        }
        None
    }
}

#[must_use]
fn engagement_permission(context: &FaultContext<'_>) -> EngagementPermission {
    let state = context.state;
    let input = context.input;
    if !matches!(
        input.ride_state.charging(),
        FloatOutBoyChargingState::NotCharging
    ) {
        return EngagementPermission::Blocked;
    }

    let footpad = context.base.footpad().state();
    if matches!(footpad, FloatOutBoyFootpadState::Both) {
        return EngagementPermission::Allowed;
    }
    if matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        return EngagementPermission::Allowed;
    }
    if !matches!(
        footpad,
        FloatOutBoyFootpadState::Left | FloatOutBoyFootpadState::Right
    ) {
        return EngagementPermission::Blocked;
    }

    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    if faults.dual_switch() {
        return EngagementPermission::Allowed;
    }
    if !startup.simplestart_enabled() {
        return EngagementPermission::Blocked;
    }
    if float_out_boy_ticks_elapsed(context.now, state.disengage_ticks, 2) {
        return EngagementPermission::Allowed;
    }
    if !float_out_boy_ticks_elapsed(context.now, state.engage_ticks, 1) {
        return EngagementPermission::Allowed;
    }
    EngagementPermission::Blocked
}

#[must_use]
fn classify_flywheel_footpad(context: &FaultContext<'_>) -> Option<FloatOutBoyStopEvent> {
    if !matches!(context.input.run_state, FloatOutBoyRunState::Running) {
        return None;
    }
    if !matches!(context.input.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        return None;
    }
    if !context.base.footpad().state().is_pressed() {
        return None;
    }
    Some(FloatOutBoyStopEvent::FlywheelFootpad)
}

#[must_use]
fn darkride_roll_applicable(context: &FaultContext<'_>) -> bool {
    let input = context.input;
    matches!(input.run_state, FloatOutBoyRunState::Running)
        && !input.darkride_active()
        && matches!(
            input.ride_state.darkride(),
            FloatOutBoyDarkRideState::Upright
        )
        && context.state.serialized_config.faults().darkride_enabled()
}

#[must_use]
fn darkride_roll_thresholds_met(context: &FaultContext<'_>) -> bool {
    let limits = DarkrideLimits::FLOAT_OUT_BOY;
    let above_lower_limit = context.input.roll_abs > limits.roll_lower;
    let below_upper_limit = context.input.roll_abs < limits.roll_upper;
    above_lower_limit && below_upper_limit
}

#[must_use]
fn classify_darkride_roll(context: &FaultContext<'_>) -> Option<FloatOutBoyStopEvent> {
    if !darkride_roll_applicable(context) {
        return None;
    }
    darkride_roll_thresholds_met(context).then_some(FloatOutBoyStopEvent::DarkrideRoll)
}

#[must_use]
fn classify_reverse_stop(context: &FaultContext<'_>) -> Option<FloatOutBoyStopEvent> {
    let input = context.input;
    if !matches!(input.run_state, FloatOutBoyRunState::Running) {
        return None;
    }
    if !matches!(
        input.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::ReverseStop
    ) {
        return None;
    }
    ActiveReverseStopFaultInput {
        footpad: context.base.footpad().state(),
        darkride: input.ride_state.darkride(),
        pitch: input.pitch_abs,
        elapsed: context
            .now
            .wrapping_duration_since(context.state.reverse_ticks),
        total_erpm: context.state.reverse_total_erpm,
    }
    .stop_event()
}

#[must_use]
fn evaluate_normal_faults(context: &FaultContext<'_>) -> NormalFaultEvaluation {
    let flywheel_footpad = classify_flywheel_footpad(context);
    let reverse_stop = classify_reverse_stop(context);
    let can_engage = engagement_permission(context);
    let switch_angle = evaluate_switch_angle_faults(context, can_engage);
    let darkride_roll = classify_darkride_roll(context);

    NormalFaultEvaluation {
        before_darkride: flywheel_footpad
            .or(reverse_stop)
            .or(switch_angle.before_darkride),
        after_darkride: switch_angle.after_darkride.or(darkride_roll),
        switch_angle,
        can_engage,
        flywheel_footpad_pressed: flywheel_footpad.is_some(),
    }
}

struct DarkrideFaultEvaluation {
    stop: Option<FloatOutBoyStopEvent>,
    high_erpm: TimedFault,
    low_erpm: TimedFault,
}

#[must_use]
fn darkride_high_erpm_applicable(context: &FaultContext<'_>) -> bool {
    let limits = DarkrideLimits::FLOAT_OUT_BOY;
    let input = context.input;
    input.darkride_active() && input.motor_erpm > limits.timed_high_erpm
}

#[must_use]
fn darkride_wheelslip_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    if !matches!(
        context.input.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::Detected
    ) {
        return false;
    }
    if !float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.upside_down_fault_ticks,
        VescSeconds::from_seconds(1.0),
    ) {
        return false;
    }
    float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_switch_ticks,
        VescSeconds::from_seconds(0.03),
    )
}

#[must_use]
fn darkride_high_erpm_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    let limits = DarkrideLimits::FLOAT_OUT_BOY;
    if float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_switch_ticks,
        limits.timed_high_delay,
    ) {
        return true;
    }
    if context.input.motor_erpm > limits.high_erpm {
        return true;
    }
    // Active darkride shortens the wheelslip runaway stop from 100 ms to
    // 30 ms after the one-second post-flip grace (`src/main.c:361-366`).
    darkride_wheelslip_timeout_elapsed(context)
}

#[must_use]
fn classify_darkride_high_erpm_fault(context: &FaultContext<'_>) -> TimedFault {
    if !darkride_high_erpm_applicable(context) {
        return TimedFault::Clear;
    }
    if darkride_high_erpm_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::DarkrideHighErpm)
    } else {
        TimedFault::Pending
    }
}

#[must_use]
fn darkride_low_erpm_applicable(context: &FaultContext<'_>) -> bool {
    let limits = DarkrideLimits::FLOAT_OUT_BOY;
    let input = context.input;
    input.darkride_active()
        && input.motor_erpm <= limits.timed_high_erpm
        && input.motor_erpm > limits.low_erpm
}

#[must_use]
fn darkride_low_erpm_timeout_elapsed(context: &FaultContext<'_>) -> bool {
    float_out_boy_ticks_elapsed_seconds(
        context.now,
        context.state.fault_angle_roll_ticks,
        DarkrideLimits::FLOAT_OUT_BOY.low_delay,
    )
}

#[must_use]
fn classify_darkride_low_erpm_fault(context: &FaultContext<'_>) -> TimedFault {
    if !darkride_low_erpm_applicable(context) {
        return TimedFault::Clear;
    }
    if darkride_low_erpm_timeout_elapsed(context) {
        TimedFault::Stop(FloatOutBoyStopEvent::DarkrideLowErpm)
    } else {
        TimedFault::Pending
    }
}

#[must_use]
fn classify_darkride_engagement_fault(
    context: &FaultContext<'_>,
    can_engage: EngagementPermission,
) -> Option<FloatOutBoyStopEvent> {
    if !context.input.darkride_active() {
        return None;
    }
    if matches!(can_engage, EngagementPermission::Blocked) {
        return None;
    }
    Some(FloatOutBoyStopEvent::DarkrideCanEngage)
}

#[must_use]
fn evaluate_darkride_faults(
    context: &FaultContext<'_>,
    can_engage: EngagementPermission,
) -> DarkrideFaultEvaluation {
    let high_erpm = classify_darkride_high_erpm_fault(context);
    let low_erpm = classify_darkride_low_erpm_fault(context);
    let engagement_stop = classify_darkride_engagement_fault(context, can_engage);
    DarkrideFaultEvaluation {
        stop: high_erpm.stop().or(low_erpm.stop()).or(engagement_stop),
        high_erpm,
        low_erpm,
    }
}

struct EngagementEvaluation {
    decision: FloatOutBoyEngagementDecision,
    centering_step: AngleDegrees,
}

const DIRTY_LANDING_PITCH_MARGIN_DEGREES: u8 = 10;

#[must_use]
fn dirty_landing_pitch_margin(context: &FaultContext<'_>) -> AngleDegrees {
    if !matches!(
        context.input.ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchFull
    ) {
        return AngleDegrees::ZERO;
    }
    if !context
        .state
        .serialized_config
        .startup()
        .dirty_landings_enabled()
    {
        return AngleDegrees::ZERO;
    }
    if float_out_boy_ticks_elapsed(context.now, context.state.fault_angle_pitch_ticks, 1) {
        return AngleDegrees::ZERO;
    }
    AngleDegrees::from_degrees(f32::from(DIRTY_LANDING_PITCH_MARGIN_DEGREES))
}

struct EngagementCandidate<'context, 'state> {
    faults: &'context FaultContext<'state>,
    permission: EngagementPermission,
    pitch_tolerance: AngleDegrees,
    roll_tolerance: AngleDegrees,
}

#[must_use]
fn normal_engagement(candidate: &EngagementCandidate<'_, '_>) -> FloatOutBoyEngagementDecision {
    let input = candidate.faults.input;
    if input.ready_flywheel_stop {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    if matches!(candidate.permission, EngagementPermission::Blocked) {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let within_pitch = input.balance_pitch_abs < candidate.pitch_tolerance;
    if !within_pitch {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let within_roll = input.roll_abs < candidate.roll_tolerance;
    if !within_roll {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    FloatOutBoyEngagementDecision::Engage
}

#[must_use]
fn darkride_engagement(candidate: &EngagementCandidate<'_, '_>) -> FloatOutBoyEngagementDecision {
    let context = candidate.faults;
    let input = context.input;
    if !matches!(
        input.ride_state.darkride(),
        FloatOutBoyDarkRideState::Active
    ) {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let within_pitch = input.balance_pitch_abs < candidate.pitch_tolerance;
    if !within_pitch {
        return FloatOutBoyEngagementDecision::Preserve;
    }

    // READY darkride either ignores roll during its initial grace or requires
    // upside-down roll within startup tolerance.
    let within_grace = if float_out_boy_ticks_elapsed(context.now, context.state.disengage_ticks, 1)
    {
        false
    } else {
        !matches!(
            input.ride_state.stop_condition(),
            FloatOutBoyStopCondition::ReverseStop
        )
    };
    let upside_down =
        (input.roll_abs - AngleDegrees::from_degrees(180.0)).abs() < candidate.roll_tolerance;
    if within_grace {
        return FloatOutBoyEngagementDecision::Engage;
    }
    if upside_down {
        return FloatOutBoyEngagementDecision::Engage;
    }
    FloatOutBoyEngagementDecision::Preserve
}

#[must_use]
fn push_start_engagement(candidate: &EngagementCandidate<'_, '_>) -> FloatOutBoyEngagementDecision {
    let context = candidate.faults;
    let input = context.input;
    let faults = context.state.serialized_config.faults();
    let startup = context.state.serialized_config.startup();
    let limits = PushStartLimits::FLOAT_OUT_BOY;
    if !startup.pushstart_enabled() {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    if matches!(candidate.permission, EngagementPermission::Blocked) {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    if faults.reversestop_enabled() && input.motor_erpm.is_negative() {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let moving_fast_enough = input.motor_erpm.abs() > limits.erpm_min;
    if !moving_fast_enough {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let within_pitch = input.balance_pitch_abs < limits.angle;
    if !within_pitch {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    let within_roll = input.roll_abs < limits.angle;
    if !within_roll {
        return FloatOutBoyEngagementDecision::Preserve;
    }
    FloatOutBoyEngagementDecision::Engage
}

#[must_use]
fn evaluate_engagement(
    context: &FaultContext<'_>,
    can_engage: EngagementPermission,
) -> EngagementEvaluation {
    let state = context.state;
    let input = context.input;
    let startup = state.serialized_config.startup();
    let centering_step = startup.centering_step();
    if input.startup_became_ready {
        return EngagementEvaluation {
            decision: FloatOutBoyEngagementDecision::Preserve,
            centering_step,
        };
    }
    if !matches!(input.run_state, FloatOutBoyRunState::Ready) {
        return EngagementEvaluation {
            decision: FloatOutBoyEngagementDecision::Preserve,
            centering_step,
        };
    }

    let pitch_tolerance = startup.pitch_tolerance() + dirty_landing_pitch_margin(context);
    let candidate = EngagementCandidate {
        faults: context,
        permission: can_engage,
        pitch_tolerance,
        roll_tolerance: startup.roll_tolerance(),
    };
    let decision = match normal_engagement(&candidate) {
        FloatOutBoyEngagementDecision::Engage => FloatOutBoyEngagementDecision::Engage,
        FloatOutBoyEngagementDecision::Preserve => match darkride_engagement(&candidate) {
            FloatOutBoyEngagementDecision::Engage => FloatOutBoyEngagementDecision::Engage,
            FloatOutBoyEngagementDecision::Preserve => push_start_engagement(&candidate),
        },
    };
    EngagementEvaluation {
        decision,
        centering_step,
    }
}

fn first_transition_stop(
    normal: &NormalFaultEvaluation,
    darkride: &DarkrideFaultEvaluation,
) -> Option<FloatOutBoyStopEvent> {
    normal
        .before_darkride
        .or(darkride.stop)
        .or(normal.after_darkride)
}

struct ControlEvaluation {
    conditions: ControlConditions,
    motor_acceleration: Rpm,
}

#[must_use]
fn should_enter_reverse_stop(context: &FaultContext<'_>) -> bool {
    let input = context.input;
    if matches!(
        input.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) {
        return false;
    }
    if input.darkride_active() {
        return false;
    }
    if !context
        .state
        .serialized_config
        .faults()
        .reversestop_enabled()
    {
        return false;
    }
    input.motor_erpm < -ReverseStopLimits::FLOAT_OUT_BOY.entry_erpm
}

struct TractionLossInput<'context, 'state> {
    faults: &'context FaultContext<'state>,
    engagement: FloatOutBoyEngagementDecision,
    stop_event: Option<FloatOutBoyStopEvent>,
    motor_acceleration: Rpm,
}

#[must_use]
fn detects_traction_loss(candidate: &TractionLossInput<'_, '_>) -> bool {
    let context = candidate.faults;
    let input = context.input;
    if candidate.stop_event.is_some() {
        return false;
    }
    if matches!(candidate.engagement, FloatOutBoyEngagementDecision::Engage) {
        return false;
    }
    if !matches!(input.run_state, FloatOutBoyRunState::Running) {
        return false;
    }
    if matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        return false;
    }
    if matches!(
        input.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) {
        return false;
    }

    let limits = TractionLossLimits::FLOAT_OUT_BOY;
    let accelerating_fast_enough = candidate.motor_acceleration.abs() > limits.acceleration_detect;
    if !accelerating_fast_enough {
        return false;
    }
    let acceleration_matches_direction =
        candidate.motor_acceleration.is_negative() == input.motor_erpm.is_negative();
    if !acceleration_matches_direction {
        return false;
    }
    let above_duty_threshold = context.base.motor().duty_cycle().ratio() > limits.duty;
    if !above_duty_threshold {
        return false;
    }
    let above_speed_threshold = input.motor_erpm.abs() > limits.erpm;
    if !above_speed_threshold {
        return false;
    }
    true
}

#[must_use]
fn evaluate_control_conditions(
    context: &FaultContext<'_>,
    engagement: FloatOutBoyEngagementDecision,
    stop_event: Option<FloatOutBoyStopEvent>,
) -> ControlEvaluation {
    let motor_acceleration = context.state.motor_kinematics.average();
    let decision = if should_enter_reverse_stop(context) {
        ControlDecision::EnterReverseStop
    } else if detects_traction_loss(&TractionLossInput {
        faults: context,
        engagement,
        stop_event,
        motor_acceleration,
    }) {
        ControlDecision::DetectTractionLoss
    } else {
        ControlDecision::Preserve
    };
    ControlEvaluation {
        conditions: ControlConditions {
            decision,
            darkride_active: context.input.darkride_active(),
        },
        motor_acceleration,
    }
}

struct TransitionActivity<'a> {
    input: &'a FaultInputs,
    normal: &'a NormalFaultEvaluation,
    darkride: &'a DarkrideFaultEvaluation,
    control: &'a ControlConditions,
    engagement: &'a EngagementEvaluation,
    stop_event: Option<FloatOutBoyStopEvent>,
}

fn apply_transition_activity(
    state: &mut FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    activity: &TransitionActivity<'_>,
) -> FloatOutBoyStateTransitionOutput {
    let transition = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        previous: activity.input.ride_state,
        run_state: activity.input.run_state,
        ready_flywheel_stop: activity.input.ready_flywheel_stop,
        engagement: activity.engagement.decision,
        traction_loss_detected: activity.control.decision.detects_traction_loss(),
        stop_event: activity.stop_event,
    });
    match transition.effect {
        FloatOutBoyTransitionEffect::Stop(event) => {
            state.play_motor_click();
            state.disengage_ticks = system_time_ticks;
            state.trigger_data_recorder(false);
            if matches!(event, FloatOutBoyStopEvent::FullSwitch) {
                state.fault_angle_pitch_ticks = system_time_ticks;
            }
            state
                .flywheel
                .latch_abort(activity.normal.flywheel_footpad_pressed);
        }
        FloatOutBoyTransitionEffect::Engage => {
            state.play_motor_click();
            state.engage_ticks = system_time_ticks;
            state.trigger_data_recorder(true);
        }
        FloatOutBoyTransitionEffect::Preserve => {}
    }
    let stopped = matches!(transition.effect, FloatOutBoyTransitionEffect::Stop(_));
    if matches!(activity.input.run_state, FloatOutBoyRunState::Running) && !stopped {
        state.upside_down_flags.enabled = true;
        if activity.input.darkride_active() && !state.upside_down_flags.started {
            state.upside_down_flags.started = true;
            state.upside_down_fault_ticks = system_time_ticks;
        }
    }
    if matches!(activity.darkride.high_erpm, TimedFault::Clear)
        && matches!(activity.normal.switch_angle.full_switch, TimedFault::Clear)
    {
        state.fault_switch_ticks = system_time_ticks;
    }
    if matches!(activity.normal.switch_angle.half_switch, TimedFault::Clear) {
        state.fault_switch_half_ticks = system_time_ticks;
    }
    // The legacy C map refreshed below 5 degrees while its timer started above
    // 5 degrees. Include equality so it cannot retain an inactive timer epoch.
    if !matches!(
        (
            activity.input.run_state,
            activity.input.ride_state.setpoint_adjustment()
        ),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    ) || reverse_stop_timer_inactive(activity.input.pitch_abs)
    {
        state.reverse_ticks = system_time_ticks;
    }
    if matches!(activity.darkride.low_erpm, TimedFault::Clear)
        && matches!(activity.normal.switch_angle.roll, TimedFault::Clear)
    {
        state.fault_angle_roll_ticks = system_time_ticks;
    }
    if matches!(activity.normal.switch_angle.pitch, TimedFault::Clear) {
        state.fault_angle_pitch_ticks = system_time_ticks;
    }
    transition
}

#[must_use]
pub(super) fn reverse_stop_timer_inactive(pitch_abs: AngleDegrees) -> bool {
    pitch_abs <= ReverseStopLimits::FLOAT_OUT_BOY.timer_slow_pitch
}

fn evaluate_transition_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    start: &RefreshStart,
) -> TransitionPhase {
    let attitude = transition_attitude(
        state,
        imu,
        &AttitudeInput {
            ride_state: start.ride_state,
            run_state: start.run_state,
            beeper_alert: start.beeper_alert,
            system_time_ticks,
        },
    );
    let ride_state = attitude.ride_state;
    let motor_erpm = base.motor().electrical_speed().rpm();
    let beep_reason = refresh_footpad_warning(
        state,
        base,
        ride_state,
        start.run_state,
        motor_erpm,
        start.beep_reason,
    );
    let readiness =
        refresh_flywheel_readiness(state, base, ride_state, start.run_state, attitude.pitch);
    let fault_inputs = FaultInputs {
        ride_state,
        run_state: readiness.run_state,
        pitch: attitude.pitch,
        pitch_abs: attitude.pitch_abs,
        roll_abs: attitude.roll_abs,
        balance_pitch_abs: readiness.balance_pitch.angle_degrees().abs(),
        remote_setpoint_abs: base.setpoints().remote().angle().abs(),
        motor_erpm,
        startup_became_ready: start.startup_became_ready,
        ready_flywheel_stop: readiness.ready_stop,
    };
    let context = FaultContext {
        state,
        base,
        now: system_time_ticks,
        input: &fault_inputs,
    };
    let normal = evaluate_normal_faults(&context);
    let darkride = evaluate_darkride_faults(&context, normal.can_engage);
    let engagement = evaluate_engagement(&context, normal.can_engage);
    let stop_event = first_transition_stop(&normal, &darkride);
    let control = evaluate_control_conditions(&context, engagement.decision, stop_event);
    let outcome = apply_transition_activity(
        state,
        system_time_ticks,
        &TransitionActivity {
            input: &fault_inputs,
            normal: &normal,
            darkride: &darkride,
            control: &control.conditions,
            engagement: &engagement,
            stop_event,
        },
    );

    TransitionPhase {
        ride_state: outcome.ride_state,
        run_state: readiness.run_state,
        beep_reason,
        beeper_alert: attitude.beeper_alert,
        events: TransitionEvents {
            startup_became_ready: start.startup_became_ready,
            effect: outcome.effect,
        },
        #[cfg(any(test, target_arch = "arm"))]
        ready_flywheel_stop: readiness.ready_stop,
        balance_pitch: readiness.balance_pitch,
        pitch_degrees: attitude.pitch_degrees,
        imu_pitch: attitude.imu_pitch,
        imu_roll: attitude.imu_roll,
        motor_erpm,
        control: control.conditions,
        motor_acceleration: control.motor_acceleration,
        startup_centering_step: engagement.centering_step,
    }
}

struct RunningControl {
    ride_state: FloatOutBoyRideState,
    board_setpoint: AngleDegrees,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
}

struct ProtectionSignals {
    high_voltage_threshold: Voltage,
    low_voltage_threshold: Voltage,
    battery_voltage: Voltage,
    bms_cell_over_voltage: bool,
    bms_connection_fault: bool,
    bms_cell_under_voltage: bool,
    bms_temperature_reason: Option<FloatOutBoyBeepReason>,
    motor_temperature_warning: Option<(FloatOutBoyBeepReason, bool)>,
}

fn protection_signals(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
) -> ProtectionSignals {
    #[cfg(any(test, target_arch = "arm"))]
    let bms_cell_over_voltage = state.bms.contains(FloatOutBoyBmsFault::CellOverVoltage);
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_cell_over_voltage = false;
    #[cfg(any(test, target_arch = "arm"))]
    let bms_connection_fault = state.bms.contains(FloatOutBoyBmsFault::Connection);
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_connection_fault = false;
    #[cfg(any(test, target_arch = "arm"))]
    let bms_temperature_reason = if state.bms.contains(FloatOutBoyBmsFault::CellOverTemperature) {
        Some(FloatOutBoyBeepReason::CellOverTemperature)
    } else if state
        .bms
        .contains(FloatOutBoyBmsFault::CellUnderTemperature)
    {
        Some(FloatOutBoyBeepReason::CellUnderTemperature)
    } else if state.bms.contains(FloatOutBoyBmsFault::BmsOverTemperature) {
        Some(FloatOutBoyBeepReason::BmsOverTemperature)
    } else {
        None
    };
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_temperature_reason = None;
    #[cfg(any(test, target_arch = "arm"))]
    let bms_cell_under_voltage = state.bms.contains(FloatOutBoyBmsFault::CellUnderVoltage);
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_cell_under_voltage = false;
    let warning_margin = Temperature::from_degrees_celsius(3.0);
    let tiltback_margin = Temperature::from_degrees_celsius(1.0);
    let mosfet_threshold = state.mosfet_temperature_limit_start.temperature() - warning_margin;
    let motor_threshold = state.motor_temperature_limit_start.temperature() - warning_margin;
    let motor_temperature_warning = if state.mosfet_temperature.temperature() > mosfet_threshold {
        Some((
            FloatOutBoyBeepReason::MosfetTemperature,
            state.mosfet_temperature.temperature() > mosfet_threshold + tiltback_margin,
        ))
    } else if state.motor_temperature.temperature() > motor_threshold {
        Some((
            FloatOutBoyBeepReason::MotorTemperature,
            state.motor_temperature.temperature() > motor_threshold + tiltback_margin,
        ))
    } else {
        None
    };
    ProtectionSignals {
        high_voltage_threshold: pack_voltage_threshold(
            state.serialized_config.high_voltage_threshold(),
            state.battery_cell_count,
        ),
        low_voltage_threshold: pack_voltage_threshold(
            state.serialized_config.low_voltage_threshold(),
            state.battery_cell_count,
        ),
        battery_voltage: base.motor().battery_voltage().voltage(),
        bms_cell_over_voltage,
        bms_connection_fault,
        bms_cell_under_voltage,
        bms_temperature_reason,
        motor_temperature_warning,
    }
}

fn enter_reverse_stop(
    state: &mut FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    pending: bool,
    control: &mut RunningControl,
) {
    if !pending {
        return;
    }
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    state.reverse_total_erpm = if matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::PushbackHighVoltage
            | FloatOutBoySetpointAdjustment::PushbackLowVoltage
            | FloatOutBoySetpointAdjustment::PushbackTemperature
    ) {
        reverse_stop.carryover_total_erpm(control.board_setpoint)
    } else {
        Rpm::ZERO
    };
    state.reverse_ticks = system_time_ticks;
    control.ride_state = control
        .ride_state
        .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::ReverseStop);
}

fn refresh_wheelslip_control(
    state: &mut FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    phase: &TransitionPhase,
    above_duty_limit: bool,
    control: &mut RunningControl,
) -> bool {
    let limits = TractionLossLimits::FLOAT_OUT_BOY;
    if phase.control.decision.detects_traction_loss() {
        state.wheelslip_ticks = system_time_ticks;
        if phase.control.darkride_active {
            state.ride_flags.traction_control = true;
        }
        return true;
    }
    if !matches!(
        control.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::Detected
    ) || matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) {
        return false;
    }
    if phase.motor_acceleration.abs() < limits.acceleration_clear {
        state.ride_flags.traction_control = false;
    }
    if above_duty_limit {
        state.wheelslip_ticks = system_time_ticks;
    } else if float_out_boy_ticks_elapsed_seconds(
        system_time_ticks,
        state.wheelslip_ticks,
        limits.clear_delay,
    ) && state.motor_duty_raw < limits.raw_duty_clear
    {
        state.ride_flags.traction_control = false;
        control.ride_state = control
            .ride_state
            .with_wheelslip(FloatOutBoyWheelSlipState::None);
    }
    true
}

fn refresh_centering(phase: &TransitionPhase, control: &mut RunningControl) {
    if !matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    ) {
        return;
    }
    if control.board_setpoint.is_zero() {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
    } else if control.board_setpoint.abs() < phase.startup_centering_step {
        control.board_setpoint = AngleDegrees::ZERO;
    } else {
        control.board_setpoint =
            control.board_setpoint - phase.startup_centering_step * control.board_setpoint.signum();
    }
}

fn refresh_reverse_stop(
    state: &mut FloatOutBoyPackageState,
    phase: &TransitionPhase,
    control: &mut RunningControl,
) {
    if phase.control.decision.enters_reverse_stop()
        || !matches!(
            control.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    {
        return;
    }
    let limits = ReverseStopLimits::FLOAT_OUT_BOY;
    state.reverse_total_erpm = state.reverse_total_erpm + phase.motor_erpm;
    let total = state.reverse_total_erpm.abs();
    let setpoint = if total > limits.tolerance_erpm {
        Some(limits.target_angle(state.reverse_total_erpm))
    } else if total <= limits.tolerance_erpm * 0.5 && !phase.motor_erpm.is_negative() {
        state.reverse_total_erpm = Rpm::ZERO;
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        Some(AngleDegrees::ZERO)
    } else {
        None
    };
    if let Some(setpoint) = setpoint {
        control.board_setpoint = setpoint;
    }
}

struct ProtectionContext<'a> {
    base: &'a FloatOutBoyAllDataBasePayload,
    phase: &'a TransitionPhase,
    signals: &'a ProtectionSignals,
    system_time_ticks: TimestampTicks,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProtectionAction {
    DutyPushback {
        target: AngleDegrees,
        step: AngleDegrees,
    },
    FlywheelDutyLimit {
        target: AngleDegrees,
        step: AngleDegrees,
    },
    HighVoltageWarning {
        reason: FloatOutBoyBeepReason,
    },
    HighVoltagePushback {
        reason: FloatOutBoyBeepReason,
        target: AngleDegrees,
    },
    BmsConnection {
        target: AngleDegrees,
    },
    MotorTemperatureWarning {
        reason: FloatOutBoyBeepReason,
    },
    MotorTemperaturePushback {
        reason: FloatOutBoyBeepReason,
        target: AngleDegrees,
    },
    BmsTemperaturePushback {
        reason: FloatOutBoyBeepReason,
        target: AngleDegrees,
    },
    LowVoltageWarning {
        reason: FloatOutBoyBeepReason,
    },
    LowVoltagePushback {
        reason: FloatOutBoyBeepReason,
        target: AngleDegrees,
    },
    SpeedPushback {
        target: AngleDegrees,
        step: AngleDegrees,
    },
}

fn apply_protection_action(action: ProtectionAction, control: &mut RunningControl) {
    match action {
        ProtectionAction::DutyPushback { target, step } => {
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);
            control.board_setpoint = rate_limit_angle(control.board_setpoint, target, step);
        }
        ProtectionAction::FlywheelDutyLimit { target, step } => {
            control.board_setpoint = rate_limit_angle(control.board_setpoint, target, step);
        }
        ProtectionAction::HighVoltageWarning { reason } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        }
        ProtectionAction::HighVoltagePushback { reason, target } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackHighVoltage);
            control.board_setpoint = target;
        }
        ProtectionAction::BmsConnection { target } => {
            control.beep_reason = FloatOutBoyBeepReason::BmsConnection;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackError);
            control.board_setpoint = target;
        }
        ProtectionAction::MotorTemperatureWarning { reason } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        }
        ProtectionAction::MotorTemperaturePushback { reason, target }
        | ProtectionAction::BmsTemperaturePushback { reason, target } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
            control.board_setpoint = target;
        }
        ProtectionAction::LowVoltageWarning { reason } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
            control.board_setpoint = AngleDegrees::ZERO;
        }
        ProtectionAction::LowVoltagePushback { reason, target } => {
            control.beep_reason = reason;
            control.beeper_alert =
                Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackLowVoltage);
            control.board_setpoint = target;
        }
        ProtectionAction::SpeedPushback { target, step } => {
            control.beep_reason = FloatOutBoyBeepReason::Speed;
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackSpeed);
            control.board_setpoint = rate_limit_angle(control.board_setpoint, target, step);
        }
    }
}

fn directional_angle(angle: AngleDegrees, motor_erpm: Rpm) -> AngleDegrees {
    if motor_erpm.is_positive() {
        angle
    } else {
        -angle
    }
}

#[must_use]
fn select_duty_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &RunningControl,
) -> Option<ProtectionAction> {
    if context.base.motor().duty_cycle().ratio().as_ratio()
        <= state.runtime_duty_pushback_threshold().as_ratio()
    {
        return None;
    }
    let angle = state.runtime_duty_pushback_angle();
    let target = directional_angle(angle, context.phase.motor_erpm);
    let step = state.runtime_duty_pushback_step();
    if matches!(control.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        Some(ProtectionAction::FlywheelDutyLimit { target, step })
    } else {
        Some(ProtectionAction::DutyPushback { target, step })
    }
}

#[must_use]
fn select_high_voltage_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    let signals = context.signals;
    if context.base.motor().duty_cycle().ratio().as_ratio() <= 0.05
        || !(signals.battery_voltage > signals.high_voltage_threshold
            || signals.bms_cell_over_voltage)
    {
        return None;
    }
    let beep_reason = if signals.bms_cell_over_voltage {
        FloatOutBoyBeepReason::CellHighVoltage
    } else {
        FloatOutBoyBeepReason::HighVoltage
    };
    let tiltback = float_out_boy_ticks_elapsed_seconds(
        context.system_time_ticks,
        state.high_voltage_ticks,
        VescSeconds::from_seconds(0.5),
    ) || signals.battery_voltage
        > signals.high_voltage_threshold + Voltage::from_volts(1.0)
        || signals.bms_cell_over_voltage;
    if tiltback {
        Some(ProtectionAction::HighVoltagePushback {
            reason: beep_reason,
            target: directional_angle(
                state.serialized_config.high_voltage_pushback_angle(),
                context.phase.motor_erpm,
            ),
        })
    } else {
        Some(ProtectionAction::HighVoltageWarning {
            reason: beep_reason,
        })
    }
}

#[must_use]
fn select_bms_connection_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    if !context.signals.bms_connection_fault {
        return None;
    }
    Some(ProtectionAction::BmsConnection {
        target: directional_angle(
            state.serialized_config.high_voltage_pushback_angle(),
            context.phase.motor_erpm,
        ),
    })
}

#[must_use]
fn select_motor_temperature_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    let (reason, tiltback) = context.signals.motor_temperature_warning?;
    if tiltback {
        Some(ProtectionAction::MotorTemperaturePushback {
            reason,
            target: directional_angle(
                state.serialized_config.low_voltage_pushback_angle(),
                context.phase.motor_erpm,
            ),
        })
    } else {
        Some(ProtectionAction::MotorTemperatureWarning { reason })
    }
}

#[must_use]
fn select_bms_temperature_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    let reason = context.signals.bms_temperature_reason?;
    Some(ProtectionAction::BmsTemperaturePushback {
        reason,
        target: directional_angle(
            state.serialized_config.low_voltage_pushback_angle(),
            context.phase.motor_erpm,
        ),
    })
}

#[must_use]
fn select_low_voltage_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    let signals = context.signals;
    if context.base.motor().duty_cycle().ratio().as_ratio() <= 0.05
        || !(signals.bms_cell_under_voltage
            || signals.battery_voltage < signals.low_voltage_threshold)
    {
        return None;
    }
    let beep_reason = if signals.bms_cell_under_voltage {
        FloatOutBoyBeepReason::CellLowVoltage
    } else {
        FloatOutBoyBeepReason::LowVoltage
    };
    let voltage_delta = signals.low_voltage_threshold - signals.battery_voltage;
    let motor_current = context
        .base
        .motor()
        .directional_motor_current()
        .current()
        .abs();
    let tiltback = voltage_delta > Voltage::from_volts(2.0)
        || motor_current < Current::from_amps(5.0)
        || voltage_delta.as_volts() * 20.0 / motor_current.as_amps() > 1.0
        || signals.bms_cell_under_voltage;
    if tiltback {
        Some(ProtectionAction::LowVoltagePushback {
            reason: beep_reason,
            target: directional_angle(
                state.serialized_config.low_voltage_pushback_angle(),
                context.phase.motor_erpm,
            ),
        })
    } else {
        Some(ProtectionAction::LowVoltageWarning {
            reason: beep_reason,
        })
    }
}

#[must_use]
fn select_speed_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
) -> Option<ProtectionAction> {
    let speed = context.base.motor().vehicle_speed().speed();
    let threshold = state.serialized_config.speed_pushback_threshold();
    if !threshold.is_positive() || speed.abs() <= threshold {
        return None;
    }
    let target = if speed.is_positive() {
        state.runtime_duty_pushback_angle()
    } else {
        -state.runtime_duty_pushback_angle()
    };
    Some(ProtectionAction::SpeedPushback {
        target,
        step: state.runtime_duty_pushback_step(),
    })
}

fn return_protective_setpoint(state: &FloatOutBoyPackageState, control: &mut RunningControl) {
    if matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::PushbackDuty
            | FloatOutBoySetpointAdjustment::PushbackHighVoltage
            | FloatOutBoySetpointAdjustment::PushbackError
            | FloatOutBoySetpointAdjustment::PushbackLowVoltage
            | FloatOutBoySetpointAdjustment::PushbackSpeed
            | FloatOutBoySetpointAdjustment::PushbackTemperature
    ) {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
    }
    if !control.board_setpoint.is_zero() {
        control.board_setpoint = rate_limit_angle(
            control.board_setpoint,
            AngleDegrees::ZERO,
            state.runtime_tiltback_return_step(),
        );
    }
}

#[must_use]
fn select_protective_setpoint(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &RunningControl,
) -> Option<ProtectionAction> {
    if let Some(action) = select_duty_pushback(state, context, control) {
        return Some(action);
    }
    if let Some(action) = select_high_voltage_pushback(state, context) {
        return Some(action);
    }
    if let Some(action) = select_bms_connection_pushback(state, context) {
        return Some(action);
    }
    if let Some(action) = select_motor_temperature_pushback(state, context) {
        return Some(action);
    }
    if let Some(action) = select_bms_temperature_pushback(state, context) {
        return Some(action);
    }
    if let Some(action) = select_low_voltage_pushback(state, context) {
        return Some(action);
    }
    select_speed_pushback(state, context)
}

fn apply_protective_setpoint(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) {
    if let Some(action) = select_protective_setpoint(state, context, control) {
        apply_protection_action(action, control);
    } else {
        return_protective_setpoint(state, control);
    }
}

fn advance_runtime_setpoints(
    state: &mut FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    phase: &TransitionPhase,
    control: &RunningControl,
) -> FloatOutBoyRealtimeRuntimeSetpoints {
    let remote_setpoint = state.remote_control.update_input_tilt(
        state.serialized_config.input_tilt_angle_limit(),
        state.serialized_config.input_tilt_speed(),
        state.serialized_config.startup().sample_rate(),
        phase.control.darkride_active,
    );
    state.ride_modifiers.advance(
        &state.serialized_config,
        RideModifierInput {
            base_setpoint: control.board_setpoint,
            remote_setpoint,
            balance_pitch: phase.balance_pitch.angle_degrees(),
            motor_erpm: phase.motor_erpm,
            filtered_current: base.motor().filtered_motor_current().current().current(),
            motor_current: base.motor().motor_current(),
            acceleration: phase.motor_acceleration,
            darkride: phase.control.darkride_active,
            wheelslip: control.ride_state.wheelslip(),
        },
    )
}

fn refresh_duty_warning(state: &mut FloatOutBoyPackageState, control: &mut RunningControl) {
    if matches!(control.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        return;
    }
    let warning = matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::PushbackDuty
    ) && (state.serialized_config.duty_beep_enabled()
        || state.serialized_config.duty_pushback_angle().is_zero());
    if warning {
        state.force_beeper_on();
        state.beeper_flags.duty_warning_active = true;
        control.beep_reason = FloatOutBoyBeepReason::Duty;
    } else if state.beeper_flags.duty_warning_active {
        state.release_beeper();
    }
}

fn advance_balance_control(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: &FloatOutBoyAllDataBasePayload,
    phase: &TransitionPhase,
    control: &RunningControl,
    mut runtime: RuntimeValues,
) -> RuntimeValues {
    let gyro = imu.angular_rate();
    let mut loop_state = state.balance_loop;
    loop_state.balance_current = runtime.balance_current.current();
    loop_state.booster_current = runtime.booster_current.current();
    let balance_loop = loop_state.advance_balance_loop(
        state.runtime_balance_loop_config(),
        LoopInput {
            setpoint: runtime.setpoints.board(),
            brake_tilt_setpoint: runtime.setpoints.brake_tilt(),
            balance_pitch: phase.balance_pitch.angle_degrees(),
            raw_pitch: phase.pitch_degrees,
            roll: imu.roll(),
            gyro_pitch: gyro.pitch(),
            gyro_yaw: gyro.yaw(),
            motor_erpm: base.motor().electrical_speed(),
            motor_current: base.motor().motor_current(),
            motor_current_max: state.motor_current_max,
            motor_current_min: state.motor_current_min,
            mode: control.ride_state.mode(),
            darkride: control.ride_state.darkride(),
            traction_control: state.ride_flags.traction_control,
        },
    );
    state.balance_loop = balance_loop.state;
    runtime.booster_current =
        FloatOutBoyRealtimeBoosterCurrent::new(state.balance_loop.booster_current);
    runtime.balance_current =
        FloatOutBoyRealtimeBalanceCurrent::new(state.balance_loop.balance_current);
    state.request_motor_current(balance_loop.requested_current);
    runtime
}

fn advance_running_control(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    phase: &TransitionPhase,
    mut runtime: RuntimeValues,
) -> (RunningControl, RuntimeValues) {
    let signals = protection_signals(state, base);
    if signals.battery_voltage < signals.high_voltage_threshold && !signals.bms_cell_over_voltage {
        state.high_voltage_ticks = system_time_ticks;
    }
    let above_duty_limit =
        base.motor().duty_cycle().magnitude() > state.duty_max_with_margin.ratio();
    let mut control = RunningControl {
        ride_state: phase.ride_state,
        board_setpoint: state.runtime_board_setpoint,
        beep_reason: phase.beep_reason,
        beeper_alert: phase.beeper_alert,
    };
    enter_reverse_stop(
        state,
        system_time_ticks,
        phase.control.decision.enters_reverse_stop(),
        &mut control,
    );
    let wheelslip_branch = refresh_wheelslip_control(
        state,
        system_time_ticks,
        phase,
        above_duty_limit,
        &mut control,
    );
    refresh_centering(phase, &mut control);
    refresh_reverse_stop(state, phase, &mut control);
    if !matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) && !wheelslip_branch
        && !matches!(
            control.ride_state.wheelslip(),
            FloatOutBoyWheelSlipState::Detected
        )
    {
        apply_protective_setpoint(
            state,
            &ProtectionContext {
                base,
                phase,
                signals: &signals,
                system_time_ticks,
            },
            &mut control,
        );
    }
    if matches!(
        control.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::Detected
    ) && above_duty_limit
    {
        control.board_setpoint = AngleDegrees::ZERO;
    }
    state.runtime_board_setpoint = control.board_setpoint;
    runtime.setpoints = advance_runtime_setpoints(state, base, phase, &control);
    refresh_duty_warning(state, &mut control);
    runtime = advance_balance_control(state, imu, base, phase, &control, runtime);
    (control, runtime)
}

fn refresh_control_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    mut phase: TransitionPhase,
) -> (TransitionPhase, RuntimeValues) {
    let reset_runtime = phase.events.startup_became_ready || phase.events.effect.engaged();
    let mut runtime = runtime_values(
        state,
        base,
        phase.balance_pitch.angle_degrees(),
        reset_runtime,
    );

    if matches!(phase.run_state, FloatOutBoyRunState::Running)
        && !phase.events.effect.engaged()
        && !phase.events.effect.stopped()
    {
        let (control, next_runtime) =
            advance_running_control(state, imu, &base, system_time_ticks, &phase, runtime);
        phase.ride_state = control.ride_state;
        phase.beep_reason = control.beep_reason;
        phase.beeper_alert = control.beeper_alert;
        runtime = next_runtime;
    } else if matches!(phase.run_state, FloatOutBoyRunState::Ready)
        && !phase.events.effect.stopped()
        && let Some(current) = state.remote_control.request_ready_current(
            phase.motor_erpm,
            state.serialized_config.remote_throttle(),
            system_time_ticks,
            state.disengage_ticks,
        )
    {
        state.request_motor_current(current);
    }

    (phase, runtime)
}

/// Float Out Boy runtime refresh of IMU-derived state and control-loop faults.
///
/// C map: upstream `check_faults`, READY engage, startup reset, and traction
/// handling live in `third_party/float-out-boy/src/main.c:263-509`,
/// `third_party/float-out-boy/src/main.c:551-574`, `third_party/float-out-boy/src/main.c:760-775`,
/// `third_party/float-out-boy/src/main.c:833-838`, and `third_party/float-out-boy/src/main.c:957-1067`.
pub(super) fn refresh(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    system_time_ticks: TimestampTicks,
) -> bool {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let start = begin_refresh(state, base, imu.is_ready(), system_time_ticks);
    let phase = evaluate_transition_phase(state, imu, &base, system_time_ticks, &start);
    let (phase, runtime) = refresh_control_phase(state, imu, base, system_time_ticks, phase);

    #[cfg(any(test, target_arch = "arm"))]
    let mut phase = phase;
    #[cfg(any(test, target_arch = "arm"))]
    if let Some((reason, alert)) = refresh_ready_alert(
        state,
        base,
        phase.run_state,
        phase.ready_flywheel_stop,
        system_time_ticks,
    ) {
        phase.beep_reason = reason;
        phase.beeper_alert = Some(alert);
    }
    if let Some(alert) = phase.beeper_alert {
        state.alert_beeper(alert);
    }

    // C publishes the just-refreshed `imu.balance_pitch` through app-data;
    // normal mode comes from the balance filter at `third_party/float-out-boy/src/imu.c:35-41`,
    // while FLYWHEEL mirrors raw pitch at `third_party/float-out-boy/src/imu.c:56-58`.
    let base = FloatOutBoyAllDataBasePayload::new(
        runtime.balance_current,
        FloatOutBoyAllDataAttitude::new(phase.balance_pitch, phase.imu_roll, phase.imu_pitch),
        FloatOutBoyAllDataStatus::new(phase.ride_state, phase.beep_reason),
        base.footpad(),
        runtime.setpoints,
        runtime.booster_current,
        runtime.motor,
    );
    state.all_data_payloads = payloads.with_base(base);
    #[cfg(any(test, target_arch = "arm"))]
    {
        phase.ready_flywheel_stop
    }
    #[cfg(not(any(test, target_arch = "arm")))]
    {
        false
    }
}
