use super::*;
use crate::domain::{FloatOutBoyChargingState, FloatOutBoyDarkRideState};

fn running_normal() -> FloatOutBoyRideState {
    FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    )
}

fn transition_input(previous: FloatOutBoyRideState) -> FloatOutBoyStateTransitionInput {
    FloatOutBoyStateTransitionInput {
        previous,
        run_state: previous.run_state(),
        ready_flywheel_stop: false,
        state_engage: false,
        traction_loss_detected: false,
        stop_event: None,
    }
}

#[test]
fn state_transition_selects_first_stop_event_like_float_out_boy_check_faults() {
    let event = float_out_boy_first_stop_event(&[
        (FloatOutBoyStopEvent::QuickStop, false),
        (FloatOutBoyStopEvent::HalfSwitch, true),
        (FloatOutBoyStopEvent::Pitch, true),
    ]);

    // Upstream returns immediately after the first active `state_stop` in
    // `third_party/float-out-boy/src/main.c:357-509`.
    assert_eq!(event, Some(FloatOutBoyStopEvent::HalfSwitch));
}

#[test]
fn darkride_erpm_stop_events_map_to_float_out_boy_reverse_stop() {
    // Float Out Boy darkride high-ERPM and low-ERPM branches both call
    // `state_stop(..., STOP_REVERSE_STOP)` at
    // `third_party/float-out-boy/src/main.c:360-379`.
    assert_eq!(
        FloatOutBoyStopEvent::DarkrideHighErpm.stop_condition(),
        FloatOutBoyStopCondition::ReverseStop
    );
    assert_eq!(
        FloatOutBoyStopEvent::DarkrideLowErpm.stop_condition(),
        FloatOutBoyStopCondition::ReverseStop
    );
}

#[test]
fn state_transition_stop_wins_over_engage_like_float_out_boy_state_stop() {
    let previous = running_normal().with_wheelslip(FloatOutBoyWheelSlipState::Detected);
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        state_engage: true,
        stop_event: Some(FloatOutBoyStopEvent::QuickStop),
        ..transition_input(previous)
    });

    // Upstream `state_stop` writes READY and clears wheelslip at
    // `third_party/float-out-boy/src/state.c:29-33`; this takes precedence over
    // engage in the caller's ordered loop.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::QuickStop
    );
    assert_eq!(
        output.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::None
    );
    assert_eq!(
        output.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::None
    );
    assert!(output.state_stopped);
    assert!(!output.state_engaged);
}

#[test]
fn state_transition_engage_sets_running_centering_and_clears_stop_like_float_out_boy() {
    let previous = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::Pitch,
    )
    .with_charging(FloatOutBoyChargingState::NotCharging);
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        state_engage: true,
        ..transition_input(previous)
    });

    // Upstream `state_engage` writes RUNNING, SAT_CENTERING, and STOP_NONE
    // at `third_party/float-out-boy/src/state.c:36-39`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        output.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::None
    );
    assert!(!output.state_stopped);
    assert!(output.state_engaged);
}

#[test]
fn state_transition_ready_flywheel_stop_returns_to_normal_like_float_out_boy_ready_loop() {
    let previous = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Flywheel,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    );
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        ready_flywheel_stop: true,
        ..transition_input(previous)
    });

    // Upstream READY stops FLYWHEEL before startup checks at
    // `third_party/float-out-boy/src/main.c:957-963`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(output.ride_state.mode(), FloatOutBoyMode::Normal);
}

#[test]
fn state_transition_marks_wheelslip_without_stopping_like_float_out_boy_traction_flag() {
    let previous = running_normal()
        .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty)
        .with_darkride(FloatOutBoyDarkRideState::Active);
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        traction_loss_detected: true,
        ..transition_input(previous)
    });

    // Upstream detects traction loss in `third_party/float-out-boy/src/main.c:551-562`;
    // freewheel happens later in `third_party/float-out-boy/src/main.c:949-954`.
    assert_eq!(
        output.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::Detected
    );
    assert_eq!(
        output.ride_state.darkride(),
        FloatOutBoyDarkRideState::Active
    );
    assert_eq!(
        output.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::None,
    );
    assert!(!output.state_stopped);
}

#[test]
fn state_transition_preserves_pushback_without_traction_loss() {
    let previous =
        running_normal().with_setpoint_adjustment(FloatOutBoySetpointAdjustment::PushbackDuty);

    let output = float_out_boy_state_transition(transition_input(previous));

    assert_eq!(
        output.ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::PushbackDuty,
    );
    assert_eq!(
        output.ride_state.wheelslip(),
        FloatOutBoyWheelSlipState::None
    );
}

#[test]
fn state_transition_stops_on_full_switch_like_float_out_boy_fault_check() {
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        stop_event: Some(FloatOutBoyStopEvent::FullSwitch),
        ..transition_input(running_normal())
    });

    // Upstream `check_faults(d)` stops a fully open switch after the delay at
    // `third_party/float-out-boy/src/main.c:397-404`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchFull
    );
}

#[test]
fn state_transition_stops_on_half_switch_like_float_out_boy_fault_check() {
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        stop_event: Some(FloatOutBoyStopEvent::HalfSwitch),
        ..transition_input(running_normal())
    });

    // Upstream `check_faults(d)` stops a partially open switch after the delay at
    // `third_party/float-out-boy/src/main.c:459-467`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchHalf
    );
}

#[test]
fn state_transition_stops_on_quickstop_like_float_out_boy_fault_check() {
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        stop_event: Some(FloatOutBoyStopEvent::QuickStop),
        ..transition_input(running_normal())
    });

    // Upstream `check_faults(d)` quick-stops the runaway case at
    // `third_party/float-out-boy/src/main.c:419-423`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::QuickStop
    );
}

#[test]
fn state_transition_stops_on_pitch_like_float_out_boy_fault_check() {
    let output = float_out_boy_state_transition(FloatOutBoyStateTransitionInput {
        stop_event: Some(FloatOutBoyStopEvent::Pitch),
        ..transition_input(
            FloatOutBoyRideState::new(
                FloatOutBoyRunState::Running,
                FloatOutBoyMode::Normal,
                FloatOutBoySetpointAdjustment::ReverseStop,
                FloatOutBoyStopCondition::None,
            )
            .with_darkride(FloatOutBoyDarkRideState::Active),
        )
    });

    // Upstream reverse-stop pitch faults stop at
    // `third_party/float-out-boy/src/main.c:423-426` and `440-443`.
    assert_eq!(output.ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        output.ride_state.stop_condition(),
        FloatOutBoyStopCondition::Pitch
    );
}
