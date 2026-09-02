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
    let previous = input.previous;
    let rolling_wheelslip = if input.traction_loss_detected {
        FloatOutBoyWheelSlipState::Detected
    } else {
        previous.wheelslip()
    };
    let mode = if input.ready_flywheel_stop {
        FloatOutBoyMode::Normal
    } else {
        previous.mode()
    };
    let mut ride_state = previous
        .with_run_state(input.run_state)
        .with_mode(mode)
        .with_wheelslip(rolling_wheelslip);
    // C map: stop checks win over READY engagement at
    // `third_party/float-out-boy/src/main.c:357-509,957-1067`.
    let (state_stopped, state_engaged) = if let Some(event) = input.stop_event {
        ride_state = ride_state
            .with_run_state(FloatOutBoyRunState::Ready)
            .with_stop_condition(event.stop_condition())
            .with_wheelslip(FloatOutBoyWheelSlipState::None);
        (true, false)
    } else if input.state_engage {
        ride_state = ride_state
            .with_run_state(FloatOutBoyRunState::Running)
            .with_setpoint_adjustment(FloatOutBoySetpointAdjustment::Centering)
            .with_stop_condition(FloatOutBoyStopCondition::None);
        (false, true)
    } else {
        if input.traction_loss_detected {
            ride_state = ride_state.with_setpoint_adjustment(FloatOutBoySetpointAdjustment::None);
        }
        (false, false)
    };

    FloatOutBoyStateTransitionOutput {
        ride_state,
        state_stopped,
        state_engaged,
    }
}

#[cfg(test)]
mod tests;
