#![expect(
    clippy::struct_excessive_bools,
    reason = "independent runtime facts are flatter than private one-use wrapper structs"
)]

use super::BatteryVoltage;
use super::limits::{
    MOVING_FAULT_ROLL, REMOTE_SETPOINT_FAULT_ANGLE, darkride, push_start, quick_stop, reverse_stop,
    traction_loss,
};
use super::{
    AngleRadians, BatteryCellCount, Current, FloatOutBoyAllDataAttitude,
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataStatus, FloatOutBoyBeeperAlert,
    FloatOutBoyChargingState, FloatOutBoyDarkRideState, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyPackageState, FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStateTransitionInput, FloatOutBoyStopCondition, FloatOutBoyStopEvent,
    FloatOutBoyWheelSlipState, Imu, LoopInput, MotorCurrent, RideModifierInput, Rpm,
    TimestampTicks, float_out_boy_first_stop_event, float_out_boy_state_transition,
};
use crate::bms::FloatOutBoyBmsFaults;
use crate::domain::{FloatOutBoyBeepReason, FloatOutBoyRideState};
use crate::wire::saturating_trunc_f32_to_u8;
use vescpkg_rs::prelude::{
    AngleDegrees, DutyCycle, SignedRatio, Temperature, VescSeconds, Voltage,
};
use vescpkg_rs::{ImuPitch, ImuRoll};

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

pub(super) fn startup_ready_beep_count(warning_threshold: Voltage, battery_voltage: Voltage) -> u8 {
    let deficit = (warning_threshold - battery_voltage).as_volts();
    saturating_trunc_f32_to_u8(deficit).min(6).saturating_add(1)
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

    let reset_after_disengage = run_state == FloatOutBoyRunState::Ready
        && state.disengage_ticks.older_than_secs(system_time_ticks, 10);
    if !reset_after_disengage {
        return (ride_state, None);
    }

    // Float Out Boy removes the post-flip darkride grace after updating the
    // roll transition at `third_party/float-out-boy/src/main.c:781-794,984-992`.
    let alert = (ride_state.darkride() == FloatOutBoyDarkRideState::Active)
        .then_some(FloatOutBoyBeeperAlert::Long(1));
    state.upside_down_flags.enabled = false;
    (
        ride_state.with_darkride(FloatOutBoyDarkRideState::Upright),
        alert,
    )
}

fn refresh_ready_alert(
    state: &mut FloatOutBoyPackageState,
    base: FloatOutBoyAllDataBasePayload,
    run_state: FloatOutBoyRunState,
    ready_flywheel_stop: bool,
    system_time_ticks: TimestampTicks,
) -> Option<(FloatOutBoyBeepReason, FloatOutBoyBeeperAlert)> {
    if run_state != FloatOutBoyRunState::Ready || ready_flywheel_stop {
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
        alert = Some((reason, FloatOutBoyBeeperAlert::Short(4)));
    }

    // READY nags after 30 idle minutes, at most once per minute, and suppresses
    // the alert while pack voltage rises.
    if state.idle_ticks.older_than_secs(system_time_ticks, 1_800) {
        if state.nag_ticks.older_than_secs(system_time_ticks, 60) {
            state.nag_ticks.restart(system_time_ticks);
            let battery_voltage = base.motor().battery_voltage();
            if battery_voltage > state.idle_voltage {
                state.idle_voltage = battery_voltage;
            } else {
                alert = Some((FloatOutBoyBeepReason::Idle, FloatOutBoyBeeperAlert::Long(2)));
            }
        }
    } else {
        state.nag_ticks.restart(system_time_ticks);
        state.idle_voltage = BatteryVoltage::new(Voltage::ZERO);
    }
    alert
}

struct TransitionPhase {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    startup_became_ready: bool,
    state_engage: bool,
    state_stop_fault: bool,
    ready_flywheel_stop: bool,
    balance_pitch: FloatOutBoyRealtimeBalancePitch,
    pitch_degrees: AngleDegrees,
    imu_pitch: ImuPitch,
    imu_roll: ImuRoll,
    motor_erpm: Rpm,
    reverse_stop_entry_pending: bool,
    traction_loss_detected: bool,
    darkride_active: bool,
    motor_acceleration: Rpm,
    startup_centering_step: AngleDegrees,
}

impl TransitionPhase {
    fn set_adjustment(&mut self, adjustment: FloatOutBoySetpointAdjustment) {
        self.ride_state = self.ride_state.with_setpoint_adjustment(adjustment);
    }
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

struct FaultEvaluation {
    stop_event: Option<FloatOutBoyStopEvent>,
    full_switch_pending: bool,
    half_switch_pending: bool,
    roll_pending: bool,
    pitch_pending: bool,
    darkride_high_erpm_pending: bool,
    darkride_low_erpm_pending: bool,
    can_engage: bool,
    flywheel_footpad_pressed: bool,
}

#[expect(
    clippy::too_many_lines,
    reason = "one fault pass is smaller than a private one-use switch-fault handoff"
)]
fn evaluate_faults(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    input: &FaultInputs,
) -> FaultEvaluation {
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let footpad = base.footpad().state();
    let running = input.run_state == FloatOutBoyRunState::Running;
    let flywheel = input.ride_state.mode() == FloatOutBoyMode::Flywheel;
    let reverse_active = running
        && input.ride_state.setpoint_adjustment() == FloatOutBoySetpointAdjustment::ReverseStop;
    let flywheel_footpad = running && flywheel && footpad.is_pressed();
    let reverse_no_footpads = reverse_active && !footpad.is_pressed();
    let reverse_pitch =
        !input.darkride_active && reverse_active && input.pitch_abs > reverse_stop::PITCH;
    let reverse_timer = !input.darkride_active
        && reverse_active
        && ((input.pitch_abs > reverse_stop::TIMER_FAST_PITCH
            && state.reverse_ticks.older_than_secs(system_time_ticks, 1))
            || (input.pitch_abs > reverse_stop::TIMER_SLOW_PITCH
                && state.reverse_ticks.older_than_secs(system_time_ticks, 2)));
    let reverse_total = !input.darkride_active
        && reverse_active
        && state.reverse_total_erpm.abs() > reverse_stop::TOTAL_ERPM;

    let single_footpad = matches!(
        footpad,
        FloatOutBoyFootpadState::Left | FloatOutBoyFootpadState::Right
    );
    let dual_switch = faults.dual_switch();
    let simple_start = startup.simplestart_enabled()
        && (state.disengage_ticks.older_than_secs(system_time_ticks, 2)
            || !state.engage_ticks.older_than_secs(system_time_ticks, 1));
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
        && input.roll_abs < MOVING_FAULT_ROLL;
    let full_fault = full_pending
        && !switch_faults_disabled
        && (state
            .fault_switch_ticks
            .older_than(system_time_ticks, faults.switch_full_delay())
            || input.motor_erpm.abs() < half_erpm * 6.0
                && state
                    .fault_switch_ticks
                    .older_than(system_time_ticks, faults.switch_half_delay()));
    let half_pending = !input.darkride_active
        && running
        && !faults.dual_switch()
        && !can_engage
        && input.motor_erpm.abs() < half_erpm;
    let half_fault = half_pending
        && state
            .fault_switch_half_ticks
            .older_than(system_time_ticks, faults.switch_half_delay());
    let roll_pending = !input.darkride_active && running && input.roll_abs > faults.roll_angle();
    let roll_fault = roll_pending
        && state
            .fault_angle_roll_ticks
            .older_than(system_time_ticks, faults.roll_delay());
    let pitch_pending = running
        && input.pitch_abs > faults.pitch_angle()
        && input.remote_setpoint_abs < REMOTE_SETPOINT_FAULT_ANGLE;
    let pitch_fault = pitch_pending
        && state
            .fault_angle_pitch_ticks
            .older_than(system_time_ticks, faults.pitch_delay());
    let quickstop_fault = running
        && !footpad.is_pressed()
        && !flywheel
        && faults.quickstop_enabled()
        && input.motor_erpm.abs() < quick_stop::STOPPED_ERPM
        && input.pitch_abs > quick_stop::PITCH
        && input.remote_setpoint_abs < REMOTE_SETPOINT_FAULT_ANGLE
        && (input.pitch >= AngleRadians::ZERO) == (input.motor_erpm >= Rpm::ZERO);
    let darkride_roll = !input.darkride_active
        && running
        && matches!(
            input.ride_state.darkride(),
            FloatOutBoyDarkRideState::Upright
        )
        && faults.darkride_enabled()
        && input.roll_abs > darkride::ROLL_LOWER
        && input.roll_abs < darkride::ROLL_UPPER;

    let darkride_high_pending =
        input.darkride_active && input.motor_erpm > darkride::TIMED_HIGH_ERPM;
    // Active darkride shortens the wheelslip runaway stop from 100 ms to
    // 30 ms after the one-second post-flip grace (`src/main.c:361-366`).
    let darkride_wheelslip_fault = darkride_high_pending
        && input.ride_state.wheelslip() == FloatOutBoyWheelSlipState::Detected
        && state
            .upside_down_fault_ticks
            .older_than(system_time_ticks, VescSeconds::from_seconds(1.0))
        && state
            .fault_switch_ticks
            .older_than(system_time_ticks, VescSeconds::from_seconds(0.03));
    let darkride_high_fault = darkride_high_pending
        && (state
            .fault_switch_ticks
            .older_than(system_time_ticks, darkride::TIMED_HIGH_DELAY)
            || input.motor_erpm > darkride::HIGH_ERPM
            || darkride_wheelslip_fault);
    let darkride_low_pending = input.darkride_active
        && input.motor_erpm <= darkride::TIMED_HIGH_ERPM
        && input.motor_erpm > darkride::LOW_ERPM;
    let darkride_low_fault = darkride_low_pending
        && state
            .fault_angle_roll_ticks
            .older_than(system_time_ticks, darkride::LOW_DELAY);
    let stop_event = float_out_boy_first_stop_event(&[
        (FloatOutBoyStopEvent::FlywheelFootpad, flywheel_footpad),
        (
            FloatOutBoyStopEvent::ReverseStopNoFootpads,
            reverse_no_footpads,
        ),
        (FloatOutBoyStopEvent::ReverseStopPitch, reverse_pitch),
        (FloatOutBoyStopEvent::ReverseStopTimer, reverse_timer),
        (FloatOutBoyStopEvent::ReverseStopTotalErpm, reverse_total),
        (FloatOutBoyStopEvent::FullSwitch, full_fault),
        (FloatOutBoyStopEvent::QuickStop, quickstop_fault),
        (FloatOutBoyStopEvent::HalfSwitch, half_fault),
        (FloatOutBoyStopEvent::DarkrideHighErpm, darkride_high_fault),
        (FloatOutBoyStopEvent::DarkrideLowErpm, darkride_low_fault),
        (
            FloatOutBoyStopEvent::DarkrideCanEngage,
            input.darkride_active && can_engage,
        ),
        (FloatOutBoyStopEvent::Roll, roll_fault),
        (FloatOutBoyStopEvent::Pitch, pitch_fault),
        (FloatOutBoyStopEvent::DarkrideRoll, darkride_roll),
    ]);

    FaultEvaluation {
        stop_event,
        full_switch_pending: full_pending,
        half_switch_pending: half_pending,
        roll_pending,
        pitch_pending,
        darkride_high_erpm_pending: darkride_high_pending,
        darkride_low_erpm_pending: darkride_low_pending,
        can_engage,
        flywheel_footpad_pressed: flywheel_footpad,
    }
}

const DIRTY_LANDING_PITCH_MARGIN_DEGREES: u8 = 10;

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
    if run_state == FloatOutBoyRunState::Running {
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

    let (imu_pitch, imu_roll) = if ride_state.mode() == FloatOutBoyMode::Flywheel {
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
    let footpad_warning = run_state == FloatOutBoyRunState::Running
        && ride_state.mode() != FloatOutBoyMode::Flywheel
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
    let balance_pitch = if ride_state.mode() == FloatOutBoyMode::Flywheel {
        FloatOutBoyRealtimeBalancePitch::new(pitch)
    } else {
        FloatOutBoyRealtimeBalancePitch::new(state.balance_filter.pitch())
    };
    let ready_flywheel_stop = run_state == FloatOutBoyRunState::Ready
        && ride_state.mode() == FloatOutBoyMode::Flywheel
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
    let darkride_active = run_state == FloatOutBoyRunState::Running
        && ride_state.darkride() == FloatOutBoyDarkRideState::Active;
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
    let fault_evaluation = evaluate_faults(state, base, system_time_ticks, &fault_inputs);
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let dirty_landing_margin = matches!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchFull
    ) && startup.dirty_landings_enabled()
        && !state
            .fault_angle_pitch_ticks
            .older_than_secs(system_time_ticks, 1);
    let pitch_tolerance = startup.pitch_tolerance()
        + AngleDegrees::from_degrees(f32::from(
            u8::from(dirty_landing_margin).saturating_mul(DIRTY_LANDING_PITCH_MARGIN_DEGREES),
        ));
    let roll_tolerance = startup.roll_tolerance();
    let ready_engage = !startup_became_ready
        && run_state == FloatOutBoyRunState::Ready
        && !ready_flywheel_stop
        && fault_evaluation.can_engage
        && balance_pitch.angle_degrees().abs() < pitch_tolerance
        && roll_abs < roll_tolerance;
    let ready_darkride = !startup_became_ready
        && run_state == FloatOutBoyRunState::Ready
        && ride_state.darkride() == FloatOutBoyDarkRideState::Active
        && balance_pitch.angle_degrees().abs() < pitch_tolerance
        && {
            // READY darkride either ignores roll during its initial grace or
            // requires upside-down roll within startup tolerance.
            let within_grace = !state.disengage_ticks.older_than_secs(system_time_ticks, 1)
                && !matches!(
                    ride_state.stop_condition(),
                    FloatOutBoyStopCondition::ReverseStop
                );
            let upside_down = (roll_abs - AngleDegrees::from_degrees(180.0)).abs() < roll_tolerance;
            within_grace || upside_down
        };
    let ready_push_start = !startup_became_ready
        && run_state == FloatOutBoyRunState::Ready
        && startup.pushstart_enabled()
        && motor_erpm.abs() > push_start::ERPM_MIN
        && fault_evaluation.can_engage
        && balance_pitch.angle_degrees().abs() < push_start::ANGLE
        && roll_abs < push_start::ANGLE
        && !(faults.reversestop_enabled() && motor_erpm.is_negative());
    let state_engage = ready_engage || ready_darkride || ready_push_start;
    let startup_centering_step = startup.centering_step();
    let stop_event = fault_evaluation.stop_event;
    let reverse_stop_entry_pending = !ride_state
        .setpoint_adjustment()
        .is_centering_or_reverse_stop()
        && faults.reversestop_enabled()
        && motor_erpm < -reverse_stop::ENTRY_ERPM
        && !darkride_active;
    let motor_acceleration = state.motor_kinematics.average();
    let traction_loss_detected = stop_event.is_none()
        && !state_engage
        && !ride_state
            .setpoint_adjustment()
            .is_centering_or_reverse_stop()
        && !reverse_stop_entry_pending
        && run_state == FloatOutBoyRunState::Running
        && ride_state.mode() != FloatOutBoyMode::Flywheel
        && motor_acceleration.abs() > traction_loss::ACCELERATION_DETECT
        && motor_acceleration.is_negative() == motor_erpm.is_negative()
        && base.motor().duty_cycle().ratio() > traction_loss::DUTY
        && motor_erpm.abs() > traction_loss::ERPM;
    let transition = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        previous: ride_state,
        run_state,
        ready_flywheel_stop,
        state_engage,
        traction_loss_detected,
        stop_event,
    });
    if transition.state_stopped {
        state.play_motor_click();
        state.disengage_ticks.restart(system_time_ticks);
        state.trigger_data_recorder(false);
        if matches!(stop_event, Some(FloatOutBoyStopEvent::FullSwitch)) {
            state.fault_angle_pitch_ticks.restart(system_time_ticks);
        }
        state
            .flywheel
            .latch_abort(fault_evaluation.flywheel_footpad_pressed);
    } else if transition.state_engaged {
        state.play_motor_click();
        state.engage_ticks.restart(system_time_ticks);
        state.trigger_data_recorder(true);
    }
    if run_state == FloatOutBoyRunState::Running && !transition.state_stopped {
        state.upside_down_flags.enabled = true;
        if darkride_active && !state.upside_down_flags.started {
            state.upside_down_flags.started = true;
            state.upside_down_fault_ticks.restart(system_time_ticks);
        }
    }
    if !fault_evaluation.darkride_high_erpm_pending && !fault_evaluation.full_switch_pending {
        state.fault_switch_ticks.restart(system_time_ticks);
    }
    if !fault_evaluation.half_switch_pending {
        state.fault_switch_half_ticks.restart(system_time_ticks);
    }
    if run_state != FloatOutBoyRunState::Running
        || ride_state.setpoint_adjustment() != FloatOutBoySetpointAdjustment::ReverseStop
        || pitch_abs < reverse_stop::TIMER_SLOW_PITCH
    {
        state.reverse_ticks.restart(system_time_ticks);
    }
    if !fault_evaluation.darkride_low_erpm_pending && !fault_evaluation.roll_pending {
        state.fault_angle_roll_ticks.restart(system_time_ticks);
    }
    if !fault_evaluation.pitch_pending {
        state.fault_angle_pitch_ticks.restart(system_time_ticks);
    }

    TransitionPhase {
        ride_state: transition.ride_state,
        run_state,
        beep_reason,
        beeper_alert,
        startup_became_ready,
        state_engage,
        state_stop_fault: transition.state_stopped,
        ready_flywheel_stop,
        balance_pitch,
        pitch_degrees,
        imu_pitch,
        imu_roll,
        motor_erpm,
        reverse_stop_entry_pending,
        traction_loss_detected,
        darkride_active,
        motor_acceleration,
        startup_centering_step,
    }
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
    let bms_cell_over_voltage = state.bms.contains(FloatOutBoyBmsFaults::CELL_OVER_VOLTAGE);
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_cell_over_voltage = false;
    let bms_connection_fault = state.bms.contains(FloatOutBoyBmsFaults::CONNECTION);
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_connection_fault = false;
    let bms_temperature_reason = if state
        .bms
        .contains(FloatOutBoyBmsFaults::CELL_OVER_TEMPERATURE)
    {
        Some(FloatOutBoyBeepReason::CellOverTemperature)
    } else if state
        .bms
        .contains(FloatOutBoyBmsFaults::CELL_UNDER_TEMPERATURE)
    {
        Some(FloatOutBoyBeepReason::CellUnderTemperature)
    } else if state
        .bms
        .contains(FloatOutBoyBmsFaults::BMS_OVER_TEMPERATURE)
    {
        Some(FloatOutBoyBeepReason::BmsOverTemperature)
    } else {
        None
    };
    #[cfg(not(any(test, target_arch = "arm")))]
    let bms_temperature_reason = None;
    let bms_cell_under_voltage = state.bms.contains(FloatOutBoyBmsFaults::CELL_UNDER_VOLTAGE);
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
    phase: &mut TransitionPhase,
    signals: &ProtectionSignals,
    system_time_ticks: TimestampTicks,
    board_setpoint: &mut AngleDegrees,
) {
    let duty = base.motor().duty_cycle().ratio().as_ratio();
    if duty > state.runtime_duty_pushback_threshold().as_ratio() {
        if phase.ride_state.mode() != FloatOutBoyMode::Flywheel {
            phase.set_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);
        }
        *board_setpoint = vescpkg_rs::slew_toward(
            *board_setpoint,
            directional_angle(state.runtime_duty_pushback_angle(), phase.motor_erpm),
            state.runtime_duty_pushback_step(),
        );
        return;
    }
    if duty > 0.05
        && (signals.battery_voltage > signals.high_voltage_threshold
            || signals.bms_cell_over_voltage)
    {
        phase.beep_reason = if signals.bms_cell_over_voltage {
            FloatOutBoyBeepReason::CellHighVoltage
        } else {
            FloatOutBoyBeepReason::HighVoltage
        };
        phase.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(3));
        let tiltback = state
            .high_voltage_ticks
            .older_than(system_time_ticks, VescSeconds::from_seconds(0.5))
            || signals.battery_voltage > signals.high_voltage_threshold + Voltage::from_volts(1.0)
            || signals.bms_cell_over_voltage;
        phase.set_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackHighVoltage
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        if tiltback {
            *board_setpoint = directional_angle(
                state.serialized_config.high_voltage_pushback_angle(),
                phase.motor_erpm,
            );
        }
        return;
    }
    if signals.bms_connection_fault {
        phase.beep_reason = FloatOutBoyBeepReason::BmsConnection;
        phase.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(3));
        phase.set_adjustment(FloatOutBoySetpointAdjustment::PushbackError);
        *board_setpoint = directional_angle(
            state.serialized_config.high_voltage_pushback_angle(),
            phase.motor_erpm,
        );
        return;
    }
    if let Some((reason, tiltback)) = signals.motor_temperature_warning {
        phase.beep_reason = reason;
        phase.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(3));
        phase.set_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackTemperature
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        if tiltback {
            *board_setpoint = directional_angle(
                state.serialized_config.low_voltage_pushback_angle(),
                phase.motor_erpm,
            );
        }
        return;
    }
    if let Some(reason) = signals.bms_temperature_reason {
        phase.beep_reason = reason;
        phase.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(3));
        phase.set_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
        *board_setpoint = directional_angle(
            state.serialized_config.low_voltage_pushback_angle(),
            phase.motor_erpm,
        );
        return;
    }
    if duty > 0.05
        && (signals.bms_cell_under_voltage
            || signals.battery_voltage < signals.low_voltage_threshold)
    {
        phase.beep_reason = if signals.bms_cell_under_voltage {
            FloatOutBoyBeepReason::CellLowVoltage
        } else {
            FloatOutBoyBeepReason::LowVoltage
        };
        phase.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(3));
        let voltage_delta = signals.low_voltage_threshold - signals.battery_voltage;
        let motor_current = base.motor().directional_motor_current().current().abs();
        let tiltback = voltage_delta > Voltage::from_volts(2.0)
            || motor_current < Current::from_amps(5.0)
            || voltage_delta.as_volts() * 20.0 / motor_current.as_amps() > 1.0
            || signals.bms_cell_under_voltage;
        phase.set_adjustment(if tiltback {
            FloatOutBoySetpointAdjustment::PushbackLowVoltage
        } else {
            FloatOutBoySetpointAdjustment::None
        });
        *board_setpoint = if tiltback {
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
        phase.beep_reason = FloatOutBoyBeepReason::Speed;
        phase.set_adjustment(FloatOutBoySetpointAdjustment::PushbackSpeed);
        let target = if speed.is_positive() {
            state.runtime_duty_pushback_angle()
        } else {
            -state.runtime_duty_pushback_angle()
        };
        *board_setpoint =
            vescpkg_rs::slew_toward(*board_setpoint, target, state.runtime_duty_pushback_step());
        return;
    }
    if phase.ride_state.setpoint_adjustment().is_pushback() {
        phase.set_adjustment(FloatOutBoySetpointAdjustment::None);
    }
    if !board_setpoint.is_zero() {
        *board_setpoint = vescpkg_rs::slew_toward(
            *board_setpoint,
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
    base: &mut FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    phase: &mut TransitionPhase,
) {
    let signals = protection_signals(state, base);
    if signals.battery_voltage < signals.high_voltage_threshold && !signals.bms_cell_over_voltage {
        state.high_voltage_ticks.restart(system_time_ticks);
    }
    let above_duty_limit =
        base.motor().duty_cycle().magnitude() > state.duty_max_with_margin.ratio();
    let mut board_setpoint = state.runtime_board_setpoint;
    if phase.reverse_stop_entry_pending {
        state.reverse_total_erpm = if matches!(
            phase.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::PushbackHighVoltage
                | FloatOutBoySetpointAdjustment::PushbackLowVoltage
                | FloatOutBoySetpointAdjustment::PushbackTemperature
        ) {
            reverse_stop::carryover_total_erpm(board_setpoint)
        } else {
            Rpm::ZERO
        };
        state.reverse_ticks.restart(system_time_ticks);
        phase.set_adjustment(FloatOutBoySetpointAdjustment::ReverseStop);
    }

    let wheelslip_branch = if phase.traction_loss_detected {
        state.wheelslip_ticks.restart(system_time_ticks);
        if phase.darkride_active {
            state.ride_flags.traction_control = true;
        }
        true
    } else if phase.ride_state.wheelslip() == FloatOutBoyWheelSlipState::Detected
        && !phase
            .ride_state
            .setpoint_adjustment()
            .is_centering_or_reverse_stop()
    {
        if phase.motor_acceleration.abs() < traction_loss::ACCELERATION_CLEAR {
            state.ride_flags.traction_control = false;
        }
        if above_duty_limit {
            state.wheelslip_ticks.restart(system_time_ticks);
        } else if state
            .wheelslip_ticks
            .older_than(system_time_ticks, traction_loss::CLEAR_DELAY)
            && state.motor_duty_raw < traction_loss::RAW_DUTY_CLEAR
        {
            state.ride_flags.traction_control = false;
            phase.ride_state = phase
                .ride_state
                .with_wheelslip(FloatOutBoyWheelSlipState::None);
        }
        true
    } else {
        false
    };

    if matches!(
        phase.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    ) {
        if board_setpoint.is_zero() {
            phase.set_adjustment(FloatOutBoySetpointAdjustment::None);
        } else if board_setpoint.abs() < phase.startup_centering_step {
            board_setpoint = AngleDegrees::ZERO;
        } else {
            board_setpoint =
                board_setpoint - phase.startup_centering_step * board_setpoint.signum();
        }
    }

    if !phase.reverse_stop_entry_pending
        && phase.ride_state.setpoint_adjustment() == FloatOutBoySetpointAdjustment::ReverseStop
    {
        state.reverse_total_erpm = state.reverse_total_erpm + phase.motor_erpm;
        let total = state.reverse_total_erpm.abs();
        let setpoint = if total > reverse_stop::TOLERANCE_ERPM {
            Some(reverse_stop::target_angle(state.reverse_total_erpm))
        } else if total <= reverse_stop::TOLERANCE_ERPM * 0.5 && !phase.motor_erpm.is_negative() {
            state.reverse_total_erpm = Rpm::ZERO;
            phase.set_adjustment(FloatOutBoySetpointAdjustment::None);
            Some(AngleDegrees::ZERO)
        } else {
            None
        };
        if let Some(setpoint) = setpoint {
            board_setpoint = setpoint;
        }
    }
    if !phase
        .ride_state
        .setpoint_adjustment()
        .is_centering_or_reverse_stop()
        && !wheelslip_branch
        && phase.ride_state.wheelslip() != FloatOutBoyWheelSlipState::Detected
    {
        apply_protective_setpoint(
            state,
            base,
            phase,
            &signals,
            system_time_ticks,
            &mut board_setpoint,
        );
    }
    if phase.ride_state.wheelslip() == FloatOutBoyWheelSlipState::Detected && above_duty_limit {
        board_setpoint = AngleDegrees::ZERO;
    }
    state.runtime_board_setpoint = board_setpoint;
    let remote_setpoint = state.remote_control.update_input_tilt(
        state.serialized_config.input_tilt_angle_limit(),
        state.serialized_config.input_tilt_speed(),
        state.serialized_config.startup().sample_rate(),
        phase.darkride_active,
    );
    let setpoints = state.ride_modifiers.advance(
        &state.serialized_config,
        RideModifierInput {
            base_setpoint: board_setpoint,
            remote_setpoint,
            balance_pitch: phase.balance_pitch.angle_degrees(),
            motor_erpm: phase.motor_erpm,
            filtered_current: base.motor().filtered_motor_current().current().current(),
            motor_current: base.motor().motor_current(),
            acceleration: phase.motor_acceleration,
            darkride: phase.darkride_active,
            wheelslip: phase.ride_state.wheelslip(),
        },
    );
    *base = base.with_setpoints(setpoints);
    if phase.ride_state.mode() != FloatOutBoyMode::Flywheel {
        let warning = matches!(
            phase.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::PushbackDuty
        ) && (state.serialized_config.duty_beep_enabled()
            || state.serialized_config.duty_pushback_angle().is_zero());
        if warning {
            state.force_beeper_on();
            state.beeper_flags.duty_warning_active = true;
            phase.beep_reason = FloatOutBoyBeepReason::Duty;
        } else if state.beeper_flags.duty_warning_active {
            state.release_beeper();
        }
    }

    let gyro = imu.angular_rate();
    let mut loop_state = state.balance_loop;
    loop_state.balance_current = base.balance_current().current();
    loop_state.booster_current = base.booster_current().current();
    let balance_loop = loop_state.advance_balance_loop(
        state.runtime_balance_loop_config(),
        LoopInput {
            setpoint: base.setpoints().board(),
            brake_tilt_setpoint: base.setpoints().brake_tilt(),
            balance_pitch: phase.balance_pitch.angle_degrees(),
            raw_pitch: phase.pitch_degrees,
            roll: imu.roll(),
            gyro_pitch: gyro.pitch(),
            gyro_yaw: gyro.yaw(),
            motor_erpm: base.motor().electrical_speed(),
            motor_current: base.motor().motor_current(),
            motor_current_max: state.motor_current_max,
            motor_current_min: state.motor_current_min,
            mode: phase.ride_state.mode(),
            darkride: phase.ride_state.darkride(),
            traction_control: state.ride_flags.traction_control,
        },
    );
    state.balance_loop = balance_loop.state;
    *base = base
        .with_booster_current(FloatOutBoyRealtimeBoosterCurrent::new(
            state.balance_loop.booster_current,
        ))
        .with_balance_current(FloatOutBoyRealtimeBalanceCurrent::new(
            state.balance_loop.balance_current,
        ));
    state.request_motor_current(balance_loop.requested_current);
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
    let mut base = payloads.base();
    let mut phase = evaluate_transition_phase(state, imu, &base, system_time_ticks);
    let reset_runtime = phase.startup_became_ready || phase.state_engage;
    if reset_runtime {
        // Upstream `reset_runtime_vars` clears control-loop history and seeds only
        // the board setpoint from the current balance pitch.
        state.balance_loop.reset_pid();
        state.balance_loop.softstart_pid_limit = MotorCurrent::new(Current::ZERO);
        state.reverse_total_erpm = Rpm::ZERO;
        state.motor_kinematics.reset_acceleration();
        state.motor_current_filter.reset();
        state.ride_flags.traction_control = false;
        state.remote_control.reset_runtime_vars();
        state.ride_modifiers.reset();
        let balance_pitch = phase.balance_pitch.angle_degrees();
        state.runtime_board_setpoint = balance_pitch;
        let board_setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(balance_pitch);
        base = base
            .with_balance_current(FloatOutBoyRealtimeBalanceCurrent::default())
            .with_setpoints(
                FloatOutBoyRealtimeRuntimeSetpoints::default().with_board(board_setpoint),
            )
            .with_booster_current(FloatOutBoyRealtimeBoosterCurrent::default())
            .with_motor(
                base.motor()
                    .with_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0))),
            );
    }

    if phase.run_state == FloatOutBoyRunState::Running
        && !phase.state_engage
        && !phase.state_stop_fault
    {
        advance_running_control(state, imu, &mut base, system_time_ticks, &mut phase);
    } else if phase.run_state == FloatOutBoyRunState::Ready
        && !phase.state_stop_fault
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
    base = base
        .with_attitude(FloatOutBoyAllDataAttitude::new(
            phase.balance_pitch,
            phase.imu_roll,
            phase.imu_pitch,
        ))
        .with_status(FloatOutBoyAllDataStatus::new(
            phase.ride_state,
            phase.beep_reason,
        ));
    state.all_data_payloads = payloads.with_base(base);
    {
        phase.ready_flywheel_stop
    }
    #[cfg(not(any(test, target_arch = "arm")))]
    {
        false
    }
}
