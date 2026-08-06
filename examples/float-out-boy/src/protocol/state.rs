//! State and alert domain types owned by Float Out Boy.
//!
//! C maps:
//! - `third_party/float-out-boy/src/state.h:23-68` defines the core run/mode/stop state image.
//! - `third_party/float-out-boy/src/main.c:61-80` defines the beep-reason IDs used in realtime data.
//! - `third_party/float-out-boy/src/main.c:1927-1930` packs fatal/data-recorder bits into realtime extra flags.

// C map: IDs mirror `third_party/float-out-boy/src/state.h:23-58`.
vesc_protocol::wire_enum! {
    /// Float Out Boy top-level run state.
    #[derive(Default)]
    pub enum FloatOutBoyRunState {
        /// Package is disabled.
        Disabled = 0,
        /// Package is starting up.
        #[default]
        Startup = 1,
        /// Package is ready but not actively balancing.
        Ready = 2,
        /// Package is actively running.
        Running = 3,
    }
}

vesc_protocol::wire_enum! {
    /// Float Out Boy runtime mode.
    #[derive(Default)]
    pub enum FloatOutBoyMode {
        /// Normal ride mode.
        #[default]
        Normal = 0,
        /// Hand-test mode.
        HandTest = 1,
        /// Flywheel mode.
        Flywheel = 2,
    }
}

vesc_protocol::wire_enum! {
    /// Float Out Boy stop reason.
    #[derive(Default)]
    pub enum FloatOutBoyStopCondition {
        /// No stop condition is active.
        #[default]
        None = 0,
        /// Pitch angle fault.
        Pitch = 1,
        /// Roll angle fault.
        Roll = 2,
        /// Half-switch fault.
        SwitchHalf = 3,
        /// Full-switch fault.
        SwitchFull = 4,
        /// Reverse-stop fault.
        ReverseStop = 5,
        /// Quickstop fault.
        QuickStop = 6,
    }
}

vesc_protocol::wire_enum! {
    /// Float Out Boy setpoint adjustment or pushback reason.
    #[derive(Default)]
    pub enum FloatOutBoySetpointAdjustment {
        /// No adjustment.
        #[default]
        None = 0,
        /// Centering adjustment.
        Centering = 1,
        /// Reverse-stop adjustment.
        ReverseStop = 2,
        /// Pushback from speed limit.
        PushbackSpeed = 5,
        /// Pushback from duty limit.
        PushbackDuty = 6,
        /// Pushback from error state.
        PushbackError = 7,
        /// Pushback from high voltage.
        PushbackHighVoltage = 10,
        /// Pushback from low voltage.
        PushbackLowVoltage = 11,
        /// Pushback from temperature.
        PushbackTemperature = 12,
    }
}

impl FloatOutBoySetpointAdjustment {
    /// Return whether centering or reverse-stop control owns the setpoint.
    #[must_use]
    pub const fn is_centering_or_reverse_stop(self) -> bool {
        matches!(self, Self::Centering | Self::ReverseStop)
    }

    /// Return whether any pushback condition owns the setpoint.
    #[must_use]
    pub const fn is_pushback(self) -> bool {
        matches!(
            self,
            Self::PushbackSpeed
                | Self::PushbackDuty
                | Self::PushbackError
                | Self::PushbackHighVoltage
                | Self::PushbackLowVoltage
                | Self::PushbackTemperature
        )
    }

    /// Return whether the compatibility float state reports tiltback.
    #[must_use]
    pub const fn is_float_state_tiltback(self) -> bool {
        matches!(
            self,
            Self::PushbackError
                | Self::PushbackHighVoltage
                | Self::PushbackLowVoltage
                | Self::PushbackTemperature
        )
    }
}

/// Float Out Boy charging state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyChargingState {
    /// Not charging.
    #[default]
    NotCharging,
    /// Charging is active.
    Charging,
}

/// Float Out Boy wheel-slip state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyWheelSlipState {
    /// No wheel slip detected.
    #[default]
    None,
    /// Wheel slip detected.
    Detected,
}

/// Float Out Boy darkride/upside-down state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyDarkRideState {
    /// Board is upright.
    #[default]
    Upright,
    /// Darkride/upside-down state is active.
    Active,
}

// C map: IDs mirror `third_party/float-out-boy/src/main.c:61-80`.
vesc_protocol::wire_enum! {
    /// Float Out Boy beeper reason.
    #[derive(Default)]
    pub enum FloatOutBoyBeepReason {
        /// No beep reason.
        #[default]
        None = 0,
        /// Low-voltage warning.
        LowVoltage = 1,
        /// High-voltage warning.
        HighVoltage = 2,
        /// MOSFET temperature warning.
        MosfetTemperature = 3,
        /// Motor temperature warning.
        MotorTemperature = 4,
        /// Current warning.
        Current = 5,
        /// Duty-cycle warning.
        Duty = 6,
        /// Footpad sensor warning.
        Sensors = 7,
        /// Low battery warning.
        LowBattery = 8,
        /// Idle warning.
        Idle = 9,
        /// Generic error warning.
        Error = 10,
        /// Speed warning.
        Speed = 11,
        /// BMS cell under-temperature warning.
        CellUnderTemperature = 12,
        /// BMS cell over-temperature warning.
        CellOverTemperature = 13,
        /// BMS low-cell-voltage warning.
        CellLowVoltage = 14,
        /// BMS high-cell-voltage warning.
        CellHighVoltage = 15,
        /// BMS cell-balance warning.
        CellBalance = 16,
        /// BMS connection warning.
        BmsConnection = 17,
        /// BMS over-temperature warning.
        BmsOverTemperature = 18,
        /// Firmware fault warning.
        FirmwareFault = 19,
    }
}

/// Float Out Boy name for the standard VESC package recorder status bits.
pub use vescpkg_rs::DataRecorderFlags as FloatOutBoyDataRecorderFlags;

/// Float Out Boy fatal-error state for realtime-data extra flags.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyFatalErrorState {
    /// No fatal error is active.
    #[default]
    None,
    /// Fatal error is active.
    Present,
}
