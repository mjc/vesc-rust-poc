//! Refloat run-state transition decisions.
//!
//! Source map: upstream `check_faults` stops in
//! `third_party/refloat/src/main.c:357-509`; READY engage checks run in
//! `third_party/refloat/src/main.c:957-1067`; `state_stop` and `state_engage`
//! write the shared state fields in `third_party/refloat/src/state.c:29-39`.

use crate::domain::{
    RefloatMode, RefloatRideState, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};

/// Ordered stop event selected from the upstream fault checks.
///
/// Source map: each event mirrors a `state_stop` branch in
/// `third_party/refloat/src/main.c:357-509`; the resulting state write happens
/// in `third_party/refloat/src/state.c:29-33`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefloatStopEvent {
    FlywheelBothFootpads,
    ReverseStopNoFootpads,
    ReverseStopPitch,
    ReverseStopTimer,
    ReverseStopTotalErpm,
    FullSwitch,
    QuickStop,
    HalfSwitch,
    DarkrideHighErpm,
    DarkrideCanEngage,
    Roll,
    Pitch,
    DarkrideRoll,
}

impl RefloatStopEvent {
    #[inline(always)]
    #[must_use]
    pub(crate) const fn stop_condition(self) -> RefloatStopCondition {
        match self {
            Self::FlywheelBothFootpads | Self::HalfSwitch | Self::DarkrideCanEngage => {
                RefloatStopCondition::SwitchHalf
            }
            Self::ReverseStopNoFootpads | Self::FullSwitch => RefloatStopCondition::SwitchFull,
            Self::ReverseStopPitch
            | Self::ReverseStopTimer
            | Self::ReverseStopTotalErpm
            | Self::DarkrideHighErpm => RefloatStopCondition::ReverseStop,
            Self::QuickStop => RefloatStopCondition::QuickStop,
            Self::Roll | Self::DarkrideRoll => RefloatStopCondition::Roll,
            Self::Pitch => RefloatStopCondition::Pitch,
        }
    }
}

/// Pick the first active stop event in the call-site order.
///
/// Source map: upstream returns immediately from `check_faults` after each
/// `state_stop` at `third_party/refloat/src/main.c:357-509`.
#[inline(always)]
pub(crate) fn refloat_first_stop_event(
    events: &[(RefloatStopEvent, bool)],
) -> Option<RefloatStopEvent> {
    events
        .iter()
        .find_map(|(event, active)| active.then_some(*event))
}

/// Inputs needed to mirror Refloat's state-transition writes.
///
/// Source map: upstream combines `check_faults`, READY engage, flywheel abort,
/// and traction state in `third_party/refloat/src/main.c:357-509` and
/// `third_party/refloat/src/main.c:957-1067`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefloatStateTransitionInput {
    pub(crate) previous: RefloatRideState,
    pub(crate) run_state: RefloatRunState,
    pub(crate) ready_flywheel_stop: bool,
    pub(crate) state_engage: bool,
    pub(crate) traction_loss_detected: bool,
    pub(crate) stop_event: Option<RefloatStopEvent>,
}

/// Output state plus the timer-routing decisions owned by the caller.
///
/// Source map: `state_stop` refreshes disengage timing through `refloat_thd`
/// around `third_party/refloat/src/main.c:1071-1074`, while `engage(d)` refreshes
/// engage timing at `third_party/refloat/src/main.c:263-270`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefloatStateTransitionOutput {
    pub(crate) ride_state: RefloatRideState,
    pub(crate) state_stopped: bool,
    pub(crate) state_engaged: bool,
}

/// Apply Refloat's run-state writes after fault and engage decisions.
///
/// Source map: `state_stop` sets READY, stop condition, and clears wheelslip at
/// `third_party/refloat/src/state.c:29-33`; `state_engage` sets RUNNING,
/// `SAT_CENTERING`, and `STOP_NONE` at `third_party/refloat/src/state.c:36-39`;
/// READY flywheel abort returns to NORMAL before startup checks at
/// `third_party/refloat/src/main.c:957-963`.
#[inline(always)]
pub(crate) fn refloat_state_transition(
    input: RefloatStateTransitionInput,
) -> RefloatStateTransitionOutput {
    let state_stopped = input.stop_event.is_some();
    let state_engaged = !state_stopped && input.state_engage;
    let stop_condition = input.stop_event.map_or_else(
        || {
            if state_engaged {
                RefloatStopCondition::None
            } else {
                input.previous.stop_condition()
            }
        },
        RefloatStopEvent::stop_condition,
    );
    let run_state = match (state_stopped, state_engaged) {
        (true, _) => RefloatRunState::Ready,
        (false, true) => RefloatRunState::Running,
        (false, false) => input.run_state,
    };
    let setpoint_adjustment = if state_engaged {
        RefloatSetpointAdjustment::Centering
    } else {
        input.previous.setpoint_adjustment()
    };
    let wheelslip = if state_stopped {
        RefloatWheelSlipState::None
    } else if input.traction_loss_detected {
        RefloatWheelSlipState::Detected
    } else {
        input.previous.wheelslip()
    };
    let mode = if input.ready_flywheel_stop {
        RefloatMode::Normal
    } else {
        input.previous.mode()
    };
    let ride_state = RefloatRideState::new(run_state, mode, setpoint_adjustment, stop_condition)
        .with_charging(input.previous.charging())
        .with_wheelslip(wheelslip)
        .with_darkride(input.previous.darkride());

    RefloatStateTransitionOutput {
        ride_state,
        state_stopped,
        state_engaged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RefloatChargingState, RefloatDarkRideState};

    fn running_normal() -> RefloatRideState {
        RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        )
    }

    #[test]
    fn state_transition_selects_first_stop_event_like_refloat_check_faults() {
        let event = refloat_first_stop_event(&[
            (RefloatStopEvent::QuickStop, false),
            (RefloatStopEvent::HalfSwitch, true),
            (RefloatStopEvent::Pitch, true),
        ]);

        // Upstream returns immediately after the first active `state_stop` in
        // `third_party/refloat/src/main.c:357-509`.
        assert_eq!(event, Some(RefloatStopEvent::HalfSwitch));
    }

    #[test]
    fn state_transition_stop_wins_over_engage_like_refloat_state_stop() {
        let previous = running_normal().with_wheelslip(RefloatWheelSlipState::Detected);
        let output = refloat_state_transition(RefloatStateTransitionInput {
            previous,
            run_state: RefloatRunState::Running,
            ready_flywheel_stop: false,
            state_engage: true,
            traction_loss_detected: false,
            stop_event: Some(RefloatStopEvent::QuickStop),
        });

        // Upstream `state_stop` writes READY and clears wheelslip at
        // `third_party/refloat/src/state.c:29-33`; this takes precedence over
        // engage in the caller's ordered loop.
        assert_eq!(output.ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            output.ride_state.stop_condition(),
            RefloatStopCondition::QuickStop
        );
        assert_eq!(
            output.ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::None
        );
        assert_eq!(output.ride_state.wheelslip(), RefloatWheelSlipState::None);
        assert!(output.state_stopped);
        assert!(!output.state_engaged);
    }

    #[test]
    fn state_transition_engage_sets_running_centering_and_clears_stop_like_refloat() {
        let previous = RefloatRideState::new(
            RefloatRunState::Ready,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::Pitch,
        )
        .with_charging(RefloatChargingState::NotCharging);
        let output = refloat_state_transition(RefloatStateTransitionInput {
            previous,
            run_state: RefloatRunState::Ready,
            ready_flywheel_stop: false,
            state_engage: true,
            traction_loss_detected: false,
            stop_event: None,
        });

        // Upstream `state_engage` writes RUNNING, SAT_CENTERING, and STOP_NONE
        // at `third_party/refloat/src/state.c:36-39`.
        assert_eq!(output.ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            output.ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
        assert_eq!(
            output.ride_state.stop_condition(),
            RefloatStopCondition::None
        );
        assert!(!output.state_stopped);
        assert!(output.state_engaged);
    }

    #[test]
    fn state_transition_ready_flywheel_stop_returns_to_normal_like_refloat_ready_loop() {
        let previous = RefloatRideState::new(
            RefloatRunState::Ready,
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let output = refloat_state_transition(RefloatStateTransitionInput {
            previous,
            run_state: RefloatRunState::Ready,
            ready_flywheel_stop: true,
            state_engage: false,
            traction_loss_detected: false,
            stop_event: None,
        });

        // Upstream READY stops FLYWHEEL before startup checks at
        // `third_party/refloat/src/main.c:957-963`.
        assert_eq!(output.ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(output.ride_state.mode(), RefloatMode::Normal);
    }

    #[test]
    fn state_transition_marks_wheelslip_without_stopping_like_refloat_traction_flag() {
        let previous = running_normal().with_darkride(RefloatDarkRideState::Active);
        let output = refloat_state_transition(RefloatStateTransitionInput {
            previous,
            run_state: RefloatRunState::Running,
            ready_flywheel_stop: false,
            state_engage: false,
            traction_loss_detected: true,
            stop_event: None,
        });

        // Upstream detects traction loss in `third_party/refloat/src/main.c:551-562`;
        // freewheel happens later in `third_party/refloat/src/main.c:949-954`.
        assert_eq!(
            output.ride_state.wheelslip(),
            RefloatWheelSlipState::Detected
        );
        assert_eq!(output.ride_state.darkride(), RefloatDarkRideState::Active);
        assert!(!output.state_stopped);
    }
}
