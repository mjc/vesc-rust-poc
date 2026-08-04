use super::BatteryVoltage;
use super::limits::{
    DarkrideLimits, MovingFaultLimits, PushStartLimits, QuickStopLimits, RemoteSetpointFaultLimit,
    ReverseStopLimits, TractionLossLimits,
};
use super::transition::{
    FloatOutBoyEngagementDecision, FloatOutBoyStateTransitionInput, FloatOutBoyStopEvent,
    float_out_boy_state_transition,
};
use super::{
    AngleRadians, BatteryCellCount, Current, DataRecorderTrigger, FloatOutBoyAllDataAttitude,
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataStatus, FloatOutBoyBeeperAlert,
    FloatOutBoyBeeperCount, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyPackageState,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStopCondition, FloatOutBoyTractionControlState, FloatOutBoyWheelSlipState, Imu,
    LoopInput, MotorCurrent, RideModifierInput, Rpm, TimestampTicks, float_out_boy_ticks_elapsed,
    float_out_boy_ticks_elapsed_seconds,
};
use crate::bms::FloatOutBoyBmsFault;
use crate::domain::{FloatOutBoyAllDataMotorPayload, FloatOutBoyBeepReason, FloatOutBoyRideState};
#[cfg(test)]
use vescpkg_rs::prelude::SystemTicks;
use vescpkg_rs::prelude::{
    AngleDegrees, DutyCycle, SignedRatio, Temperature, VescSeconds, Voltage,
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
    state_engage: bool,
    state_stop_fault: bool,
}

struct ControlConditions {
    reverse_stop_entry_pending: bool,
    traction_loss_detected: bool,
    darkride_active: bool,
}

struct TransitionPhase {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    events: TransitionEvents,
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

struct FaultInputs {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    pitch: AngleRadians,
    pitch_abs: AngleDegrees,
    roll_abs: AngleDegrees,
    remote_setpoint_abs: AngleDegrees,
    motor_erpm: Rpm,
    darkride_active: bool,
}

struct SwitchFaultActivity {
    full: bool,
    half: bool,
}

struct AngleFaultActivity {
    roll: bool,
    pitch: bool,
}

struct NormalFaultEvaluation {
    conditions: [bool; 11],
    switches: SwitchFaultActivity,
    angles: AngleFaultActivity,
    can_engage: bool,
    flywheel_footpad_pressed: bool,
}

#[expect(
    clippy::too_many_lines,
    reason = "one fault pass is smaller than a private one-use switch-fault handoff"
)]
fn evaluate_normal_faults(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    input: &FaultInputs,
) -> NormalFaultEvaluation {
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    let footpad = base.footpad().state();
    let running = matches!(input.run_state, FloatOutBoyRunState::Running);
    let flywheel = matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel);
    let reverse_active = running
        && matches!(
            input.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::ReverseStop
        );
    let flywheel_footpad = running && flywheel && footpad.is_pressed();
    let reverse_no_footpads = reverse_active && !footpad.is_pressed();
    let reverse_pitch =
        !input.darkride_active && reverse_active && input.pitch_abs > reverse_stop.pitch;
    let reverse_timer = !input.darkride_active
        && reverse_active
        && ((input.pitch_abs > reverse_stop.timer_fast_pitch
            && float_out_boy_ticks_elapsed(system_time_ticks, state.reverse_ticks, 1))
            || (input.pitch_abs > reverse_stop.timer_slow_pitch
                && float_out_boy_ticks_elapsed(system_time_ticks, state.reverse_ticks, 2)));
    let reverse_total = !input.darkride_active
        && reverse_active
        && state.reverse_total_erpm.abs() > reverse_stop.total_erpm;

    let single_footpad = matches!(
        footpad,
        FloatOutBoyFootpadState::Left | FloatOutBoyFootpadState::Right
    );
    let dual_switch = faults.dual_switch();
    let simple_start = startup.simplestart_enabled()
        && (float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 2)
            || !float_out_boy_ticks_elapsed(system_time_ticks, state.engage_ticks, 1));
    let can_engage = matches!(
        input.ride_state.charging(),
        FloatOutBoyChargingState::NotCharging
    ) && (matches!(footpad, FloatOutBoyFootpadState::Both)
        || single_footpad && (dual_switch || simple_start)
        || flywheel);
    let half_erpm = faults.adc_half_erpm().rpm();
    let full_pending = !input.darkride_active && running && !footpad.is_pressed() && !flywheel;
    let switch_faults_disabled = faults.moving_faults_disabled()
        && input.motor_erpm > half_erpm * 2.0
        && input.roll_abs < MovingFaultLimits::FLOAT_OUT_BOY.roll;
    let full_fault = full_pending
        && !switch_faults_disabled
        && (float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            faults.switch_full_delay(),
        ) || input.motor_erpm.abs() < half_erpm * 6.0
            && float_out_boy_ticks_elapsed_seconds(
                system_time_ticks,
                state.fault_switch_ticks,
                faults.switch_half_delay(),
            ));
    let half_pending = !input.darkride_active
        && running
        && !faults.dual_switch()
        && !can_engage
        && input.motor_erpm.abs() < half_erpm;
    let half_fault = half_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_half_ticks,
            faults.switch_half_delay(),
        );
    let roll_pending = !input.darkride_active && running && input.roll_abs > faults.roll_angle();
    let roll_fault = roll_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            faults.roll_delay(),
        );
    let pitch_pending = running
        && input.pitch_abs > faults.pitch_angle()
        && input.remote_setpoint_abs < RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle();
    let pitch_fault = pitch_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_pitch_ticks,
            faults.pitch_delay(),
        );
    let quickstop = QuickStopLimits::FLOAT_OUT_BOY;
    let quickstop_fault = running
        && !footpad.is_pressed()
        && !flywheel
        && faults.quickstop_enabled()
        && input.motor_erpm.abs() < quickstop.stopped_erpm
        && input.pitch_abs > quickstop.pitch
        && input.remote_setpoint_abs < RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle()
        && (input.pitch >= AngleRadians::ZERO) == (input.motor_erpm >= Rpm::ZERO);
    let darkride = DarkrideLimits::FLOAT_OUT_BOY;
    let darkride_roll = !input.darkride_active
        && running
        && matches!(
            input.ride_state.darkride(),
            FloatOutBoyDarkRideState::Upright
        )
        && faults.darkride_enabled()
        && input.roll_abs > darkride.roll_lower
        && input.roll_abs < darkride.roll_upper;

    NormalFaultEvaluation {
        conditions: [
            flywheel_footpad,
            reverse_no_footpads,
            reverse_pitch,
            reverse_timer,
            reverse_total,
            full_fault,
            quickstop_fault,
            half_fault,
            roll_fault,
            pitch_fault,
            darkride_roll,
        ],
        switches: SwitchFaultActivity {
            full: full_pending,
            half: half_pending,
        },
        angles: AngleFaultActivity {
            roll: roll_pending,
            pitch: pitch_pending,
        },
        can_engage,
        flywheel_footpad_pressed: flywheel_footpad,
    }
}

struct DarkrideFaultEvaluation {
    conditions: [bool; 3],
    high_erpm_pending: bool,
    low_erpm_pending: bool,
}

fn evaluate_darkride_faults(
    state: &FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    input: &FaultInputs,
    can_engage: bool,
) -> DarkrideFaultEvaluation {
    let limits = DarkrideLimits::FLOAT_OUT_BOY;
    let high_pending = input.darkride_active && input.motor_erpm > limits.timed_high_erpm;
    // Active darkride shortens the wheelslip runaway stop from 100 ms to
    // 30 ms after the one-second post-flip grace (`src/main.c:361-366`).
    let wheelslip_fault = high_pending
        && matches!(
            input.ride_state.wheelslip(),
            FloatOutBoyWheelSlipState::Detected
        )
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.upside_down_fault_ticks,
            VescSeconds::from_seconds(1.0),
        )
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            VescSeconds::from_seconds(0.03),
        );
    let high_fault = high_pending
        && (float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            limits.timed_high_delay,
        ) || input.motor_erpm > limits.high_erpm
            || wheelslip_fault);
    let low_pending = input.darkride_active
        && input.motor_erpm <= limits.timed_high_erpm
        && input.motor_erpm > limits.low_erpm;
    let low_fault = low_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            limits.low_delay,
        );
    DarkrideFaultEvaluation {
        conditions: [high_fault, low_fault, input.darkride_active && can_engage],
        high_erpm_pending: high_pending,
        low_erpm_pending: low_pending,
    }
}

const DIRTY_LANDING_PITCH_MARGIN_DEGREES: u8 = 10;

#[cfg(test)]
pub(super) struct ActiveReverseStopFaultInput {
    pub(super) footpad: FloatOutBoyFootpadState,
    pub(super) darkride: FloatOutBoyDarkRideState,
    pub(super) pitch: AngleDegrees,
    pub(super) elapsed: SystemTicks,
    pub(super) total_erpm: Rpm,
}

#[cfg(test)]
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
        if self.pitch > limits.pitch {
            return Some(FloatOutBoyStopEvent::ReverseStopPitch);
        }
        let fast_timer_expired = self.pitch > limits.timer_fast_pitch
            && VescSeconds::from_seconds(1.0)
                .to_system_ticks_saturating()
                .is_some_and(|timeout| self.elapsed > timeout);
        let slow_timer_expired = self.pitch > limits.timer_slow_pitch
            && VescSeconds::from_seconds(2.0)
                .to_system_ticks_saturating()
                .is_some_and(|timeout| self.elapsed > timeout);
        if fast_timer_expired || slow_timer_expired {
            return Some(FloatOutBoyStopEvent::ReverseStopTimer);
        }
        (self.total_erpm.abs() > limits.total_erpm)
            .then_some(FloatOutBoyStopEvent::ReverseStopTotalErpm)
    }
}

#[must_use]
#[cfg(test)]
pub(super) fn reverse_stop_timer_inactive(pitch_abs: AngleDegrees) -> bool {
    pitch_abs <= ReverseStopLimits::FLOAT_OUT_BOY.timer_slow_pitch
}

fn first_transition_stop(
    normal: &NormalFaultEvaluation,
    darkride: &DarkrideFaultEvaluation,
) -> Option<FloatOutBoyStopEvent> {
    let [
        flywheel_footpad,
        reverse_no_footpads,
        reverse_pitch,
        reverse_timer,
        reverse_total,
        full_switch,
        quickstop,
        half_switch,
        roll,
        pitch,
        darkride_roll,
    ] = normal.conditions;
    let [darkride_high, darkride_low, darkride_can_engage] = darkride.conditions;
    [
        (FloatOutBoyStopEvent::FlywheelFootpad, flywheel_footpad),
        (
            FloatOutBoyStopEvent::ReverseStopNoFootpads,
            reverse_no_footpads,
        ),
        (FloatOutBoyStopEvent::ReverseStopPitch, reverse_pitch),
        (FloatOutBoyStopEvent::ReverseStopTimer, reverse_timer),
        (FloatOutBoyStopEvent::ReverseStopTotalErpm, reverse_total),
        (FloatOutBoyStopEvent::FullSwitch, full_switch),
        (FloatOutBoyStopEvent::QuickStop, quickstop),
        (FloatOutBoyStopEvent::HalfSwitch, half_switch),
        (FloatOutBoyStopEvent::DarkrideHighErpm, darkride_high),
        (FloatOutBoyStopEvent::DarkrideLowErpm, darkride_low),
        (FloatOutBoyStopEvent::DarkrideCanEngage, darkride_can_engage),
        (FloatOutBoyStopEvent::Roll, roll),
        (FloatOutBoyStopEvent::Pitch, pitch),
        (FloatOutBoyStopEvent::DarkrideRoll, darkride_roll),
    ]
    .into_iter()
    .find_map(|(event, active)| active.then_some(event))
}

#[expect(
    clippy::too_many_lines,
    reason = "one transition pass avoids four private one-use handoff structs"
)]
fn evaluate_transition_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
) -> TransitionPhase {
    let status = base.status();
    let mut ride_state = status.ride_state();
    let startup_became_ready =
        matches!(ride_state.run_state(), FloatOutBoyRunState::Startup) && imu.is_ready();
    let mut run_state = if startup_became_ready {
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
        let warning_threshold = pack_voltage_threshold(
            state.serialized_config.low_voltage_threshold(),
            state.battery_cell_count,
        ) + Voltage::from_volts(5.0);
        let battery_voltage = base.motor().battery_voltage().voltage();
        if battery_voltage < warning_threshold {
            beep_reason = FloatOutBoyBeepReason::LowBattery;
        }
        beeper_alert = Some(FloatOutBoyBeeperAlert::Long(startup_ready_beep_count(
            warning_threshold,
            battery_voltage,
        )));
    }

    let (imu_pitch, imu_roll) = if matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
        let (pitch, roll) = state.flywheel_attitude(
            ride_state.mode(),
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
    let (next_ride_state, darkride_alert) =
        refresh_darkride_state(state, ride_state, run_state, roll_abs, system_time_ticks);
    ride_state = next_ride_state;
    beeper_alert = darkride_alert.or(beeper_alert);

    let motor_erpm = base.motor().electrical_speed().rpm();
    let switch_warning_erpm = if state.serialized_config.foot_beep_enabled() {
        Rpm::from_revolutions_per_minute(2_000.0)
    } else {
        Rpm::from_revolutions_per_minute(100_000.0)
    };
    let footpad_warning = matches!(run_state, FloatOutBoyRunState::Running)
        && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && !base.footpad().state().is_pressed()
        && motor_erpm.abs() > switch_warning_erpm;
    if footpad_warning {
        state.force_beeper_on();
        beep_reason = FloatOutBoyBeepReason::Sensors;
    } else {
        state.release_beeper();
    }

    // Float Out Boy normally uses its balance filter, while FLYWHEEL uses raw
    // pitch (`src/imu.c:35-41,56-58`).
    let balance_pitch = if matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
        FloatOutBoyRealtimeBalancePitch::new(pitch)
    } else {
        state.balance_filter.balance_pitch()
    };
    let ready_flywheel_stop = matches!(run_state, FloatOutBoyRunState::Ready)
        && matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && state
            .flywheel
            .should_stop(base.footpad().state().is_pressed());
    if ready_flywheel_stop {
        state.prepare_flywheel_restore();
        run_state = state
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
    }
    let darkride_active = matches!(
        (run_state, ride_state.darkride()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyDarkRideState::Active
        )
    );
    let fault_inputs = FaultInputs {
        ride_state,
        run_state,
        pitch,
        pitch_abs,
        roll_abs,
        remote_setpoint_abs: base.setpoints().remote().angle().abs(),
        motor_erpm,
        darkride_active,
    };
    let normal = evaluate_normal_faults(state, base, system_time_ticks, &fault_inputs);
    let darkride =
        evaluate_darkride_faults(state, system_time_ticks, &fault_inputs, normal.can_engage);
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let dirty_landing_margin = matches!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchFull
    ) && startup.dirty_landings_enabled()
        && !float_out_boy_ticks_elapsed(system_time_ticks, state.fault_angle_pitch_ticks, 1);
    let pitch_tolerance = startup.pitch_tolerance()
        + AngleDegrees::from_degrees(f32::from(
            u8::from(dirty_landing_margin).saturating_mul(DIRTY_LANDING_PITCH_MARGIN_DEGREES),
        ));
    let roll_tolerance = startup.roll_tolerance();
    let ready_engage = !startup_became_ready
        && matches!(run_state, FloatOutBoyRunState::Ready)
        && !ready_flywheel_stop
        && normal.can_engage
        && balance_pitch.angle_degrees().abs() < pitch_tolerance
        && roll_abs < roll_tolerance;
    let ready_darkride = !startup_became_ready
        && matches!(
            (run_state, ride_state.darkride()),
            (FloatOutBoyRunState::Ready, FloatOutBoyDarkRideState::Active)
        )
        && balance_pitch.angle_degrees().abs() < pitch_tolerance
        && {
            // READY darkride either ignores roll during its initial grace or
            // requires upside-down roll within startup tolerance.
            let within_grace =
                !float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 1)
                    && !matches!(
                        ride_state.stop_condition(),
                        FloatOutBoyStopCondition::ReverseStop
                    );
            let upside_down = (roll_abs - AngleDegrees::from_degrees(180.0)).abs() < roll_tolerance;
            within_grace || upside_down
        };
    let push_start = PushStartLimits::FLOAT_OUT_BOY;
    let ready_push_start = !startup_became_ready
        && matches!(run_state, FloatOutBoyRunState::Ready)
        && startup.pushstart_enabled()
        && motor_erpm.abs() > push_start.erpm_min
        && normal.can_engage
        && balance_pitch.angle_degrees().abs() < push_start.angle
        && roll_abs < push_start.angle
        && !(faults.reversestop_enabled() && motor_erpm.is_negative());
    let state_engage = ready_engage || ready_darkride || ready_push_start;
    let startup_centering_step = startup.centering_step();
    let stop_event = first_transition_stop(&normal, &darkride);
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    let reverse_stop_entry_pending = !matches!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) && faults.reversestop_enabled()
        && motor_erpm < -reverse_stop.entry_erpm
        && !darkride_active;
    let motor_acceleration = state.motor_kinematics.average();
    let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
    let traction_loss_detected = stop_event.is_none()
        && !state_engage
        && !matches!(
            ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
        )
        && !reverse_stop_entry_pending
        && matches!(run_state, FloatOutBoyRunState::Running)
        && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && motor_acceleration.abs() > traction_loss.acceleration_detect
        && motor_acceleration.is_negative() == motor_erpm.is_negative()
        && base.motor().duty_cycle().ratio() > traction_loss.duty
        && motor_erpm.abs() > traction_loss.erpm;
    let transition = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        previous: ride_state,
        run_state,
        ready_flywheel_stop,
        engagement: if state_engage {
            FloatOutBoyEngagementDecision::Engage
        } else {
            FloatOutBoyEngagementDecision::Preserve
        },
        traction_loss_detected,
        stop_event,
    });
    if transition.effect.stopped() {
        state.play_motor_click();
        state.disengage_ticks = system_time_ticks;
        state.trigger_data_recorder(DataRecorderTrigger::Disengage);
        if matches!(stop_event, Some(FloatOutBoyStopEvent::FullSwitch)) {
            state.fault_angle_pitch_ticks = system_time_ticks;
        }
        state.flywheel.latch_abort(normal.flywheel_footpad_pressed);
    } else if transition.effect.engaged() {
        state.play_motor_click();
        state.engage_ticks = system_time_ticks;
        state.trigger_data_recorder(DataRecorderTrigger::Engage);
    }
    if matches!(run_state, FloatOutBoyRunState::Running) && !transition.effect.stopped() {
        state.upside_down_flags.enabled = true;
        if darkride_active && !state.upside_down_flags.started {
            state.upside_down_flags.started = true;
            state.upside_down_fault_ticks = system_time_ticks;
        }
    }
    if !darkride.high_erpm_pending && !normal.switches.full {
        state.fault_switch_ticks = system_time_ticks;
    }
    if !normal.switches.half {
        state.fault_switch_half_ticks = system_time_ticks;
    }
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    if !matches!(
        (run_state, ride_state.setpoint_adjustment()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    ) || pitch_abs < reverse_stop.timer_slow_pitch
    {
        state.reverse_ticks = system_time_ticks;
    }
    if !darkride.low_erpm_pending && !normal.angles.roll {
        state.fault_angle_roll_ticks = system_time_ticks;
    }
    if !normal.angles.pitch {
        state.fault_angle_pitch_ticks = system_time_ticks;
    }

    TransitionPhase {
        ride_state: transition.ride_state,
        run_state,
        beep_reason,
        beeper_alert,
        events: TransitionEvents {
            startup_became_ready,
            state_engage,
            state_stop_fault: transition.effect.stopped(),
        },
        ready_flywheel_stop,
        balance_pitch,
        pitch_degrees,
        imu_pitch,
        imu_roll,
        motor_erpm,
        control: ControlConditions {
            reverse_stop_entry_pending,
            traction_loss_detected,
            darkride_active,
        },
        motor_acceleration,
        startup_centering_step,
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
    let bms_cell_over_voltage = state.bms.contains(FloatOutBoyBmsFault::CellOverVoltage);
    let bms_connection_fault = state.bms.contains(FloatOutBoyBmsFault::Connection);
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
    let bms_cell_under_voltage = state.bms.contains(FloatOutBoyBmsFault::CellUnderVoltage);
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

fn directional_angle(angle: AngleDegrees, motor_erpm: Rpm) -> AngleDegrees {
    if motor_erpm.is_positive() {
        angle
    } else {
        -angle
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered dispatcher preserves priority without seven one-use boolean helpers"
)]
fn apply_protective_setpoint(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    phase: &TransitionPhase,
    signals: &ProtectionSignals,
    system_time_ticks: TimestampTicks,
    control: &mut RunningControl,
) {
    let duty = base.motor().duty_cycle().ratio().as_ratio();
    if duty > state.runtime_duty_pushback_threshold().as_ratio() {
        if !matches!(control.ride_state.mode(), FloatOutBoyMode::Flywheel) {
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);
        }
        control.board_setpoint = rate_limit_angle(
            control.board_setpoint,
            directional_angle(state.runtime_duty_pushback_angle(), phase.motor_erpm),
            state.runtime_duty_pushback_step(),
        );
        return;
    }
    if duty > 0.05
        && (signals.battery_voltage > signals.high_voltage_threshold
            || signals.bms_cell_over_voltage)
    {
        control.beep_reason = if signals.bms_cell_over_voltage {
            FloatOutBoyBeepReason::CellHighVoltage
        } else {
            FloatOutBoyBeepReason::HighVoltage
        };
        control.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
        let tiltback = float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.high_voltage_ticks,
            VescSeconds::from_seconds(0.5),
        ) || signals.battery_voltage
            > signals.high_voltage_threshold + Voltage::from_volts(1.0)
            || signals.bms_cell_over_voltage;
        control.ride_state = control.ride_state.with_setpoint_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackHighVoltage
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        if tiltback {
            control.board_setpoint = directional_angle(
                state.serialized_config.high_voltage_pushback_angle(),
                phase.motor_erpm,
            );
        }
        return;
    }
    if signals.bms_connection_fault {
        control.beep_reason = FloatOutBoyBeepReason::BmsConnection;
        control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackError);
        control.board_setpoint = directional_angle(
            state.serialized_config.high_voltage_pushback_angle(),
            phase.motor_erpm,
        );
        return;
    }
    if let Some((reason, tiltback)) = signals.motor_temperature_warning {
        control.beep_reason = reason;
        control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
        control.ride_state = control.ride_state.with_setpoint_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackTemperature
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        if tiltback {
            control.board_setpoint = directional_angle(
                state.serialized_config.low_voltage_pushback_angle(),
                phase.motor_erpm,
            );
        }
        return;
    }
    if let Some(reason) = signals.bms_temperature_reason {
        control.beep_reason = reason;
        control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
        control.board_setpoint = directional_angle(
            state.serialized_config.low_voltage_pushback_angle(),
            phase.motor_erpm,
        );
        return;
    }
    if duty > 0.05
        && (signals.bms_cell_under_voltage
            || signals.battery_voltage < signals.low_voltage_threshold)
    {
        control.beep_reason = if signals.bms_cell_under_voltage {
            FloatOutBoyBeepReason::CellLowVoltage
        } else {
            FloatOutBoyBeepReason::LowVoltage
        };
        control.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
        let voltage_delta = signals.low_voltage_threshold - signals.battery_voltage;
        let motor_current = base.motor().directional_motor_current().current().abs();
        let tiltback = voltage_delta > Voltage::from_volts(2.0)
            || motor_current < Current::from_amps(5.0)
            || voltage_delta.as_volts() * 20.0 / motor_current.as_amps() > 1.0
            || signals.bms_cell_under_voltage;
        control.ride_state = control.ride_state.with_setpoint_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackLowVoltage
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        control.board_setpoint = if tiltback {
            directional_angle(
                state.serialized_config.low_voltage_pushback_angle(),
                phase.motor_erpm,
            )
        } else {
            AngleDegrees::ZERO
        };
        return;
    }
    let speed = base.motor().vehicle_speed().speed();
    let threshold = state.serialized_config.speed_pushback_threshold();
    if threshold.is_positive() && speed.abs() > threshold {
        control.beep_reason = FloatOutBoyBeepReason::Speed;
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackSpeed);
        let target = if speed.is_positive() {
            state.runtime_duty_pushback_angle()
        } else {
            -state.runtime_duty_pushback_angle()
        };
        control.board_setpoint = rate_limit_angle(
            control.board_setpoint,
            target,
            state.runtime_duty_pushback_step(),
        );
        return;
    }
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

#[expect(
    clippy::too_many_lines,
    reason = "one ordered control pass is smaller than three private one-use handoffs"
)]
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
    if phase.control.reverse_stop_entry_pending {
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

    let wheelslip_branch = if phase.control.traction_loss_detected {
        state.wheelslip_ticks = system_time_ticks;
        if phase.control.darkride_active {
            state.ride_flags.traction_control = FloatOutBoyTractionControlState::Freewheeling;
        }
        true
    } else if matches!(
        control.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::Detected
    ) && !matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) {
        let limits = TractionLossLimits::FLOAT_OUT_BOY;
        if phase.motor_acceleration.abs() < limits.acceleration_clear {
            state.ride_flags.traction_control = FloatOutBoyTractionControlState::FilteringCurrent;
        }
        if above_duty_limit {
            state.wheelslip_ticks = system_time_ticks;
        } else if float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.wheelslip_ticks,
            limits.clear_delay,
        ) && state.motor_duty_raw < limits.raw_duty_clear
        {
            state.ride_flags.traction_control = FloatOutBoyTractionControlState::FilteringCurrent;
            control.ride_state = control
                .ride_state
                .with_wheelslip(FloatOutBoyWheelSlipState::None);
        }
        true
    } else {
        false
    };

    if matches!(
        control.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    ) {
        if control.board_setpoint.is_zero() {
            control.ride_state = control
                .ride_state
                .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        } else if control.board_setpoint.abs() < phase.startup_centering_step {
            control.board_setpoint = AngleDegrees::ZERO;
        } else {
            control.board_setpoint = control.board_setpoint
                - phase.startup_centering_step * control.board_setpoint.signum();
        }
    }

    if !phase.control.reverse_stop_entry_pending
        && matches!(
            control.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    {
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
            base,
            phase,
            &signals,
            system_time_ticks,
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
    let remote_setpoint = state.remote_control.update_input_tilt(
        state.serialized_config.input_tilt_angle_limit(),
        state.serialized_config.input_tilt_speed(),
        state.serialized_config.startup().sample_rate(),
        phase.control.darkride_active,
    );
    runtime.setpoints = state.ride_modifiers.advance(
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
    );
    if !matches!(control.ride_state.mode(), FloatOutBoyMode::Flywheel) {
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
    (control, runtime)
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
    let mut phase = evaluate_transition_phase(state, imu, &base, system_time_ticks);
    let reset_runtime = phase.events.startup_became_ready || phase.events.state_engage;
    let mut runtime = if reset_runtime {
        // Upstream `reset_runtime_vars` clears control-loop history and seeds only
        // the board setpoint from the current balance pitch.
        state.balance_loop.reset_pid();
        state.balance_loop.softstart_pid_limit = MotorCurrent::new(Current::ZERO);
        state.reverse_total_erpm = Rpm::ZERO;
        state.motor_kinematics.reset_acceleration();
        state.motor_current_filter.reset_runtime();
        state.ride_flags.traction_control = FloatOutBoyTractionControlState::FilteringCurrent;
        state.remote_control.reset_runtime_vars();
        state.ride_modifiers.reset();
        let balance_pitch = phase.balance_pitch.angle_degrees();
        state.runtime_board_setpoint = balance_pitch;
        let board_setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(balance_pitch);
        let zero_setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::ZERO);
        RuntimeValues {
            balance_current: FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(
                Current::ZERO,
            )),
            setpoints: FloatOutBoyRealtimeRuntimeSetpoints::new(
                board_setpoint,
                zero_setpoint,
                zero_setpoint,
                zero_setpoint,
                zero_setpoint,
                zero_setpoint,
            ),
            booster_current: FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(
                Current::ZERO,
            )),
            motor: base
                .motor()
                .with_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0))),
        }
    } else {
        RuntimeValues {
            balance_current: base.balance_current(),
            setpoints: base.setpoints(),
            booster_current: base.booster_current(),
            motor: base.motor(),
        }
    };

    if matches!(phase.run_state, FloatOutBoyRunState::Running)
        && !phase.events.state_engage
        && !phase.events.state_stop_fault
    {
        let (control, next_runtime) =
            advance_running_control(state, imu, &base, system_time_ticks, &phase, runtime);
        phase.ride_state = control.ride_state;
        phase.beep_reason = control.beep_reason;
        phase.beeper_alert = control.beeper_alert;
        runtime = next_runtime;
    } else if matches!(phase.run_state, FloatOutBoyRunState::Ready)
        && !phase.events.state_stop_fault
        && let Some(current) = state.remote_control.request_ready_current(
            phase.motor_erpm,
            state.serialized_config.remote_throttle(),
            system_time_ticks,
            state.disengage_ticks,
        )
    {
        state.request_motor_current(current);
    }

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
    phase.ready_flywheel_stop
}
