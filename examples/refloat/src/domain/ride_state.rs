//! Refloat ride-state compatibility mapping.
//!
//! Source map: compact all-data uses `state_compat`/`sat_compat` at
//! `third_party/refloat/src/main.c:1279-1285` and
//! `third_party/refloat/src/main.c:1333-1341`; realtime data packs the typed
//! state directly at `third_party/refloat/src/main.c:1934-1939`.

use super::{
    RefloatChargingState, RefloatDarkRideState, RefloatMode, RefloatRunState,
    RefloatSetpointAdjustment, RefloatStopCondition, RefloatWheelSlipState,
};

/// Refloat ride state as typed package-domain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRideState {
    run_state: RefloatRunState,
    mode: RefloatMode,
    setpoint_adjustment: RefloatSetpointAdjustment,
    stop_condition: RefloatStopCondition,
    charging: RefloatChargingState,
    wheelslip: RefloatWheelSlipState,
    darkride: RefloatDarkRideState,
}

impl RefloatRideState {
    /// Build a Refloat ride state from required enum-shaped state.
    pub const fn new(
        run_state: RefloatRunState,
        mode: RefloatMode,
        setpoint_adjustment: RefloatSetpointAdjustment,
        stop_condition: RefloatStopCondition,
    ) -> Self {
        Self {
            run_state,
            mode,
            setpoint_adjustment,
            stop_condition,
            charging: RefloatChargingState::NotCharging,
            wheelslip: RefloatWheelSlipState::None,
            darkride: RefloatDarkRideState::Upright,
        }
    }

    /// Return this state with the requested charging state.
    pub const fn with_charging(mut self, charging: RefloatChargingState) -> Self {
        self.charging = charging;
        self
    }

    /// Return this state with the requested wheel-slip state.
    pub const fn with_wheelslip(mut self, wheelslip: RefloatWheelSlipState) -> Self {
        self.wheelslip = wheelslip;
        self
    }

    /// Return this state with the requested darkride/upside-down state.
    pub const fn with_darkride(mut self, darkride: RefloatDarkRideState) -> Self {
        self.darkride = darkride;
        self
    }

    /// Return this state with the requested setpoint adjustment.
    pub(crate) const fn with_setpoint_adjustment(
        mut self,
        setpoint_adjustment: RefloatSetpointAdjustment,
    ) -> Self {
        self.setpoint_adjustment = setpoint_adjustment;
        self
    }

    /// Return the top-level run state.
    ///
    /// Mirrors upstream `d->state.state`, read by `set_cfg` at
    /// `third_party/refloat/src/main.c:2369-2372`.
    pub const fn run_state(self) -> RefloatRunState {
        self.run_state
    }

    /// Return the runtime mode.
    ///
    /// Mirrors upstream `d->state.mode`, read by `set_cfg` at
    /// `third_party/refloat/src/main.c:2362-2365`.
    pub const fn mode(self) -> RefloatMode {
        self.mode
    }

    /// Return the setpoint adjustment/pushback state.
    pub const fn setpoint_adjustment(self) -> RefloatSetpointAdjustment {
        self.setpoint_adjustment
    }

    /// Return the stop condition.
    pub const fn stop_condition(self) -> RefloatStopCondition {
        self.stop_condition
    }

    /// Return the charging state.
    pub const fn charging(self) -> RefloatChargingState {
        self.charging
    }

    /// Return the wheel-slip state.
    pub const fn wheelslip(self) -> RefloatWheelSlipState {
        self.wheelslip
    }

    /// Return the darkride/upside-down state.
    pub const fn darkride(self) -> RefloatDarkRideState {
        self.darkride
    }

    /// Return the Refloat app-data Float State compatibility value.
    pub const fn float_state_compat(self) -> u8 {
        if matches!(self.charging, RefloatChargingState::Charging) {
            return 14;
        }

        match self.run_state {
            RefloatRunState::Disabled => 15,
            RefloatRunState::Startup => 0,
            RefloatRunState::Ready => self.ready_float_state_compat(),
            RefloatRunState::Running => self.running_float_state_compat(),
        }
    }

    /// Return the Refloat app-data setpoint-adjustment compatibility value.
    pub const fn setpoint_adjustment_compat(self) -> u8 {
        match self.setpoint_adjustment {
            RefloatSetpointAdjustment::Centering => 0,
            RefloatSetpointAdjustment::ReverseStop => 1,
            RefloatSetpointAdjustment::None => 2,
            RefloatSetpointAdjustment::PushbackDuty => 3,
            RefloatSetpointAdjustment::PushbackHighVoltage => 4,
            RefloatSetpointAdjustment::PushbackLowVoltage => 5,
            RefloatSetpointAdjustment::PushbackTemperature => 6,
            RefloatSetpointAdjustment::PushbackSpeed => 7,
            RefloatSetpointAdjustment::PushbackError => 8,
        }
    }

    const fn ready_float_state_compat(self) -> u8 {
        match self.stop_condition {
            RefloatStopCondition::None => 11,
            RefloatStopCondition::Pitch => 6,
            RefloatStopCondition::Roll => 7,
            RefloatStopCondition::SwitchHalf => 8,
            RefloatStopCondition::SwitchFull => 9,
            RefloatStopCondition::ReverseStop => 12,
            RefloatStopCondition::QuickStop => 13,
        }
    }

    const fn running_float_state_compat(self) -> u8 {
        if self.setpoint_adjustment.is_float_state_tiltback() {
            2
        } else if matches!(self.wheelslip, RefloatWheelSlipState::Detected) {
            3
        } else if matches!(self.darkride, RefloatDarkRideState::Active) {
            4
        } else if matches!(self.mode, RefloatMode::Flywheel) {
            5
        } else {
            1
        }
    }
}
