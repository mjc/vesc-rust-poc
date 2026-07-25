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
        && matches!(base.footpad().state(), FloatOutBoyFootpadState::None)
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
    balance_pitch_abs: AngleDegrees,
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
        && (state.ride_flags.flywheel_abort
            || matches!(base.footpad().state(), FloatOutBoyFootpadState::Both));
    let run_state = if ready_stop {
        state.restore_flywheel_config();
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
        balance_pitch_abs: balance_pitch.angle_degrees().abs(),
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
    darkride_active: bool,
    startup_became_ready: bool,
    ready_flywheel_stop: bool,
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
    flywheel_both_footpads: bool,
}

struct SwitchAngleFaultEvaluation {
    conditions: [bool; 5],
    switches: SwitchFaultActivity,
    angles: AngleFaultActivity,
}

fn evaluate_switch_angle_faults(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    input: &FaultInputs,
    can_engage: bool,
) -> SwitchAngleFaultEvaluation {
    let faults = state.serialized_config.faults();
    let footpad = base.footpad().state();
    let running = matches!(input.run_state, FloatOutBoyRunState::Running);
    let flywheel = matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel);
    let half_erpm = faults.adc_half_erpm().rpm();
    let full_pending = !input.darkride_active
        && running
        && matches!(footpad, FloatOutBoyFootpadState::None)
        && !flywheel;
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
        && matches!(footpad, FloatOutBoyFootpadState::None)
        && !flywheel
        && faults.quickstop_enabled()
        && input.motor_erpm.abs() < quickstop.stopped_erpm
        && input.pitch_abs > quickstop.pitch
        && input.remote_setpoint_abs < RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle()
        && (input.pitch >= AngleRadians::ZERO) == (input.motor_erpm >= Rpm::ZERO);
    SwitchAngleFaultEvaluation {
        conditions: [
            full_fault,
            quickstop_fault,
            half_fault,
            roll_fault,
            pitch_fault,
        ],
        switches: SwitchFaultActivity {
            full: full_pending,
            half: half_pending,
        },
        angles: AngleFaultActivity {
            roll: roll_pending,
            pitch: pitch_pending,
        },
    }
}

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
    let flywheel_both = running && flywheel && matches!(footpad, FloatOutBoyFootpadState::Both);
    let reverse_no_footpads = reverse_active && matches!(footpad, FloatOutBoyFootpadState::None);
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
    let switch_angle =
        evaluate_switch_angle_faults(state, base, system_time_ticks, input, can_engage);
    let [
        full_fault,
        quickstop_fault,
        half_fault,
        roll_fault,
        pitch_fault,
    ] = switch_angle.conditions;
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
            flywheel_both,
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
        switches: switch_angle.switches,
        angles: switch_angle.angles,
        can_engage,
        flywheel_both_footpads: flywheel_both,
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

struct EngagementEvaluation {
    engage: bool,
    centering_step: AngleDegrees,
}

fn evaluate_engagement(
    state: &FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    input: &FaultInputs,
    can_engage: bool,
) -> EngagementEvaluation {
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let push_start = PushStartLimits::FLOAT_OUT_BOY;
    let pitch_tolerance = startup.pitch_tolerance();
    let roll_tolerance = startup.roll_tolerance();
    let ready_engage = !input.startup_became_ready
        && matches!(input.run_state, FloatOutBoyRunState::Ready)
        && !input.ready_flywheel_stop
        && can_engage
        && input.balance_pitch_abs < pitch_tolerance
        && input.roll_abs < roll_tolerance;
    let ready_darkride = !input.startup_became_ready
        && matches!(
            (input.run_state, input.ride_state.darkride()),
            (FloatOutBoyRunState::Ready, FloatOutBoyDarkRideState::Active)
        )
        && input.balance_pitch_abs < pitch_tolerance
        && {
            // READY darkride either ignores roll during its initial grace or
            // requires upside-down roll within startup tolerance.
            let within_grace =
                !float_out_boy_ticks_elapsed(system_time_ticks, state.disengage_ticks, 1)
                    && !matches!(
                        input.ride_state.stop_condition(),
                        FloatOutBoyStopCondition::ReverseStop
                    );
            let upside_down =
                (input.roll_abs - AngleDegrees::from_degrees(180.0)).abs() < roll_tolerance;
            within_grace || upside_down
        };
    let ready_push_start = !input.startup_became_ready
        && matches!(input.run_state, FloatOutBoyRunState::Ready)
        && startup.pushstart_enabled()
        && input.motor_erpm.abs() > push_start.erpm_min
        && can_engage
        && input.balance_pitch_abs < push_start.angle
        && input.roll_abs < push_start.angle
        && !(faults.reversestop_enabled() && input.motor_erpm.is_negative());
    EngagementEvaluation {
        engage: ready_engage || ready_darkride || ready_push_start,
        centering_step: startup.centering_step(),
    }
}

fn first_transition_stop(
    normal: &NormalFaultEvaluation,
    darkride: &DarkrideFaultEvaluation,
) -> Option<FloatOutBoyStopEvent> {
    let [
        flywheel_both,
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
    float_out_boy_first_stop_event(&[
        (FloatOutBoyStopEvent::FlywheelBothFootpads, flywheel_both),
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
    ])
}

fn transition_control_conditions(
    state: &FloatOutBoyPackageState,
    base: &FloatOutBoyAllDataBasePayload,
    input: &FaultInputs,
    state_engage: bool,
    stop_event: Option<FloatOutBoyStopEvent>,
) -> (ControlConditions, Rpm) {
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
    let reverse_stop_entry_pending = !matches!(
        input.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
    ) && state.serialized_config.faults().reversestop_enabled()
        && input.motor_erpm < -reverse_stop.entry_erpm
        && !input.darkride_active;
    let motor_acceleration = state.motor_acceleration.average();
    let traction_loss_detected = stop_event.is_none()
        && !state_engage
        && !matches!(
            input.ride_state.setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::Centering | FloatOutBoySetpointAdjustment::ReverseStop
        )
        && !reverse_stop_entry_pending
        && matches!(input.run_state, FloatOutBoyRunState::Running)
        && !matches!(input.ride_state.mode(), FloatOutBoyMode::Flywheel)
        && motor_acceleration.abs() > traction_loss.acceleration_detect
        && motor_acceleration.is_negative() == input.motor_erpm.is_negative()
        && base.motor().duty_cycle().ratio() > traction_loss.duty
        && input.motor_erpm.abs() > traction_loss.erpm;
    (
        ControlConditions {
            reverse_stop_entry_pending,
            traction_loss_detected,
            darkride_active: input.darkride_active,
        },
        motor_acceleration,
    )
}

struct TransitionOutcome {
    ride_state: FloatOutBoyRideState,
    stopped: bool,
}

struct TransitionActivity<'a> {
    input: &'a FaultInputs,
    normal: &'a NormalFaultEvaluation,
    darkride: &'a DarkrideFaultEvaluation,
    control: &'a ControlConditions,
    state_engage: bool,
    stop_event: Option<FloatOutBoyStopEvent>,
}

fn apply_transition_activity(
    state: &mut FloatOutBoyPackageState,
    system_time_ticks: TimestampTicks,
    activity: &TransitionActivity<'_>,
) -> TransitionOutcome {
    let transition = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        previous: activity.input.ride_state,
        run_state: activity.input.run_state,
        ready_flywheel_stop: activity.input.ready_flywheel_stop,
        state_engage: activity.state_engage,
        traction_loss_detected: activity.control.traction_loss_detected,
        stop_event: activity.stop_event,
    });
    if transition.state_stopped {
        state.disengage_ticks = system_time_ticks;
        state.ride_flags.flywheel_abort |= activity.normal.flywheel_both_footpads;
    } else if transition.state_engaged {
        state.engage_ticks = system_time_ticks;
    }
    if matches!(activity.input.run_state, FloatOutBoyRunState::Running) && !transition.state_stopped
    {
        state.upside_down_flags.enabled = true;
        if activity.input.darkride_active && !state.upside_down_flags.started {
            state.upside_down_flags.started = true;
            state.upside_down_fault_ticks = system_time_ticks;
        }
    }
    if !activity.darkride.high_erpm_pending && !activity.normal.switches.full {
        state.fault_switch_ticks = system_time_ticks;
    }
    if !activity.normal.switches.half {
        state.fault_switch_half_ticks = system_time_ticks;
    }
    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    if !matches!(
        (
            activity.input.run_state,
            activity.input.ride_state.setpoint_adjustment()
        ),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoySetpointAdjustment::ReverseStop
        )
    ) || activity.input.pitch_abs < reverse_stop.timer_slow_pitch
    {
        state.reverse_ticks = system_time_ticks;
    }
    if !activity.darkride.low_erpm_pending && !activity.normal.angles.roll {
        state.fault_angle_roll_ticks = system_time_ticks;
    }
    if !activity.normal.angles.pitch {
        state.fault_angle_pitch_ticks = system_time_ticks;
    }
    TransitionOutcome {
        ride_state: transition.ride_state,
        stopped: transition.state_stopped,
    }
}

fn evaluate_transition_phase(
    state: &mut FloatOutBoyPackageState,
    imu: &impl Imu,
    base: &FloatOutBoyAllDataBasePayload,
    system_time_ticks: TimestampTicks,
    start: &RefreshStart,
) -> TransitionPhase {
    let mut beep_reason = start.beep_reason;
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
    beep_reason = refresh_footpad_warning(
        state,
        base,
        ride_state,
        start.run_state,
        motor_erpm,
        beep_reason,
    );
    let readiness =
        refresh_flywheel_readiness(state, base, ride_state, start.run_state, attitude.pitch);
    let darkride_active = matches!(
        (readiness.run_state, ride_state.darkride()),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyDarkRideState::Active
        )
    );
    let fault_inputs = FaultInputs {
        ride_state,
        run_state: readiness.run_state,
        pitch: attitude.pitch,
        pitch_abs: attitude.pitch_abs,
        roll_abs: attitude.roll_abs,
        balance_pitch_abs: readiness.balance_pitch_abs,
        remote_setpoint_abs: base.setpoints().remote().angle().abs(),
        motor_erpm,
        darkride_active,
        startup_became_ready: start.startup_became_ready,
        ready_flywheel_stop: readiness.ready_stop,
    };
    let normal = evaluate_normal_faults(state, base, system_time_ticks, &fault_inputs);
    let darkride =
        evaluate_darkride_faults(state, system_time_ticks, &fault_inputs, normal.can_engage);
    let engagement =
        evaluate_engagement(state, system_time_ticks, &fault_inputs, normal.can_engage);
    let stop_event = first_transition_stop(&normal, &darkride);
    let (control, motor_acceleration) =
        transition_control_conditions(state, base, &fault_inputs, engagement.engage, stop_event);
    let outcome = apply_transition_activity(
        state,
        system_time_ticks,
        &TransitionActivity {
            input: &fault_inputs,
            normal: &normal,
            darkride: &darkride,
            control: &control,
            state_engage: engagement.engage,
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
            state_engage: engagement.engage,
            state_stop_fault: outcome.stopped,
        },
        #[cfg(any(test, target_arch = "arm"))]
        ready_flywheel_stop: readiness.ready_stop,
        balance_pitch: readiness.balance_pitch,
        pitch_degrees: attitude.pitch_degrees,
        imu_pitch: attitude.imu_pitch,
        imu_roll: attitude.imu_roll,
        motor_erpm,
        control,
        motor_acceleration,
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
    let bms_temperature_reason = None;
    #[cfg(any(test, target_arch = "arm"))]
    let bms_cell_under_voltage = state
        .bms_faults
        .contains(FloatOutBoyBmsFault::CellUnderVoltage);
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
    if phase.control.traction_loss_detected {
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
    if phase.control.reverse_stop_entry_pending
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

fn directional_angle(angle: AngleDegrees, motor_erpm: Rpm) -> AngleDegrees {
    if motor_erpm.is_positive() {
        angle
    } else {
        -angle
    }
}

fn apply_duty_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    if context.base.motor().duty_cycle().ratio().as_ratio()
        <= state.runtime_duty_pushback_threshold().as_ratio()
    {
        return false;
    }
    let angle = state.runtime_duty_pushback_angle();
    if !matches!(control.ride_state.mode(), FloatOutBoyMode::Flywheel) {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);
    }
    control.board_setpoint = rate_limit_angle(
        control.board_setpoint,
        directional_angle(angle, context.phase.motor_erpm),
        state.runtime_duty_pushback_step(),
    );
    true
}

fn apply_high_voltage_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    let signals = context.signals;
    if context.base.motor().duty_cycle().ratio().as_ratio() <= 0.05
        || !(signals.battery_voltage > signals.high_voltage_threshold
            || signals.bms_cell_over_voltage)
    {
        return false;
    }
    control.beep_reason = if signals.bms_cell_over_voltage {
        FloatOutBoyBeepReason::CellHighVoltage
    } else {
        FloatOutBoyBeepReason::HighVoltage
    };
    control.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
    let tiltback = float_out_boy_ticks_elapsed_seconds(
        context.system_time_ticks,
        state.high_voltage_ticks,
        VescSeconds::from_seconds(0.5),
    ) || signals.battery_voltage
        > signals.high_voltage_threshold + Voltage::from_volts(1.0)
        || signals.bms_cell_over_voltage;
    if tiltback {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackHighVoltage);
        control.board_setpoint = directional_angle(
            state.serialized_config.high_voltage_pushback_angle(),
            context.phase.motor_erpm,
        );
    } else {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
    }
    true
}

fn apply_bms_connection_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    if !context.signals.bms_connection_fault {
        return false;
    }
    control.beep_reason = FloatOutBoyBeepReason::BmsConnection;
    control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
    control.ride_state = control
        .ride_state
        .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackError);
    control.board_setpoint = directional_angle(
        state.serialized_config.high_voltage_pushback_angle(),
        context.phase.motor_erpm,
    );
    true
}

fn apply_motor_temperature_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    let Some((reason, tiltback)) = context.signals.motor_temperature_warning else {
        return false;
    };
    control.beep_reason = reason;
    control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
    if tiltback {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
        control.board_setpoint = directional_angle(
            state.serialized_config.low_voltage_pushback_angle(),
            context.phase.motor_erpm,
        );
    } else {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
    }
    true
}

fn apply_bms_temperature_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    let Some(reason) = context.signals.bms_temperature_reason else {
        return false;
    };
    control.beep_reason = reason;
    control.beeper_alert = Some(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));
    control.ride_state = control
        .ride_state
        .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackTemperature);
    control.board_setpoint = directional_angle(
        state.serialized_config.low_voltage_pushback_angle(),
        context.phase.motor_erpm,
    );
    true
}

fn apply_low_voltage_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    let signals = context.signals;
    if context.base.motor().duty_cycle().ratio().as_ratio() <= 0.05
        || !(signals.bms_cell_under_voltage
            || signals.battery_voltage < signals.low_voltage_threshold)
    {
        return false;
    }
    control.beep_reason = if signals.bms_cell_under_voltage {
        FloatOutBoyBeepReason::CellLowVoltage
    } else {
        FloatOutBoyBeepReason::LowVoltage
    };
    control.beeper_alert = Some(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
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
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackLowVoltage);
        control.board_setpoint = directional_angle(
            state.serialized_config.low_voltage_pushback_angle(),
            context.phase.motor_erpm,
        );
    } else {
        control.ride_state = control
            .ride_state
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        control.board_setpoint = AngleDegrees::ZERO;
    }
    true
}

fn apply_speed_pushback(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) -> bool {
    let speed = context.base.motor().vehicle_speed().speed();
    let threshold = state.serialized_config.speed_pushback_threshold();
    if !threshold.is_positive() || speed.abs() <= threshold {
        return false;
    }
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
    true
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

fn apply_protective_setpoint(
    state: &FloatOutBoyPackageState,
    context: &ProtectionContext<'_>,
    control: &mut RunningControl,
) {
    if apply_duty_pushback(state, context, control)
        || apply_high_voltage_pushback(state, context, control)
        || apply_bms_connection_pushback(state, context, control)
        || apply_motor_temperature_pushback(state, context, control)
        || apply_bms_temperature_pushback(state, context, control)
        || apply_low_voltage_pushback(state, context, control)
        || apply_speed_pushback(state, context, control)
    {
        return;
    }
    return_protective_setpoint(state, control);
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
        phase.control.reverse_stop_entry_pending,
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
    let reset_runtime = phase.events.startup_became_ready || phase.events.state_engage;
    let mut runtime = runtime_values(
        state,
        base,
        phase.balance_pitch.angle_degrees(),
        reset_runtime,
    );

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
) {
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
        base.motor(),
    );
    state.all_data_payloads =
        FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}
