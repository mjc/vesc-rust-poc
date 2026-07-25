//! Float Out Boy run-state transition decisions.
//!
//! Source map: upstream `check_faults` stops in
//! `third_party/float-out-boy/src/main.c:357-509`; READY engage checks run in
//! `third_party/float-out-boy/src/main.c:957-1067`; `state_stop` and `state_engage`
//! write the shared state fields in `third_party/float-out-boy/src/state.c:29-39`.

use crate::domain::{
    FloatOutBoyMode, FloatOutBoyRideState, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};

/// Ordered stop event selected from the upstream fault checks.
///
/// Source map: each event mirrors a `state_stop` branch in
/// `third_party/float-out-boy/src/main.c:357-509`; the resulting state write happens
/// in `third_party/float-out-boy/src/state.c:29-33`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyStopEvent {
    FlywheelBothFootpads,
    ReverseStopNoFootpads,
    ReverseStopPitch,
    ReverseStopTimer,
    ReverseStopTotalErpm,
    FullSwitch,
    QuickStop,
    HalfSwitch,
    DarkrideHighErpm,
    DarkrideLowErpm,
    DarkrideCanEngage,
    Roll,
    Pitch,
    DarkrideRoll,
}

impl FloatOutBoyStopEvent {
    #[inline]
    #[must_use]
    pub(crate) const fn stop_condition(self) -> FloatOutBoyStopCondition {
        // C map: `state_stop` chooses the stored stop condition from the
        // active fault branch at `third_party/float-out-boy/src/state.c:29-33`.
        match self {
            Self::FlywheelBothFootpads | Self::HalfSwitch | Self::DarkrideCanEngage => {
                FloatOutBoyStopCondition::SwitchHalf
            }
            Self::ReverseStopNoFootpads | Self::FullSwitch => FloatOutBoyStopCondition::SwitchFull,
            Self::ReverseStopPitch
            | Self::ReverseStopTimer
            | Self::ReverseStopTotalErpm
            | Self::DarkrideHighErpm
            | Self::DarkrideLowErpm => FloatOutBoyStopCondition::ReverseStop,
            Self::QuickStop => FloatOutBoyStopCondition::QuickStop,
            Self::Roll | Self::DarkrideRoll => FloatOutBoyStopCondition::Roll,
            Self::Pitch => FloatOutBoyStopCondition::Pitch,
        }
    }
}

/// Pick the first active stop event in the call-site order.
///
/// Source map: upstream returns immediately from `check_faults` after each
/// `state_stop` at `third_party/float-out-boy/src/main.c:357-509`.
#[inline]
pub(crate) fn float_out_boy_first_stop_event(
    events: &[(FloatOutBoyStopEvent, bool)],
) -> Option<FloatOutBoyStopEvent> {
    events
        .iter()
        .find_map(|(event, active)| active.then_some(*event))
}

/// Inputs needed to mirror Float Out Boy's state-transition writes.
///
/// Source map: upstream combines `check_faults`, READY engage, flywheel abort,
/// and traction state in `third_party/float-out-boy/src/main.c:357-509` and
/// `third_party/float-out-boy/src/main.c:957-1067`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyStateTransitionInput {
    pub(crate) previous: FloatOutBoyRideState,
    pub(crate) run_state: FloatOutBoyRunState,
    pub(crate) ready_flywheel_stop: bool,
    pub(crate) state_engage: bool,
    pub(crate) traction_loss_detected: bool,
    pub(crate) stop_event: Option<FloatOutBoyStopEvent>,
}

/// Output state plus the timer-routing decisions owned by the caller.
///
/// Source map: `state_stop` refreshes disengage timing through `float_out_boy_thd`
/// around `third_party/float-out-boy/src/main.c:1071-1074`, while `engage(d)` refreshes
/// engage timing at `third_party/float-out-boy/src/main.c:263-270`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyStateTransitionOutput {
    pub(crate) ride_state: FloatOutBoyRideState,
    pub(crate) state_stopped: bool,
    pub(crate) state_engaged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyStateTransitionAction {
    Stop(FloatOutBoyStopEvent),
    Engage,
    Preserve,
}

impl FloatOutBoyStateTransitionAction {
    #[inline]
    fn select(input: &FloatOutBoyStateTransitionInput) -> Self {
        // C map: upstream evaluates stop checks before READY engage, and
        // then preserves state only when no stop and no engage path fires at
        // `third_party/float-out-boy/src/main.c:357-509` and
        // `third_party/float-out-boy/src/main.c:957-1067`.
        match (input.stop_event, input.state_engage) {
            (Some(event), _) => Self::Stop(event),
            (None, true) => Self::Engage,
            (None, false) => Self::Preserve,
        }
    }

    #[inline]
    fn apply(self, input: FloatOutBoyStateTransitionInput) -> FloatOutBoyStateTransitionOutput {
        let previous = input.previous;
        // C map: `state_stop` writes READY/stop condition and clears wheelslip at
        // `third_party/float-out-boy/src/state.c:29-33`; `state_engage` writes RUNNING,
        // SAT_CENTERING, and STOP_NONE at `third_party/float-out-boy/src/state.c:36-39`.
        let (
            run_state,
            setpoint_adjustment,
            stop_condition,
            wheelslip,
            state_stopped,
            state_engaged,
        ) = match self {
            Self::Stop(event) => (
                FloatOutBoyRunState::Ready,
                previous.setpoint_adjustment(),
                event.stop_condition(),
                FloatOutBoyWheelSlipState::None,
                true,
                false,
            ),
            Self::Engage => (
                FloatOutBoyRunState::Running,
                FloatOutBoySetpointAdjustment::Centering,
                FloatOutBoyStopCondition::None,
                Self::rolling_wheelslip(previous, input.traction_loss_detected),
                false,
                true,
            ),
            Self::Preserve => (
                input.run_state,
                Self::rolling_setpoint_adjustment(previous, input.traction_loss_detected),
                previous.stop_condition(),
                Self::rolling_wheelslip(previous, input.traction_loss_detected),
                false,
                false,
            ),
        };

        FloatOutBoyStateTransitionOutput {
            ride_state: FloatOutBoyRideState::new(
                run_state,
                Self::mode_after_ready_check(input),
                setpoint_adjustment,
                stop_condition,
            )
            .with_charging(previous.charging())
            .with_wheelslip(wheelslip)
            .with_darkride(previous.darkride()),
            state_stopped,
            state_engaged,
        }
    }

    #[inline]
    fn rolling_setpoint_adjustment(
        previous: FloatOutBoyRideState,
        traction_loss_detected: bool,
    ) -> FloatOutBoySetpointAdjustment {
        // Float Out Boy clears `sat` on the same branch that marks wheelslip at
        // `third_party/float-out-boy/src/main.c:551-562`.
        if traction_loss_detected {
            FloatOutBoySetpointAdjustment::None
        } else {
            previous.setpoint_adjustment()
        }
    }

    #[inline]
    fn mode_after_ready_check(input: FloatOutBoyStateTransitionInput) -> FloatOutBoyMode {
        // C map: READY flywheel abort calls `flywheel_stop(d)` before startup checks at
        // `third_party/float-out-boy/src/main.c:957-963`; `flywheel_stop` returns mode to NORMAL at
        // `third_party/float-out-boy/src/main.c:1869-1873`.
        if input.ready_flywheel_stop {
            FloatOutBoyMode::Normal
        } else {
            input.previous.mode()
        }
    }

    #[inline]
    fn rolling_wheelslip(
        previous: FloatOutBoyRideState,
        traction_loss_detected: bool,
    ) -> FloatOutBoyWheelSlipState {
        // C map: wheelslip is set in the runtime setpoint path at
        // `third_party/float-out-boy/src/main.c:551-574` and cleared only by
        // `state_stop` or the later traction-control clear path.
        if traction_loss_detected {
            FloatOutBoyWheelSlipState::Detected
        } else {
            previous.wheelslip()
        }
    }
}

/// Apply Float Out Boy's run-state writes after fault and engage decisions.
///
/// Source map: `state_stop` sets READY, stop condition, and clears wheelslip at
/// `third_party/float-out-boy/src/state.c:29-33`; `state_engage` sets RUNNING,
/// `SAT_CENTERING`, and `STOP_NONE` at `third_party/float-out-boy/src/state.c:36-39`;
/// READY flywheel abort returns to NORMAL before startup checks at
/// `third_party/float-out-boy/src/main.c:957-963` via `third_party/float-out-boy/src/main.c:1869-1873`.
#[inline]
pub(crate) fn float_out_boy_state_transition(
    input: FloatOutBoyStateTransitionInput,
) -> FloatOutBoyStateTransitionOutput {
    FloatOutBoyStateTransitionAction::select(&input).apply(input)
}

#[cfg(test)]
mod tests {
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
    fn state_transition_action_selects_stop_engage_or_preserve() {
        let stop_input = FloatOutBoyStateTransitionInput {
            state_engage: true,
            stop_event: Some(FloatOutBoyStopEvent::QuickStop),
            ..transition_input(running_normal())
        };
        let engage_input = FloatOutBoyStateTransitionInput {
            state_engage: true,
            ..transition_input(running_normal())
        };
        let preserve_input = transition_input(running_normal());

        assert_eq!(
            FloatOutBoyStateTransitionAction::select(&stop_input),
            FloatOutBoyStateTransitionAction::Stop(FloatOutBoyStopEvent::QuickStop)
        );
        assert_eq!(
            FloatOutBoyStateTransitionAction::select(&engage_input),
            FloatOutBoyStateTransitionAction::Engage
        );
        assert_eq!(
            FloatOutBoyStateTransitionAction::select(&preserve_input),
            FloatOutBoyStateTransitionAction::Preserve
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
}
