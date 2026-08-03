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
    FlywheelFootpad,
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
            Self::FlywheelFootpad | Self::HalfSwitch | Self::DarkrideCanEngage => {
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
mod tests;
