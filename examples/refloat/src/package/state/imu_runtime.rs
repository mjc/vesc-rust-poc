use super::limits::{
    DarkrideLimits, MovingFaultLimits, PushStartLimits, QuickStopLimits, RemoteSetpointFaultLimit,
    ReverseStopLimits, TractionLossLimits,
};
use super::*;
#[cfg(any(test, target_arch = "arm"))]
use crate::bms::RefloatBmsFault;
use crate::domain::RefloatBeepReason;
use vescpkg_rs::prelude::{AngleDegrees, VescSeconds, Voltage};

/// Refloat runtime refresh of IMU-derived state and control-loop faults.
///
/// C map: upstream `check_faults`, READY engage, startup reset, and traction
/// handling live in `third_party/refloat/src/main.c:263-509`,
/// `third_party/refloat/src/main.c:551-574`, `third_party/refloat/src/main.c:760-775`,
/// `third_party/refloat/src/main.c:833-838`, and `third_party/refloat/src/main.c:957-1067`.
pub(super) fn refresh(
    state: &mut RefloatPackageState,
    imu: &impl Imu,
    system_time_ticks: TimestampTicks,
) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let status = base.status();
    let mut beep_reason = status.beep_reason();
    let mut beeper_alert = None;
    let ride_state = status.ride_state();
    let resets_runtime_vars =
        matches!(ride_state.run_state(), RefloatRunState::Startup) && imu.is_ready();
    let run_state = match (ride_state.run_state(), imu.is_ready()) {
        (RefloatRunState::Startup, true) => RefloatRunState::Ready,
        (run_state, _) => run_state,
    };
    let flywheel_both_footpads_fault = matches!(
        (run_state, ride_state.mode(), base.footpad().state()),
        (
            RefloatRunState::Running,
            RefloatMode::Flywheel,
            RefloatFootpadState::Both
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
            RefloatFootpadState::None
        )
    );
    let reverse_stop_active = matches!(
        (run_state, ride_state.setpoint_adjustment()),
        (
            RefloatRunState::Running,
            RefloatSetpointAdjustment::ReverseStop
        )
    );
    let pitch = imu.pitch().angle();
    let pitch_degrees = AngleDegrees::from(pitch);
    let pitch_abs = pitch_degrees.abs();
    let roll = imu.roll().angle();
    let roll_degrees = AngleDegrees::from(roll);
    let roll_abs = roll_degrees.abs();
    let remote_setpoint_abs = base.setpoints().remote().angle().abs();
    let quickstop = QuickStopLimits::REFLOAT;
    let reverse_stop = ReverseStopLimits::REFLOAT;
    let remote_setpoint_fault = RemoteSetpointFaultLimit::REFLOAT.angle();
    let moving_fault = MovingFaultLimits::REFLOAT;
    let darkride = DarkrideLimits::REFLOAT;
    let push_start = PushStartLimits::REFLOAT;
    let traction_loss = TractionLossLimits::REFLOAT;
    // C map: `check_faults(d)` has a dedicated darkride branch at
    // `third_party/refloat/src/main.c:359-390`; normal switch/reverse/roll
    // faults only run in the `else` branch at `third_party/refloat/src/main.c:392-491`.
    let darkride_active = matches!(
        (run_state, ride_state.darkride()),
        (RefloatRunState::Running, RefloatDarkRideState::Active)
    );
    let reverse_stop_pitch_fault =
        !darkride_active && reverse_stop_active && pitch_abs > reverse_stop.pitch;
    let reverse_stop_timer_fault = !darkride_active
        && matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        )
        && {
            (pitch_abs > reverse_stop.timer_fast_pitch
                && refloat_ticks_elapsed(system_time_ticks, state.reverse_ticks, 1))
                || (pitch_abs > reverse_stop.timer_slow_pitch
                    && refloat_ticks_elapsed(system_time_ticks, state.reverse_ticks, 2))
        };
    let reverse_stop_total_erpm_fault = !darkride_active
        && matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        )
        && state.reverse_total_erpm.abs() > reverse_stop.total_erpm;
    let motor_erpm = base.motor().electrical_speed().rpm();
    // C updates `imu.balance_pitch` from the Refloat-owned balance filter
    // before control at `third_party/refloat/src/main.c:760-775`, `third_party/refloat/src/imu.c:35-41`, and
    // `third_party/refloat/src/balance_filter.c:145-154`; FLYWHEEL then overrides it with raw
    // pitch at `third_party/refloat/src/imu.c:56-58`.
    let balance_pitch = if matches!(ride_state.mode(), RefloatMode::Flywheel) {
        RefloatRealtimeBalancePitch::new(pitch)
    } else {
        state.balance_filter.balance_pitch()
    };
    let balance_pitch_degrees = balance_pitch.angle_degrees();
    let balance_pitch_abs = balance_pitch_degrees.abs();
    let faults = state.serialized_config.faults();
    let startup = state.serialized_config.startup();
    let quickstop_fault = matches!(
        (run_state, base.footpad().state(), ride_state.mode()),
        (
            RefloatRunState::Running,
            RefloatFootpadState::None,
            mode
        ) if !matches!(mode, RefloatMode::Flywheel)
    ) && faults.quickstop_enabled()
        && motor_erpm.abs() < quickstop.stopped_erpm
        && pitch_abs > quickstop.pitch
        && remote_setpoint_abs < remote_setpoint_fault
        && (pitch >= AngleRadians::ZERO) == (motor_erpm >= Rpm::ZERO);
    let single_footpad = matches!(
        base.footpad().state(),
        RefloatFootpadState::Left | RefloatFootpadState::Right
    );
    let dual_switch = faults.dual_switch();
    let simple_start = startup.simplestart_enabled()
        && (refloat_ticks_elapsed(system_time_ticks, state.disengage_ticks, 2)
            || !refloat_ticks_elapsed(system_time_ticks, state.engage_ticks, 1));
    let can_engage = matches!(ride_state.charging(), RefloatChargingState::NotCharging)
        && (matches!(base.footpad().state(), RefloatFootpadState::Both)
            || single_footpad && (dual_switch || simple_start)
            || matches!(ride_state.mode(), RefloatMode::Flywheel));
    let fault_adc_half_erpm = faults.adc_half_erpm().rpm();
    let fault_delay_switch_half = faults.switch_half_delay();
    let fault_delay_switch_full = faults.switch_full_delay();
    let switch_faults_disabled = faults.moving_faults_disabled()
        && motor_erpm > fault_adc_half_erpm * 2.0
        && roll_abs < moving_fault.roll;
    let full_switch_pending = !darkride_active
        && matches!(run_state, RefloatRunState::Running)
        && matches!(base.footpad().state(), RefloatFootpadState::None)
        && !matches!(ride_state.mode(), RefloatMode::Flywheel);
    let full_switch_fault = full_switch_pending
        && !switch_faults_disabled
        && (refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            fault_delay_switch_full,
        ) || motor_erpm.abs() < fault_adc_half_erpm * 6.0
            && refloat_ticks_elapsed_seconds(
                system_time_ticks,
                state.fault_switch_ticks,
                fault_delay_switch_half,
            ));
    let half_switch_pending = !darkride_active
        && matches!(run_state, RefloatRunState::Running)
        && !dual_switch
        && !can_engage
        && motor_erpm.abs() < fault_adc_half_erpm;
    let half_switch_fault = half_switch_pending
        && refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_half_ticks,
            fault_delay_switch_half,
        );
    let fault_roll = faults.roll_angle();
    let fault_delay_roll = faults.roll_delay();
    let roll_fault_pending =
        !darkride_active && matches!(run_state, RefloatRunState::Running) && roll_abs > fault_roll;
    let roll_fault = roll_fault_pending
        && refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            fault_delay_roll,
        );
    let fault_pitch = faults.pitch_angle();
    let fault_delay_pitch = faults.pitch_delay();
    let pitch_fault_pending = matches!(run_state, RefloatRunState::Running)
        && pitch_abs > fault_pitch
        && remote_setpoint_abs < remote_setpoint_fault;
    let pitch_fault = pitch_fault_pending
        && refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_pitch_ticks,
            fault_delay_pitch,
        );
    let ready_flywheel_stop = matches!(
        (run_state, ride_state.mode(), base.footpad().state()),
        (
            RefloatRunState::Ready,
            RefloatMode::Flywheel,
            RefloatFootpadState::Both
        )
    );
    let darkride_high_erpm_pending = darkride_active && motor_erpm > darkride.timed_high_erpm;
    let darkride_high_erpm_fault = darkride_high_erpm_pending
        && (refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_switch_ticks,
            darkride.timed_high_delay,
        ) || motor_erpm > darkride.high_erpm);
    let darkride_low_erpm_pending =
        darkride_active && motor_erpm <= darkride.timed_high_erpm && motor_erpm > darkride.low_erpm;
    let darkride_low_erpm_fault = darkride_low_erpm_pending
        && refloat_ticks_elapsed_seconds(
            system_time_ticks,
            state.fault_angle_roll_ticks,
            darkride.low_delay,
        );
    let darkride_can_engage_fault = darkride_active && can_engage;
    let darkride_roll_fault = !darkride_active
        && matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Upright)
        )
        && faults.darkride_enabled()
        && roll_abs > darkride.roll_lower
        && roll_abs < darkride.roll_upper;
    let startup_pitch_tolerance = startup.pitch_tolerance();
    let startup_roll_tolerance = startup.roll_tolerance();
    let ready_engage = !resets_runtime_vars
        && matches!(run_state, RefloatRunState::Ready)
        && !ready_flywheel_stop
        && can_engage
        && balance_pitch_abs < startup_pitch_tolerance
        && roll_abs < startup_roll_tolerance;
    let ready_darkride_engage = !resets_runtime_vars
        && matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Ready, RefloatDarkRideState::Active)
        )
        && balance_pitch_abs < startup_pitch_tolerance
        && {
            // Upstream READY darkride startup either ignores roll for the
            // first second unless the previous stop was reverse-stop, or
            // after that requires upside-down roll within startup tolerance
            // at `third_party/refloat/src/main.c:1038-1054`.
            let within_darkride_grace =
                !refloat_ticks_elapsed(system_time_ticks, state.disengage_ticks, 1)
                    && !matches!(
                        ride_state.stop_condition(),
                        RefloatStopCondition::ReverseStop
                    );
            let roll_near_upside_down =
                (roll_abs - AngleDegrees::from_degrees(180.0)).abs() < startup_roll_tolerance;

            within_darkride_grace || roll_near_upside_down
        };
    let ready_push_start = !resets_runtime_vars
        && matches!(run_state, RefloatRunState::Ready)
        && startup.pushstart_enabled()
        && motor_erpm.abs() > push_start.erpm_min
        && can_engage
        && balance_pitch_abs < push_start.angle
        && roll_abs < push_start.angle
        && !(faults.reversestop_enabled() && motor_erpm.is_negative());
    let state_engage = ready_engage || ready_darkride_engage || ready_push_start;
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
        // C map: darkride high-ERPM and low-ERPM branches both stop as
        // reverse-stop at `third_party/refloat/src/main.c:360-379`.
        (RefloatStopEvent::DarkrideHighErpm, darkride_high_erpm_fault),
        (RefloatStopEvent::DarkrideLowErpm, darkride_low_erpm_fault),
        (
            RefloatStopEvent::DarkrideCanEngage,
            darkride_can_engage_fault,
        ),
        (RefloatStopEvent::Roll, roll_fault),
        (RefloatStopEvent::Pitch, pitch_fault),
        (RefloatStopEvent::DarkrideRoll, darkride_roll_fault),
    ]);
    let reverse_stop_entry_pending = !matches!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering | RefloatSetpointAdjustment::ReverseStop
    ) && faults.reversestop_enabled()
        && motor_erpm < -reverse_stop.entry_erpm
        && !darkride_active;
    let motor_acceleration = state.motor_acceleration.average();
    let traction_loss_detected = stop_event.is_none()
        && !state_engage
        && !matches!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering | RefloatSetpointAdjustment::ReverseStop
        )
        && !reverse_stop_entry_pending
        && matches!(run_state, RefloatRunState::Running)
        && !matches!(ride_state.mode(), RefloatMode::Flywheel)
        && motor_acceleration.abs() > traction_loss.acceleration_detect
        && motor_acceleration.signum() == motor_erpm.signum()
        && base.motor().duty_cycle().ratio() > traction_loss.duty
        && motor_erpm.abs() > traction_loss.erpm;
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
        state.disengage_ticks = system_time_ticks;
    } else if state_transition.state_engaged {
        state.engage_ticks = system_time_ticks;
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
            RefloatRunState::Running,
            RefloatSetpointAdjustment::ReverseStop
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
        state.balance_loop.pid_integral_current = MotorCurrent::new(Current::ZERO);
        state.balance_loop.softstart_pid_limit = MotorCurrent::new(Current::ZERO);
        state.reverse_total_erpm = Rpm::ZERO;
        state.traction_control = false;
        state.remote_control.reset_runtime_vars();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(balance_pitch_degrees);
        let zero_setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::ZERO);
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
    if matches!(run_state, RefloatRunState::Running) && !state_engage && !state_stop_fault {
        let configured_high_voltage = state.serialized_config.high_voltage_threshold();
        let high_voltage_threshold = if configured_high_voltage.as_volts() < 10.0 {
            state
                .battery_cell_count
                .map_or(configured_high_voltage, |count| {
                    configured_high_voltage * count
                })
        } else {
            configured_high_voltage
        };
        let battery_voltage = base.motor().battery_voltage().voltage();
        #[cfg(any(test, target_arch = "arm"))]
        let bms_cell_over_voltage = state.bms_faults.contains(RefloatBmsFault::CellOverVoltage);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_cell_over_voltage = false;
        #[cfg(any(test, target_arch = "arm"))]
        let bms_connection_fault = state.bms_faults.contains(RefloatBmsFault::Connection);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_connection_fault = false;
        #[cfg(any(test, target_arch = "arm"))]
        let bms_temperature_reason = if state
            .bms_faults
            .contains(RefloatBmsFault::CellOverTemperature)
        {
            Some(RefloatBeepReason::CellOverTemperature)
        } else if state
            .bms_faults
            .contains(RefloatBmsFault::CellUnderTemperature)
        {
            Some(RefloatBeepReason::CellUnderTemperature)
        } else if state
            .bms_faults
            .contains(RefloatBmsFault::BmsOverTemperature)
        {
            Some(RefloatBeepReason::BmsOverTemperature)
        } else {
            None
        };
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_temperature_reason: Option<RefloatBeepReason> = None;
        #[cfg(any(test, target_arch = "arm"))]
        let bms_cell_under_voltage = state.bms_faults.contains(RefloatBmsFault::CellUnderVoltage);
        #[cfg(not(any(test, target_arch = "arm")))]
        let bms_cell_under_voltage = false;
        // Refloat refreshes this before every setpoint-adjustment branch at
        // `third_party/refloat/src/main.c:512-518`, except while a cell
        // over-voltage fault is active.
        if battery_voltage < high_voltage_threshold && !bms_cell_over_voltage {
            state.high_voltage_ticks = system_time_ticks;
        }
        let motor_duty = base.motor().duty_cycle().magnitude();
        let above_wheelslip_duty_limit = motor_duty > state.duty_max_with_margin.ratio();
        let entered_reverse_stop = reverse_stop_entry_pending;
        if entered_reverse_stop {
            // Refloat carries an existing HV/LV/temperature target into
            // reverse-stop at `third_party/refloat/src/main.c:538-550`.
            state.reverse_total_erpm = if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::PushbackHighVoltage
                    | RefloatSetpointAdjustment::PushbackLowVoltage
                    | RefloatSetpointAdjustment::PushbackTemperature
            ) {
                reverse_stop.carryover_total_erpm(setpoints.board().angle())
            } else {
                Rpm::ZERO
            };
            state.reverse_ticks = system_time_ticks;
            ride_state =
                ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::ReverseStop);
        }
        // C map: these are the detection and active-wheelslip branches in
        // `calculate_setpoint_target` at `third_party/refloat/src/main.c:551-575`.
        let wheelslip_branch_active = if traction_loss_detected {
            state.wheelslip_ticks = system_time_ticks;
            if darkride_active {
                state.traction_control = true;
            }
            true
        } else if matches!(ride_state.wheelslip(), RefloatWheelSlipState::Detected)
            && !matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::Centering | RefloatSetpointAdjustment::ReverseStop
            )
        {
            if motor_acceleration.abs() < traction_loss.acceleration_clear {
                state.traction_control = false;
            }
            if above_wheelslip_duty_limit {
                state.wheelslip_ticks = system_time_ticks;
            } else if refloat_ticks_elapsed_seconds(
                system_time_ticks,
                state.wheelslip_ticks,
                traction_loss.clear_delay,
            ) && state.motor_duty_raw < traction_loss.raw_duty_clear
            {
                state.traction_control = false;
                ride_state = ride_state.with_wheelslip(RefloatWheelSlipState::None);
            }
            true
        } else {
            false
        };
        if matches!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        ) {
            let board_setpoint = setpoints.board().angle();
            if board_setpoint.is_zero() {
                // Upstream `calculate_setpoint_target(d)` exits
                // `SAT_CENTERING` when `setpoint_target_interpolated`
                // already equals target zero at
                // `third_party/refloat/src/main.c:517-520`.
                ride_state = ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::None);
            } else {
                let startup_step = startup.centering_step();
                let centered_board = if board_setpoint.abs() < startup_step {
                    AngleDegrees::ZERO
                } else {
                    board_setpoint - startup_step * board_setpoint.signum()
                };
                // Upstream stores `startup_speed / hertz` at
                // `third_party/refloat/src/main.c:172`, selects it for
                // `SAT_CENTERING` at `third_party/refloat/src/main.c:304-310`,
                // applies `rate_limitf` at
                // `third_party/refloat/src/utils.c:25-33`, and assigns the
                // centered setpoint before PID at
                // `third_party/refloat/src/main.c:869-875`.
                let centered_board = RefloatRealtimeRuntimeSetpoint::new(centered_board);
                setpoints = setpoints.with_board(centered_board);
            }
        }
        if matches!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::ReverseStop
        ) && !entered_reverse_stop
        {
            // Upstream `calculate_setpoint_target(d)` accumulates ERPM, grows
            // the nose-down target past tolerance, and exits below half
            // tolerance while moving forward at
            // `third_party/refloat/src/main.c:522-536`.
            state.reverse_total_erpm = state.reverse_total_erpm + motor_erpm;
            let reverse_total_erpm = state.reverse_total_erpm.abs();
            let board_setpoint = if reverse_total_erpm > reverse_stop.tolerance_erpm {
                Some(reverse_stop.target_angle(state.reverse_total_erpm))
            } else if reverse_total_erpm <= reverse_stop.tolerance_erpm * 0.5
                && !motor_erpm.is_negative()
            {
                state.reverse_total_erpm = Rpm::ZERO;
                ride_state = ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::None);
                Some(AngleDegrees::ZERO)
            } else {
                None
            };
            if let Some(board_setpoint) = board_setpoint {
                setpoints =
                    setpoints.with_board(RefloatRealtimeRuntimeSetpoint::new(board_setpoint));
            }
        }
        if !matches!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering | RefloatSetpointAdjustment::ReverseStop
        ) && !wheelslip_branch_active
            && !matches!(ride_state.wheelslip(), RefloatWheelSlipState::Detected)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel)
        {
            let duty_pushback_active = base.motor().duty_cycle().ratio().as_ratio()
                > state.serialized_config.duty_pushback_threshold().as_ratio();
            let board_setpoint = if duty_pushback_active {
                let angle = state.serialized_config.duty_pushback_angle();
                ride_state =
                    ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::PushbackDuty);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if base.motor().duty_cycle().ratio().as_ratio() > 0.05
                && (battery_voltage > high_voltage_threshold || bms_cell_over_voltage)
            {
                beep_reason = if bms_cell_over_voltage {
                    RefloatBeepReason::CellHighVoltage
                } else {
                    RefloatBeepReason::HighVoltage
                };
                beeper_alert = Some(RefloatBeeperAlert::ThreeShort);
                if refloat_ticks_elapsed_seconds(
                    system_time_ticks,
                    state.high_voltage_ticks,
                    VescSeconds::from_seconds(0.5),
                ) || battery_voltage > high_voltage_threshold + Voltage::from_volts(1.0)
                    || bms_cell_over_voltage
                {
                    let angle = state.serialized_config.high_voltage_pushback_angle();
                    ride_state = ride_state
                        .with_setpoint_adjustment(RefloatSetpointAdjustment::PushbackHighVoltage);
                    Some(if motor_erpm.is_positive() {
                        angle
                    } else {
                        -angle
                    })
                } else {
                    ride_state =
                        ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::None);
                    None
                }
            } else if bms_connection_fault {
                beep_reason = RefloatBeepReason::BmsConnection;
                beeper_alert = Some(RefloatBeeperAlert::Long(RefloatBeeperCount::THREE));
                let angle = state.serialized_config.high_voltage_pushback_angle();
                ride_state =
                    ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::PushbackError);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if let Some(temperature_reason) = bms_temperature_reason {
                beep_reason = temperature_reason;
                beeper_alert = Some(RefloatBeeperAlert::Long(RefloatBeeperCount::THREE));
                let angle = state.serialized_config.low_voltage_pushback_angle();
                ride_state = ride_state
                    .with_setpoint_adjustment(RefloatSetpointAdjustment::PushbackTemperature);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if base.motor().duty_cycle().ratio().as_ratio() > 0.05 && bms_cell_under_voltage
            {
                beep_reason = RefloatBeepReason::CellLowVoltage;
                beeper_alert = Some(RefloatBeeperAlert::ThreeShort);
                let angle = state.serialized_config.low_voltage_pushback_angle();
                ride_state = ride_state
                    .with_setpoint_adjustment(RefloatSetpointAdjustment::PushbackLowVoltage);
                Some(if motor_erpm.is_positive() {
                    angle
                } else {
                    -angle
                })
            } else if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::PushbackDuty
                    | RefloatSetpointAdjustment::PushbackHighVoltage
                    | RefloatSetpointAdjustment::PushbackError
                    | RefloatSetpointAdjustment::PushbackLowVoltage
                    | RefloatSetpointAdjustment::PushbackTemperature
            ) {
                ride_state = ride_state.with_setpoint_adjustment(RefloatSetpointAdjustment::None);
                Some(AngleDegrees::ZERO)
            } else {
                None
            };
            if let Some(board_setpoint) = board_setpoint {
                // Refloat selects duty pushback after reverse stop and
                // wheelslip at `third_party/refloat/src/main.c:551-592`.
                setpoints =
                    setpoints.with_board(RefloatRealtimeRuntimeSetpoint::new(board_setpoint));
            }
        }
        if matches!(ride_state.wheelslip(), RefloatWheelSlipState::Detected)
            && above_wheelslip_duty_limit
        {
            // Upstream forces the target back to zero after every protective
            // selection while wheelslip remains above the motor duty limit at
            // `third_party/refloat/src/main.c:719-721`.
            setpoints =
                setpoints.with_board(RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::ZERO));
        }
        let gyro = imu.angular_rate();
        // Upstream RUNNING executes this exact balance-current pipeline at
        // `third_party/refloat/src/main.c:918-956`; the helper keeps the
        // PID, booster, pitch-rate, soft-start, limit, darkride, and
        // traction branches unit-testable while this method preserves the
        // surrounding state-machine order.
        let mut loop_state = state.balance_loop;
        loop_state.balance_current = balance_current.current();
        loop_state.booster_current = booster_current.current();
        let balance_loop = loop_state.advance_balance_loop(
            state.serialized_config.balance_loop_config(),
            LoopInput {
                setpoint: setpoints.board(),
                brake_tilt_setpoint: setpoints.brake_tilt(),
                balance_pitch: balance_pitch.angle_degrees(),
                raw_pitch: AngleDegrees::from(imu.pitch().angle()),
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
        booster_current = RefloatRealtimeBoosterCurrent::new(state.balance_loop.booster_current);
        balance_current = RefloatRealtimeBalanceCurrent::new(state.balance_loop.balance_current);
        state.request_motor_current(balance_loop.requested_current);
    } else if matches!(run_state, RefloatRunState::Ready)
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
    #[cfg(any(test, target_arch = "arm"))]
    if matches!(run_state, RefloatRunState::Ready) && !ready_flywheel_stop {
        let connection_fault = state.bms_faults.contains(RefloatBmsFault::Connection);
        let balance_fault = state.bms_faults.contains(RefloatBmsFault::CellBalance)
            && refloat_ticks_elapsed(system_time_ticks, state.disengage_ticks, 5);
        if (connection_fault || balance_fault)
            && refloat_ticks_elapsed(system_time_ticks, state.bms_alert_ticks, 15)
        {
            state.bms_alert_ticks = system_time_ticks;
            beep_reason = if connection_fault {
                RefloatBeepReason::BmsConnection
            } else {
                RefloatBeepReason::CellBalance
            };
            beeper_alert = Some(RefloatBeeperAlert::FourShort);
        }
    }
    if let Some(alert) = beeper_alert {
        state.alert_beeper(alert);
    }
    // C publishes the just-refreshed `imu.balance_pitch` through app-data;
    // normal mode comes from the balance filter at `third_party/refloat/src/imu.c:35-41`, while
    // FLYWHEEL mirrors raw pitch at `third_party/refloat/src/imu.c:56-58`.
    let base = RefloatAllDataBasePayload::new(
        balance_current,
        RefloatAllDataAttitude::new(balance_pitch, imu.roll(), imu.pitch()),
        RefloatAllDataStatus::new(ride_state, beep_reason),
        base.footpad(),
        setpoints,
        booster_current,
        base.motor(),
    );
    state.all_data_payloads =
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}
