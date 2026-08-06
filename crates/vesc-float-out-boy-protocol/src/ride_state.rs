//! Float Out Boy ride-state compatibility mapping.
//!
//! Source map: compact all-data uses `state_compat`/`sat_compat` at
//! `third_party/float-out-boy/src/main.c:1279-1285` and
//! `third_party/float-out-boy/src/main.c:1333-1341`; realtime data packs the typed
//! state directly at `third_party/float-out-boy/src/main.c:1934-1939`.

use super::{
    FloatOutBoyChargingState, FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};

/// Float Out Boy ride state as typed package-domain values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyRideState {
    run_state: FloatOutBoyRunState,
    mode: FloatOutBoyMode,
    setpoint_adjustment: FloatOutBoySetpointAdjustment,
    stop_condition: FloatOutBoyStopCondition,
    charging: FloatOutBoyChargingState,
    wheelslip: FloatOutBoyWheelSlipState,
    darkride: FloatOutBoyDarkRideState,
}

impl FloatOutBoyRideState {
    /// Build a Float Out Boy ride state from required enum-shaped state.
    #[must_use]
    pub const fn new(
        run_state: FloatOutBoyRunState,
        mode: FloatOutBoyMode,
        setpoint_adjustment: FloatOutBoySetpointAdjustment,
        stop_condition: FloatOutBoyStopCondition,
    ) -> Self {
        Self {
            run_state,
            mode,
            setpoint_adjustment,
            stop_condition,
            charging: FloatOutBoyChargingState::NotCharging,
            wheelslip: FloatOutBoyWheelSlipState::None,
            darkride: FloatOutBoyDarkRideState::Upright,
        }
    }

    vesc_protocol::const_field_builders! {
        /// Return this state with the requested charging state.
        pub fn with_charging(charging: FloatOutBoyChargingState) => charging;
        /// Return this state with the requested wheel-slip state.
        pub fn with_wheelslip(wheelslip: FloatOutBoyWheelSlipState) => wheelslip;
        /// Return this state with the requested darkride/upside-down state.
        pub fn with_darkride(darkride: FloatOutBoyDarkRideState) => darkride;
        /// Return this state with the requested setpoint adjustment.
        pub fn with_setpoint_adjustment(setpoint_adjustment: FloatOutBoySetpointAdjustment) => setpoint_adjustment;
    }

    vesc_protocol::const_field_getters! {
        /// Return the top-level run state.
        ///
        /// Mirrors upstream `d->state.state`, read by `set_cfg` at
        /// `third_party/float-out-boy/src/main.c:2369-2372`.
        pub fn run_state -> FloatOutBoyRunState = run_state;
        /// Return the runtime mode.
        ///
        /// Mirrors upstream `d->state.mode`, read by `set_cfg` at
        /// `third_party/float-out-boy/src/main.c:2362-2365`.
        pub fn mode -> FloatOutBoyMode = mode;
        /// Return the setpoint adjustment/pushback state.
        pub fn setpoint_adjustment -> FloatOutBoySetpointAdjustment = setpoint_adjustment;
        /// Return the stop condition.
        pub fn stop_condition -> FloatOutBoyStopCondition = stop_condition;
        /// Return the charging state.
        pub fn charging -> FloatOutBoyChargingState = charging;
        /// Return the wheel-slip state.
        pub fn wheelslip -> FloatOutBoyWheelSlipState = wheelslip;
        /// Return the darkride/upside-down state.
        pub fn darkride -> FloatOutBoyDarkRideState = darkride;
    }

    /// Return the Float Out Boy app-data Float State compatibility value.
    #[must_use]
    pub const fn float_state_compat(self) -> u8 {
        if matches!(self.charging, FloatOutBoyChargingState::Charging) {
            return 14;
        }

        match self.run_state {
            FloatOutBoyRunState::Disabled => 15,
            FloatOutBoyRunState::Startup => 0,
            FloatOutBoyRunState::Ready => self.ready_float_state_compat(),
            FloatOutBoyRunState::Running => self.running_float_state_compat(),
        }
    }

    /// Return the Float Out Boy app-data setpoint-adjustment compatibility value.
    #[must_use]
    pub const fn setpoint_adjustment_compat(self) -> u8 {
        match self.setpoint_adjustment {
            FloatOutBoySetpointAdjustment::Centering => 0,
            FloatOutBoySetpointAdjustment::ReverseStop => 1,
            FloatOutBoySetpointAdjustment::None => 2,
            FloatOutBoySetpointAdjustment::PushbackDuty => 3,
            FloatOutBoySetpointAdjustment::PushbackHighVoltage => 4,
            FloatOutBoySetpointAdjustment::PushbackLowVoltage => 5,
            FloatOutBoySetpointAdjustment::PushbackTemperature => 6,
            FloatOutBoySetpointAdjustment::PushbackSpeed => 7,
            FloatOutBoySetpointAdjustment::PushbackError => 8,
        }
    }

    const fn ready_float_state_compat(self) -> u8 {
        match self.stop_condition {
            FloatOutBoyStopCondition::None => 11,
            FloatOutBoyStopCondition::Pitch => 6,
            FloatOutBoyStopCondition::Roll => 7,
            FloatOutBoyStopCondition::SwitchHalf => 8,
            FloatOutBoyStopCondition::SwitchFull => 9,
            FloatOutBoyStopCondition::ReverseStop => 12,
            FloatOutBoyStopCondition::QuickStop => 13,
        }
    }

    const fn running_float_state_compat(self) -> u8 {
        if self.setpoint_adjustment.is_float_state_tiltback() {
            2
        } else if matches!(self.wheelslip, FloatOutBoyWheelSlipState::Detected) {
            3
        } else if matches!(self.darkride, FloatOutBoyDarkRideState::Active) {
            4
        } else if matches!(self.mode, FloatOutBoyMode::Flywheel) {
            5
        } else {
            1
        }
    }
}
