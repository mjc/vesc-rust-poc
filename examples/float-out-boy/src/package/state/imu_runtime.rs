#[cfg(any(test, target_arch = "arm"))]
use super::BatteryVoltage;
use super::limits::{
    DarkrideLimits, MovingFaultLimits, PushStartLimits, QuickStopLimits, RemoteSetpointFaultLimit,
    ReverseStopLimits, TractionLossLimits,
};
use super::{
    AngleRadians, BatteryCellCount, Current, FloatOutBoyAllDataAttitude,
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount, FloatOutBoyChargingState,
    FloatOutBoyDarkRideState, FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyPackageState,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStateTransitionInput, FloatOutBoyStopCondition, FloatOutBoyStopEvent,
    FloatOutBoyWheelSlipState, Imu, LoopInput, MotorCurrent, RideModifierInput, Rpm,
    TimestampTicks, float_out_boy_first_stop_event, float_out_boy_state_transition,
    float_out_boy_ticks_elapsed, float_out_boy_ticks_elapsed_seconds,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::FloatOutBoyBmsFault;
use crate::domain::{FloatOutBoyBeepReason, FloatOutBoyRideState};
use vescpkg_rs::prelude::{AngleDegrees, Temperature, VescSeconds, Voltage};
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
        // `time_update` refreshes Float Out Boy's idle timer on every RUNNING loop
        // at `third_party/float-out-boy/src/time.c:38-43`.
        state.idle_ticks = system_time_ticks;
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
                if state.upside_down_enabled && roll_abs > AngleDegrees::from_degrees(150.0) =>
            {
                ride_state = ride_state.with_darkride(FloatOutBoyDarkRideState::Active);
                state.upside_down_started = false;
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
    state.upside_down_enabled = false;
    (
        ride_state.with_darkride(FloatOutBoyDarkRideState::Upright),
        alert,
    )
}

struct RuntimeValues {
    balance_current: FloatOutBoyRealtimeBalanceCurrent,
    setpoints: FloatOutBoyRealtimeRuntimeSetpoints,
    booster_current: FloatOutBoyRealtimeBoosterCurrent,
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
        };
    }

    // Upstream `reset_runtime_vars` clears control-loop history and seeds only
    // the board setpoint from the current balance pitch.
    state.balance_loop.reset_pid();
    state.balance_loop.softstart_pid_limit = MotorCurrent::new(Current::ZERO);
    state.reverse_total_erpm = Rpm::ZERO;
    state.traction_control = false;
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

    let connection_fault = state.bms_faults.contains(FloatOutBoyBmsFault::Connection);
    let balance_fault = state.bms_faults.contains(FloatOutBoyBmsFault::CellBalance)
        && float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 5);
    let mut alert = None;
    if (connection_fault || balance_fault)
        && float_out_boy_ticks_elapsed(system_time_ticks, state.bms_alert_ticks, 15)
    {
        state.bms_alert_ticks = system_time_ticks;
        let reason = if connection_fault {
            FloatOutBoyBeepReason::BmsConnection
        } else {
            FloatOutBoyBeepReason::CellBalance
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

struct TransitionPhase {
    ride_state: FloatOutBoyRideState,
    run_state: FloatOutBoyRunState,
    beep_reason: FloatOutBoyBeepReason,
    beeper_alert: Option<FloatOutBoyBeeperAlert>,
    startup_became_ready: bool,
    state_engage: bool,
    state_stop_fault: bool,
    #[cfg(any(test, target_arch = "arm"))]
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

fn evaluate_transition_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    start: RefreshStart,
) -> TransitionPhase {
    let RefreshStart {
        mut ride_state,
        mut run_state,
        mut beep_reason,
        mut beeper_alert,
        startup_became_ready,
    } = start;
    let flywheel_both_footpads_fault = matches!(
        (run_state, ride_state.mode(), base.footpad().state()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Flywheel,
            FloatOutBoyFootpadState::Both
        )
    );
    let reverse_stop_no_footpads_fault = matches!(
        (
            run_state,
            ride_state.setpoint_adjustment(),
            base.footpad().state()
        ),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoySetpointAdjustment::ReverseStop,
            FloatOutBoyFootpadState::None
        )
    );
    let reverse_stop_active = matches!(
        (run_state, ride_state.setpoint_adjustment()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    );
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
    let roll = imu_roll.angle();
    let roll_degrees = AngleDegrees::from(roll);
    let roll_abs = roll_degrees.abs();
    state
        .ride_modifiers
        .aggregate_yaw(AngleDegrees::from(imu.yaw().angle()));
    let (next_ride_state, darkride_alert) =
        refresh_darkride_state(state, ride_state, run_state, roll_abs, system_time_ticks);
    ride_state = next_ride_state;
    if darkride_alert.is_some() {
        beeper_alert = darkride_alert;
    }
    let remote_setpoint_abs = base.setpoints().remote().angle().abs();
    let quickstop = QuickStopLimits::FLOAT_OUT_BOY;
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    let remote_setpoint_fault = RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle();
    let moving_fault = MovingFaultLimits::FLOAT_OUT_BOY;
    let darkride = DarkrideLimits::FLOAT_OUT_BOY;
    let push_start = PushStartLimits::FLOAT_OUT_BOY;
    let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
    // C map: `check_faults(d)` has a dedicated darkride branch at
    // `third_party/float-out-boy/src/main.c:359-390`; normal switch/reverse/roll
    // faults only run in the `else` branch at `third_party/float-out-boy/src/main.c:392-491`.
    let darkride_active = matches!(
        (run_state, ride_state.darkride()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyDarkRideState::Active
        )
    );
    let reverse_stop_pitch_fault =
        !darkride_active && reverse_stop_active && pitch_abs > reverse_stop.pitch;
    let reverse_stop_timer_fault = !darkride_active
        && matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                FloatOutBoyRunState::Running,
                FloatOutBoySetpointAdjustment::ReverseStop
            )
        )
        && {
            (pitch_abs > reverse_stop.timer_fast_pitch
                && float_out_boy_ticks_elapsed(system_time_ticks, state.reverse_ticks, 1))
                || (pitch_abs > reverse_stop.timer_slow_pitch
                    && float_out_boy_ticks_elapsed(system_time_ticks, state.reverse_ticks, 2))
        };
    let reverse_stop_total_erpm_fault = !darkride_active
        && matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                FloatOutBoyRunState::Running,
                FloatOutBoySetpointAdjustment::ReverseStop
            )
        )
        && state.reverse_total_erpm.abs() > reverse_stop.total_erpm;
    let motor_erpm = base.motor().electrical_speed().rpm();
    let switch_warning_erpm = if state.serialized_config.foot_beep_enabled() {
        Rpm::from_revolutions_per_minute(2_000.0)
    } else {
        Rpm::from_revolutions_per_minute(100_000.0)
    };
    let footpad_warning = matches!(run_state, FloatOutBoyRunState::Running)
        && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && matches!(base.footpad().state(), FloatOutBoyFootpadState::None)
        && motor_erpm.abs() > switch_warning_erpm;
    if footpad_warning {
        state.force_beeper_on();
        beep_reason = FloatOutBoyBeepReason::Sensors;
    } else {
        state.release_beeper();
    }
    // C updates `imu.balance_pitch` from the Float Out Boy-owned balance filter
    // before control at `third_party/float-out-boy/src/main.c:760-775`, `third_party/float-out-boy/src/imu.c:35-41`, and
    // `third_party/float-out-boy/src/balance_filter.c:145-154`; FLYWHEEL then overrides it with raw
    // pitch at `third_party/float-out-boy/src/imu.c:56-58`.
    let balance_pitch = if matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
        FloatOutBoyRealtimeBalancePitch::new(pitch)
    } else {
        state.balance_filter.balance_pitch()
    };
    let balance_pitch_degrees = balance_pitch.angle_degrees();
    let balance_pitch_abs = balance_pitch_degrees.abs();
    let ready_flywheel_stop = matches!(run_state, FloatOutBoyRunState::Ready)
        && matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        && (state.flywheel_abort
            || matches!(base.footpad().state(), FloatOutBoyFootpadState::Both));
    if ready_flywheel_stop {
        state.restore_flywheel_config();
        run_state = state
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
    }
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let quickstop_fault = matches!(
        (run_state, base.footpad().state(), ride_state.mode()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyFootpadState::None,
            mode
        ) if !matches!(mode, FloatOutBoyMode::Flywheel)
    ) && faults.quickstop_enabled()
        && motor_erpm.abs() < quickstop.stopped_erpm
        && pitch_abs > quickstop.pitch
        && remote_setpoint_abs < remote_setpoint_fault
        && (pitch >= AngleRadians::ZERO) == (motor_erpm >= Rpm::ZERO);
    let single_footpad = matches!(
        base.footpad().state(),
        FloatOutBoyFootpadState::Left | FloatOutBoyFootpadState::Right
    );
    let dual_switch = faults.dual_switch();
    let simple_start = startup.simplestart_enabled()
        && (float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 2)
            || !float_out_boy_ticks_elapsed(system_time_ticks, state.engage_ticks, 1));
    let can_engage = matches!(ride_state.charging(), FloatOutBoyChargingState::NotCharging)
        && (matches!(base.footpad().state(), FloatOutBoyFootpadState::Both)
            || single_footpad && (dual_switch || simple_start)
            || matches!(ride_state.mode(), FloatOutBoyMode::Flywheel));
    let fault_adc_half_erpm = faults.adc_half_erpm().rpm();
    let fault_delay_switch_half = faults.switch_half_delay();
    let fault_delay_switch_full = faults.switch_full_delay();
    let switch_faults_disabled = faults.moving_faults_disabled()
        && motor_erpm > fault_adc_half_erpm * 2.0
        && roll_abs < moving_fault.roll;
    let full_switch_pending = !darkride_active
        && matches!(run_state, FloatOutBoyRunState::Running)
        && matches!(base.footpad().state(), FloatOutBoyFootpadState::None)
        && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel);
    let full_switch_fault = full_switch_pending
        && !switch_faults_disabled
        && (float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            fault_delay_switch_full,
        ) || motor_erpm.abs() < fault_adc_half_erpm * 6.0
            && float_out_boy_ticks_elapsed_seconds(
                system_time_ticks,
                state.fault_switch_ticks,
                fault_delay_switch_half,
            ));
    let half_switch_pending = !darkride_active
        && matches!(run_state, FloatOutBoyRunState::Running)
        && !dual_switch
        && !can_engage
        && motor_erpm.abs() < fault_adc_half_erpm;
    let half_switch_fault = half_switch_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_half_ticks,
            fault_delay_switch_half,
        );
    let fault_roll = faults.roll_angle();
    let fault_delay_roll = faults.roll_delay();
    let roll_fault_pending = !darkride_active
        && matches!(run_state, FloatOutBoyRunState::Running)
        && roll_abs > fault_roll;
    let roll_fault = roll_fault_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            fault_delay_roll,
        );
    let fault_pitch = faults.pitch_angle();
    let fault_delay_pitch = faults.pitch_delay();
    let pitch_fault_pending = matches!(run_state, FloatOutBoyRunState::Running)
        && pitch_abs > fault_pitch
        && remote_setpoint_abs < remote_setpoint_fault;
    let pitch_fault = pitch_fault_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_pitch_ticks,
            fault_delay_pitch,
        );
    let darkride_high_erpm_pending = darkride_active && motor_erpm > darkride.timed_high_erpm;
    // C map: after the one-second post-flip grace, active darkride shortens
    // the wheelslip runaway stop from 100 ms to 30 ms at
    // `third_party/float-out-boy/src/main.c:361-366`.
    let darkride_wheelslip_fault = darkride_high_erpm_pending
        && matches!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::Detected)
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
    let darkride_high_erpm_fault = darkride_high_erpm_pending
        && (float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            darkride.timed_high_delay,
        ) || motor_erpm > darkride.high_erpm
            || darkride_wheelslip_fault);
    let darkride_low_erpm_pending =
        darkride_active && motor_erpm <= darkride.timed_high_erpm && motor_erpm > darkride.low_erpm;
    let darkride_low_erpm_fault = darkride_low_erpm_pending
        && float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            darkride.low_delay,
        );
    let darkride_can_engage_fault = darkride_active && can_engage;
    let darkride_roll_fault = !darkride_active
        && matches!(
            (run_state, ride_state.darkride()),
            (
                FloatOutBoyRunState::Running,
                FloatOutBoyDarkRideState::Upright
            )
        )
        && faults.darkride_enabled()
        && roll_abs > darkride.roll_lower
        && roll_abs < darkride.roll_upper;
    let startup_pitch_tolerance = startup.pitch_tolerance();
    let startup_roll_tolerance = startup.roll_tolerance();
    let startup_centering_step = startup.centering_step();
    let ready_engage = !startup_became_ready
        && matches!(run_state, FloatOutBoyRunState::Ready)
        && !ready_flywheel_stop
        && can_engage
        && balance_pitch_abs < startup_pitch_tolerance
        && roll_abs < startup_roll_tolerance;
    let ready_darkride_engage = !startup_became_ready
        && matches!(
            (run_state, ride_state.darkride()),
            (FloatOutBoyRunState::Ready, FloatOutBoyDarkRideState::Active)
        )
        && balance_pitch_abs < startup_pitch_tolerance
        && {
            // Upstream READY darkride startup either ignores roll for the
            // first second unless the previous stop was reverse-stop, or
            // after that requires upside-down roll within startup tolerance
            // at `third_party/float-out-boy/src/main.c:1038-1054`.
            let within_darkride_grace =
                !float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 1)
                    && !matches!(
                        ride_state.stop_condition(),
                        FloatOutBoyStopCondition::ReverseStop
                    );
            let roll_near_upside_down =
                (roll_abs - AngleDegrees::from_degrees(180.0)).abs() < startup_roll_tolerance;

            within_darkride_grace || roll_near_upside_down
        };
    let ready_push_start = !startup_became_ready
        && matches!(run_state, FloatOutBoyRunState::Ready)
        && startup.pushstart_enabled()
        && motor_erpm.abs() > push_start.erpm_min
        && can_engage
        && balance_pitch_abs < push_start.angle
        && roll_abs < push_start.angle
        && !(faults.reversestop_enabled() && motor_erpm.is_negative());
    let state_engage = ready_engage || ready_darkride_engage || ready_push_start;
    // Upstream `check_faults(d)` returns immediately after each stop branch
    // in `third_party/float-out-boy/src/main.c:357-509`; this call preserves the
    // same Rust condition priority before `state_stop` writes READY and
    // clears wheelslip at `third_party/float-out-boy/src/state.c:29-33`.
    let stop_event = float_out_boy_first_stop_event(&[
        (
            FloatOutBoyStopEvent::FlywheelBothFootpads,
            flywheel_both_footpads_fault,
        ),
        (
            FloatOutBoyStopEvent::ReverseStopNoFootpads,
            reverse_stop_no_footpads_fault,
        ),
        (
            FloatOutBoyStopEvent::ReverseStopPitch,
            reverse_stop_pitch_fault,
        ),
        (
            FloatOutBoyStopEvent::ReverseStopTimer,
            reverse_stop_timer_fault,
        ),
        (
            FloatOutBoyStopEvent::ReverseStopTotalErpm,
            reverse_stop_total_erpm_fault,
        ),
        (FloatOutBoyStopEvent::FullSwitch, full_switch_fault),
        (FloatOutBoyStopEvent::QuickStop, quickstop_fault),
        (FloatOutBoyStopEvent::HalfSwitch, half_switch_fault),
        // C map: darkride high-ERPM and low-ERPM branches both stop as
        // reverse-stop at `third_party/float-out-boy/src/main.c:360-379`.
        (
            FloatOutBoyStopEvent::DarkrideHighErpm,
            darkride_high_erpm_fault,
        ),
        (
            FloatOutBoyStopEvent::DarkrideLowErpm,
            darkride_low_erpm_fault,
        ),
        (
            FloatOutBoyStopEvent::DarkrideCanEngage,
            darkride_can_engage_fault,
        ),
        (FloatOutBoyStopEvent::Roll, roll_fault),
        (FloatOutBoyStopEvent::Pitch, pitch_fault),
        (FloatOutBoyStopEvent::DarkrideRoll, darkride_roll_fault),
    ]);
    let reverse_stop_entry_pending = !matches!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) && faults.reversestop_enabled()
        && motor_erpm < -reverse_stop.entry_erpm
        && !darkride_active;
    let motor_acceleration = state.motor_acceleration.average();
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
    let state_transition = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        previous: ride_state,
        run_state,
        ready_flywheel_stop,
        state_engage,
        traction_loss_detected,
        stop_event,
    });
    let state_stop_fault = state_transition.state_stopped;
    if state_transition.state_stopped {
        state.disengage_ticks = system_time_ticks;
        state.flywheel_abort |= flywheel_both_footpads_fault;
    } else if state_transition.state_engaged {
        state.engage_ticks = system_time_ticks;
    }
    if matches!(run_state, FloatOutBoyRunState::Running) && !state_stop_fault {
        // C map: a surviving RUNNING tick enables a later upside-down
        // transition, and the first active tick starts the runaway grace at
        // `third_party/float-out-boy/src/main.c:867,723-729`.
        state.upside_down_enabled = true;
        if darkride_active && !state.upside_down_started {
            state.upside_down_started = true;
            state.upside_down_fault_ticks = system_time_ticks;
        }
    }
    if !darkride_high_erpm_pending && !full_switch_pending {
        state.fault_switch_ticks = system_time_ticks;
    }
    if !half_switch_pending {
        state.fault_switch_half_ticks = system_time_ticks;
    }
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
    if !darkride_low_erpm_pending && !roll_fault_pending {
        state.fault_angle_roll_ticks = system_time_ticks;
    }
    if !pitch_fault_pending {
        state.fault_angle_pitch_ticks = system_time_ticks;
    }

    TransitionPhase {
        ride_state: state_transition.ride_state,
        run_state,
        beep_reason,
        beeper_alert,
        startup_became_ready,
        state_engage,
        state_stop_fault,
        #[cfg(any(test, target_arch = "arm"))]
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

fn refresh_control_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    mut phase: TransitionPhase,
) -> (TransitionPhase, RuntimeValues) {
    let run_state = phase.run_state;
    let state_engage = phase.state_engage;
    let state_stop_fault = phase.state_stop_fault;
    let startup_became_ready = phase.startup_became_ready;
    let balance_pitch = phase.balance_pitch;
    let balance_pitch_degrees = balance_pitch.angle_degrees();
    let pitch_degrees = phase.pitch_degrees;
    let motor_erpm = phase.motor_erpm;
    let reverse_stop_entry_pending = phase.reverse_stop_entry_pending;
    let traction_loss_detected = phase.traction_loss_detected;
    let darkride_active = phase.darkride_active;
    let motor_acceleration = phase.motor_acceleration;
    let startup_centering_step = phase.startup_centering_step;
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
    let mut beep_reason = phase.beep_reason;
    let mut beeper_alert = phase.beeper_alert;
    // Upstream READY engages at `third_party/float-out-boy/src/main.c:1033-1067`;
    // `state_engage` writes RUNNING/CENTERING/STOP_NONE at
    // `third_party/float-out-boy/src/state.c:36-39`; READY flywheel abort returns
    // to NORMAL before startup checks at `third_party/float-out-boy/src/main.c:957-963`.
    let mut ride_state = phase.ride_state;
    let reset_runtime_vars = startup_became_ready || state_engage;
    let RuntimeValues {
        mut balance_current,
        mut setpoints,
        mut booster_current,
    } = runtime_values(state, base, balance_pitch_degrees, reset_runtime_vars);
    if matches!(run_state, FloatOutBoyRunState::Running) && !state_engage && !state_stop_fault {
        let mut board_setpoint = state.runtime_board_setpoint;
        let high_voltage_threshold = pack_voltage_threshold(
            state.serialized_config.high_voltage_threshold(),
            state.battery_cell_count,
        );
        let low_voltage_threshold = pack_voltage_threshold(
            state.serialized_config.low_voltage_threshold(),
            state.battery_cell_count,
        );
        let battery_voltage = base.motor().battery_voltage().voltage();
        #[cfg(any(test, target_arch = "arm"))]
        let bms_cell_over_voltage = state
            .bms_faults
            .contains(FloatOutBoyBmsFault::CellOverVoltage);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_cell_over_voltage = false;
        #[cfg(any(test, target_arch = "arm"))]
        let bms_connection_fault = state.bms_faults.contains(FloatOutBoyBmsFault::Connection);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_connection_fault = false;
        #[cfg(any(test, target_arch = "arm"))]
        let bms_temperature_reason = if state
            .bms_faults
            .contains(FloatOutBoyBmsFault::CellOverTemperature)
        {
            Some(FloatOutBoyBeepReason::CellOverTemperature)
        } else if state
            .bms_faults
            .contains(FloatOutBoyBmsFault::CellUnderTemperature)
        {
            Some(FloatOutBoyBeepReason::CellUnderTemperature)
        } else if state
            .bms_faults
            .contains(FloatOutBoyBmsFault::BmsOverTemperature)
        {
            Some(FloatOutBoyBeepReason::BmsOverTemperature)
        } else {
            None
        };
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_temperature_reason: Option<FloatOutBoyBeepReason> = None;
        let temperature_warning_margin = Temperature::from_degrees_celsius(3.0);
        let temperature_tiltback_margin = Temperature::from_degrees_celsius(1.0);
        let mosfet_temperature_threshold =
            state.mosfet_temperature_limit_start.temperature() - temperature_warning_margin;
        let motor_temperature_threshold =
            state.motor_temperature_limit_start.temperature() - temperature_warning_margin;
        let motor_temperature_warning =
            if state.mosfet_temperature.temperature() > mosfet_temperature_threshold {
                Some((
                    FloatOutBoyBeepReason::MosfetTemperature,
                    state.mosfet_temperature.temperature()
                        > mosfet_temperature_threshold + temperature_tiltback_margin,
                ))
            } else if state.motor_temperature.temperature() > motor_temperature_threshold {
                Some((
                    FloatOutBoyBeepReason::MotorTemperature,
                    state.motor_temperature.temperature()
                        > motor_temperature_threshold + temperature_tiltback_margin,
                ))
            } else {
                None
            };
        #[cfg(any(test, target_arch = "arm"))]
        let bms_cell_under_voltage = state
            .bms_faults
            .contains(FloatOutBoyBmsFault::CellUnderVoltage);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_cell_under_voltage = false;
        // Float Out Boy refreshes this before every setpoint-adjustment branch at
        // `third_party/float-out-boy/src/main.c:512-518`, except while a cell
        // over-voltage fault is active.
        if battery_voltage < high_voltage_threshold && !bms_cell_over_voltage {
            state.high_voltage_ticks = system_time_ticks;
        }
        let motor_duty = base.motor().duty_cycle().magnitude();
        let above_wheelslip_duty_limit = motor_duty > state.duty_max_with_margin.ratio();
        let entered_reverse_stop = reverse_stop_entry_pending;
        if entered_reverse_stop {
            // Float Out Boy carries an existing HV/LV/temperature target into
            // reverse-stop at `third_party/float-out-boy/src/main.c:538-550`.
            state.reverse_total_erpm = if matches!(
                ride_state.setpoint_adjustment(),
                FloatOutBoySetpointAdjustment::PushbackHighVoltage
                    | FloatOutBoySetpointAdjustment::PushbackLowVoltage
                    | FloatOutBoySetpointAdjustment::PushbackTemperature
            ) {
                reverse_stop.carryover_total_erpm(board_setpoint)
            } else {
                Rpm::ZERO
            };
            state.reverse_ticks = system_time_ticks;
            ride_state =
                ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::ReverseStop);
        }
        // C map: these are the detection and active-wheelslip branches in
        // `calculate_setpoint_target` at `third_party/float-out-boy/src/main.c:551-575`.
        let wheelslip_branch_active = if traction_loss_detected {
            state.wheelslip_ticks = system_time_ticks;
            if darkride_active {
                state.traction_control = true;
            }
            true
        } else if matches!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::Detected)
            && !matches!(
                ride_state.setpoint_adjustment(),
                FloatOutBoySetpointAdjustment::Centering
                    | FloatOutBoySetpointAdjustment::ReverseStop
            )
        {
            if motor_acceleration.abs() < traction_loss.acceleration_clear {
                state.traction_control = false;
            }
            if above_wheelslip_duty_limit {
                state.wheelslip_ticks = system_time_ticks;
            } else if float_out_boy_ticks_elapsed_seconds(
                system_time_ticks,
                state.wheelslip_ticks,
                traction_loss.clear_delay,
            ) && state.motor_duty_raw < traction_loss.raw_duty_clear
            {
                state.traction_control = false;
                ride_state = ride_state.with_wheelslip(FloatOutBoyWheelSlipState::None);
            }
            true
        } else {
            false
        };
        if matches!(
            ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::Centering
        ) {
            if board_setpoint.is_zero() {
                // Upstream `calculate_setpoint_target(d)` exits
                // `SAT_CENTERING` when `setpoint_target_interpolated`
                // already equals target zero at
                // `third_party/float-out-boy/src/main.c:517-520`.
                ride_state =
                    ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
            } else {
                board_setpoint = if board_setpoint.abs() < startup_centering_step {
                    AngleDegrees::ZERO
                } else {
                    board_setpoint - startup_centering_step * board_setpoint.signum()
                };
                // Upstream stores `startup_speed / hertz` at
                // `third_party/float-out-boy/src/main.c:172`, selects it for
                // `SAT_CENTERING` at `third_party/float-out-boy/src/main.c:304-310`,
                // applies `rate_limitf` at
                // `third_party/float-out-boy/src/utils.c:25-33`, and assigns the
                // centered setpoint before PID at
                // `third_party/float-out-boy/src/main.c:869-875`.
            }
        }
        if matches!(
            ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::ReverseStop
        ) && !entered_reverse_stop
        {
            // Upstream `calculate_setpoint_target(d)` accumulates ERPM, grows
            // the nose-down target past tolerance, and exits below half
            // tolerance while moving forward at
            // `third_party/float-out-boy/src/main.c:522-536`.
            state.reverse_total_erpm = state.reverse_total_erpm + motor_erpm;
            let reverse_total_erpm = state.reverse_total_erpm.abs();
            let reverse_setpoint = if reverse_total_erpm > reverse_stop.tolerance_erpm {
                Some(reverse_stop.target_angle(state.reverse_total_erpm))
            } else if reverse_total_erpm <= reverse_stop.tolerance_erpm * 0.5
                && !motor_erpm.is_negative()
            {
                state.reverse_total_erpm = Rpm::ZERO;
                ride_state =
                    ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
                Some(AngleDegrees::ZERO)
            } else {
                None
            };
            if let Some(reverse_setpoint) = reverse_setpoint {
                board_setpoint = reverse_setpoint;
            }
        }
        if !matches!(
            ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
        ) && !wheelslip_branch_active
            && !matches!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::Detected)
        {
            let duty_pushback_active = base.motor().duty_cycle().ratio().as_ratio()
                > state.runtime_duty_pushback_threshold().as_ratio();
            let voltage_pushback_duty = base.motor().duty_cycle().ratio().as_ratio() > 0.05;
            let speed = base.motor().vehicle_speed().speed();
            let speed_pushback_threshold = state.serialized_config.speed_pushback_threshold();
            let speed_pushback_active =
                speed_pushback_threshold.is_positive() && speed.abs() > speed_pushback_threshold;
            let protective_setpoint = if duty_pushback_active {
                let angle = state.runtime_duty_pushback_angle();
                if !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
                    ride_state = ride_state
                        .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);
                }
                let target = if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                };
                Some(rate_limit_angle(
                    board_setpoint,
                    target,
                    state.runtime_duty_pushback_step(),
                ))
            } else if base.motor().duty_cycle().ratio().as_ratio() > 0.05
                && (battery_voltage > high_voltage_threshold || bms_cell_over_voltage)
            {
                beep_reason = if bms_cell_over_voltage {
                    FloatOutBoyBeepReason::CellHighVoltage
                } else {
                    FloatOutBoyBeepReason::HighVoltage
                };
                beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
                if float_out_boy_ticks_elapsed_seconds(
                    system_time_ticks,
                    state.high_voltage_ticks,
                    VescSeconds::from_seconds(0.5),
                ) || battery_voltage > high_voltage_threshold + Voltage::from_volts(1.0)
                    || bms_cell_over_voltage
                {
                    let angle = state.serialized_config.high_voltage_pushback_angle();
                    ride_state = ride_state.with_setpoint_adjustment(
                        FloatOutBoySetpointAdjustment::PushbackHighVoltage,
                    );
                    Some(if motor_erpm.is_positive() {
                        angle
                    } else {
                        -angle
                    })
                } else {
                    ride_state =
                        ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
                    None
                }
            } else if bms_connection_fault {
                beep_reason = FloatOutBoyBeepReason::BmsConnection;
                beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
                let angle = state.serialized_config.high_voltage_pushback_angle();
                ride_state = ride_state
                    .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackError);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if let Some((temperature_reason, tiltback)) = motor_temperature_warning {
                beep_reason = temperature_reason;
                beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
                if tiltback {
                    let angle = state.serialized_config.low_voltage_pushback_angle();
                    ride_state = ride_state.with_setpoint_adjustment(
                        FloatOutBoySetpointAdjustment::PushbackTemperature,
                    );
                    Some(if motor_erpm.is_positive() {
                        angle
                    } else {
                        -angle
                    })
                } else {
                    ride_state =
                        ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
                    None
                }
            } else if let Some(temperature_reason) = bms_temperature_reason {
                beep_reason = temperature_reason;
                beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
                let angle = state.serialized_config.low_voltage_pushback_angle();
                ride_state = ride_state
                    .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if voltage_pushback_duty
                && (bms_cell_under_voltage || battery_voltage < low_voltage_threshold)
            {
                beep_reason = if bms_cell_under_voltage {
                    FloatOutBoyBeepReason::CellLowVoltage
                } else {
                    FloatOutBoyBeepReason::LowVoltage
                };
                beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
                let voltage_delta = low_voltage_threshold - battery_voltage;
                let abs_motor_current = base.motor().directional_motor_current().current().abs();
                // C map: Float Out Boy tolerates pack sag only within 2 V, at 5 A
                // or more, and below 20 A per volt at
                // `third_party/float-out-boy/src/main.c:680-716`.
                let pushback = voltage_delta > Voltage::from_volts(2.0)
                    || abs_motor_current < Current::from_amps(5.0)
                    || voltage_delta.as_volts() * 20.0 / abs_motor_current.as_amps() > 1.0
                    || bms_cell_under_voltage;
                if pushback {
                    let angle = state.serialized_config.low_voltage_pushback_angle();
                    ride_state = ride_state.with_setpoint_adjustment(
                        FloatOutBoySetpointAdjustment::PushbackLowVoltage,
                    );
                    Some(if motor_erpm.is_positive() {
                        angle
                    } else {
                        -angle
                    })
                } else {
                    ride_state =
                        ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
                    Some(AngleDegrees::ZERO)
                }
            } else if speed_pushback_active {
                // C map: configured km/h speed pushback follows pack LV and
                // uses the duty angle/speed at
                // `third_party/float-out-boy/src/main.c:717-729`.
                let angle = state.runtime_duty_pushback_angle();
                let target = if speed.is_positive() { angle } else { -angle };
                beep_reason = FloatOutBoyBeepReason::Speed;
                ride_state = ride_state
                    .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackSpeed);
                Some(rate_limit_angle(
                    board_setpoint,
                    target,
                    state.runtime_duty_pushback_step(),
                ))
            } else if matches!(
                ride_state.setpoint_adjustment(),
                FloatOutBoySetpointAdjustment::PushbackDuty
                    | FloatOutBoySetpointAdjustment::PushbackHighVoltage
                    | FloatOutBoySetpointAdjustment::PushbackError
                    | FloatOutBoySetpointAdjustment::PushbackLowVoltage
                    | FloatOutBoySetpointAdjustment::PushbackSpeed
                    | FloatOutBoySetpointAdjustment::PushbackTemperature
            ) {
                ride_state =
                    ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
                Some(rate_limit_angle(
                    board_setpoint,
                    AngleDegrees::ZERO,
                    state.runtime_tiltback_return_step(),
                ))
            } else if !board_setpoint.is_zero() {
                Some(rate_limit_angle(
                    board_setpoint,
                    AngleDegrees::ZERO,
                    state.runtime_tiltback_return_step(),
                ))
            } else {
                None
            };
            if let Some(protective_setpoint) = protective_setpoint {
                // Float Out Boy selects duty pushback after reverse stop and
                // wheelslip at `third_party/float-out-boy/src/main.c:551-592`.
                board_setpoint = protective_setpoint;
            }
        }
        if matches!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::Detected)
            && above_wheelslip_duty_limit
        {
            // Upstream forces the target back to zero after every protective
            // selection while wheelslip remains above the motor duty limit at
            // `third_party/float-out-boy/src/main.c:719-721`.
            board_setpoint = AngleDegrees::ZERO;
        }
        state.runtime_board_setpoint = board_setpoint;
        let remote_setpoint = state.remote_control.update_input_tilt(
            state.serialized_config.input_tilt_angle_limit(),
            state.serialized_config.input_tilt_speed(),
            state.serialized_config.startup().sample_rate(),
            darkride_active,
        );
        // C map: `remote_update` runs after protective setpoint interpolation,
        // then the ride modifiers update and combine at
        // `third_party/float-out-boy/src/main.c:869-917`.
        setpoints = state.ride_modifiers.advance(
            &state.serialized_config,
            RideModifierInput {
                base_setpoint: board_setpoint,
                remote_setpoint,
                balance_pitch: balance_pitch_degrees,
                motor_erpm,
                filtered_current: base.motor().filtered_motor_current().current().current(),
                motor_current: base.motor().motor_current(),
                acceleration: motor_acceleration,
                darkride: darkride_active,
                wheelslip: ride_state.wheelslip(),
            },
        );
        if !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel) {
            let duty_warning = matches!(
                ride_state.setpoint_adjustment(),
                FloatOutBoySetpointAdjustment::PushbackDuty
            ) && (state.serialized_config.duty_beep_enabled()
                || state.serialized_config.duty_pushback_angle().is_zero());
            if duty_warning {
                state.force_beeper_on();
                state.duty_beeping = true;
                beep_reason = FloatOutBoyBeepReason::Duty;
            } else if state.duty_beeping {
                state.release_beeper();
            }
        }
        let gyro = imu.angular_rate();
        // Upstream RUNNING executes this exact balance-current pipeline at
        // `third_party/float-out-boy/src/main.c:918-956`; the helper keeps the
        // PID, booster, pitch-rate, soft-start, limit, darkride, and
        // traction branches unit-testable while this method preserves the
        // surrounding state-machine order.
        let mut loop_state = state.balance_loop;
        loop_state.balance_current = balance_current.current();
        loop_state.booster_current = booster_current.current();
        let balance_loop = loop_state.advance_balance_loop(
            state.runtime_balance_loop_config(),
            LoopInput {
                setpoint: setpoints.board(),
                brake_tilt_setpoint: setpoints.brake_tilt(),
                balance_pitch: balance_pitch.angle_degrees(),
                raw_pitch: pitch_degrees,
                roll: imu.roll(),
                gyro_pitch: gyro.pitch(),
                gyro_yaw: gyro.yaw(),
                motor_erpm: base.motor().electrical_speed(),
                motor_current: base.motor().motor_current(),
                motor_current_max: state.motor_current_max,
                motor_current_min: state.motor_current_min,
                mode: ride_state.mode(),
                darkride: ride_state.darkride(),
                traction_control: state.traction_control,
            },
        );
        state.balance_loop = balance_loop.state;
        booster_current =
            FloatOutBoyRealtimeBoosterCurrent::new(state.balance_loop.booster_current);
        balance_current =
            FloatOutBoyRealtimeBalanceCurrent::new(state.balance_loop.balance_current);
        state.request_motor_current(balance_loop.requested_current);
    } else if matches!(run_state, FloatOutBoyRunState::Ready)
        && !state_stop_fault
        && let Some(current) = state.remote_control.request_ready_current(
            motor_erpm,
            state.serialized_config.remote_throttle(),
            system_time_ticks,
            state.disengage_ticks,
        )
    {
        state.request_motor_current(current);
    }

    phase.ride_state = ride_state;
    phase.beep_reason = beep_reason;
    phase.beeper_alert = beeper_alert;
    (
        phase,
        RuntimeValues {
            balance_current,
            setpoints,
            booster_current,
        },
    )
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
) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let start = begin_refresh(state, base, imu.is_ready(), system_time_ticks);
    let phase = evaluate_transition_phase(state, imu, base, system_time_ticks, start);
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
        base.motor(),
    );
    state.all_data_payloads =
        FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}
