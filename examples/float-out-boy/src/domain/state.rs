//! Float Out Boy state and alert domain types.
//!
//! C maps:
//! - `third_party/float-out-boy/src/state.h:23-68` defines the core run/mode/stop state image.
//! - `third_party/float-out-boy/src/main.c:61-80` defines the beep-reason IDs used in realtime data.
//! - `third_party/float-out-boy/src/main.c:1927-1930` packs fatal/data-recorder bits into realtime extra flags.

/// Float Out Boy top-level run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyRunState {
    /// Package is disabled.
    Disabled,
    /// Package is starting up.
    Startup,
    /// Package is ready but not actively balancing.
    Ready,
    /// Package is actively running.
    Running,
}

impl FloatOutBoyRunState {
    /// Return the Float Out Boy `v1.2.1` run-state ID.
    ///
    /// C map: `third_party/float-out-boy/src/state.h:23-28`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Startup => 1,
            Self::Ready => 2,
            Self::Running => 3,
        }
    }
}

/// Float Out Boy runtime mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyMode {
    /// Normal ride mode.
    Normal,
    /// Hand-test mode.
    HandTest,
    /// Flywheel mode.
    Flywheel,
}

impl FloatOutBoyMode {
    /// Return the Float Out Boy `v1.2.1` mode ID.
    ///
    /// C map: `third_party/float-out-boy/src/state.h:30-34`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::HandTest => 1,
            Self::Flywheel => 2,
        }
    }
}

/// Float Out Boy stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyStopCondition {
    /// No stop condition is active.
    None,
    /// Pitch angle fault.
    Pitch,
    /// Roll angle fault.
    Roll,
    /// Half-switch fault.
    SwitchHalf,
    /// Full-switch fault.
    SwitchFull,
    /// Reverse-stop fault.
    ReverseStop,
    /// Quickstop fault.
    QuickStop,
}

impl FloatOutBoyStopCondition {
    /// Return the Float Out Boy `v1.2.1` stop-condition ID.
    ///
    /// C map: `third_party/float-out-boy/src/state.h:36-44`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Pitch => 1,
            Self::Roll => 2,
            Self::SwitchHalf => 3,
            Self::SwitchFull => 4,
            Self::ReverseStop => 5,
            Self::QuickStop => 6,
        }
    }
}

/// Float Out Boy setpoint adjustment or pushback reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoySetpointAdjustment {
    /// No adjustment.
    None,
    /// Centering adjustment.
    Centering,
    /// Reverse-stop adjustment.
    ReverseStop,
    /// Pushback from speed limit.
    PushbackSpeed,
    /// Pushback from duty limit.
    PushbackDuty,
    /// Pushback from error state.
    PushbackError,
    /// Pushback from high voltage.
    PushbackHighVoltage,
    /// Pushback from low voltage.
    PushbackLowVoltage,
    /// Pushback from temperature.
    PushbackTemperature,
}

impl FloatOutBoySetpointAdjustment {
    /// Return the Float Out Boy `v1.2.1` realtime-data setpoint-adjustment ID.
    ///
    /// C map: `third_party/float-out-boy/src/state.h:46-58`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Centering => 1,
            Self::ReverseStop => 2,
            Self::PushbackSpeed => 5,
            Self::PushbackDuty => 6,
            Self::PushbackError => 7,
            Self::PushbackHighVoltage => 10,
            Self::PushbackLowVoltage => 11,
            Self::PushbackTemperature => 12,
        }
    }

    pub(crate) const fn is_float_state_tiltback(self) -> bool {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyChargingState {
    /// Not charging.
    NotCharging,
    /// Charging is active.
    Charging,
}

/// Float Out Boy wheel-slip state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyWheelSlipState {
    /// No wheel slip detected.
    None,
    /// Wheel slip detected.
    Detected,
}

/// Float Out Boy darkride/upside-down state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyDarkRideState {
    /// Board is upright.
    Upright,
    /// Darkride/upside-down state is active.
    Active,
}

/// Float Out Boy beeper reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyBeepReason {
    /// No beep reason.
    None,
    /// Low-voltage warning.
    LowVoltage,
    /// High-voltage warning.
    HighVoltage,
    /// MOSFET temperature warning.
    MosfetTemperature,
    /// Motor temperature warning.
    MotorTemperature,
    /// Current warning.
    Current,
    /// Duty-cycle warning.
    Duty,
    /// Footpad sensor warning.
    Sensors,
    /// Low battery warning.
    LowBattery,
    /// Idle warning.
    Idle,
    /// Generic error warning.
    Error,
    /// Speed warning.
    Speed,
    /// BMS cell under-temperature warning.
    CellUnderTemperature,
    /// BMS cell over-temperature warning.
    CellOverTemperature,
    /// BMS low-cell-voltage warning.
    CellLowVoltage,
    /// BMS high-cell-voltage warning.
    CellHighVoltage,
    /// BMS cell-balance warning.
    CellBalance,
    /// BMS connection warning.
    BmsConnection,
    /// BMS over-temperature warning.
    BmsOverTemperature,
    /// Firmware fault warning.
    FirmwareFault,
}

impl FloatOutBoyBeepReason {
    /// Return the Float Out Boy `v1.2.1` beep-reason ID.
    ///
    /// C map: `third_party/float-out-boy/src/main.c:61-80`.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::LowVoltage => 1,
            Self::HighVoltage => 2,
            Self::MosfetTemperature => 3,
            Self::MotorTemperature => 4,
            Self::Current => 5,
            Self::Duty => 6,
            Self::Sensors => 7,
            Self::LowBattery => 8,
            Self::Idle => 9,
            Self::Error => 10,
            Self::Speed => 11,
            Self::CellUnderTemperature => 12,
            Self::CellOverTemperature => 13,
            Self::CellLowVoltage => 14,
            Self::CellHighVoltage => 15,
            Self::CellBalance => 16,
            Self::BmsConnection => 17,
            Self::BmsOverTemperature => 18,
            Self::FirmwareFault => 19,
        }
    }
}

/// Float Out Boy data-recorder status flags sent in realtime data.
///
/// C map: upstream packs fatal/data-recorder bits into realtime `extra_flags`
/// at `third_party/float-out-boy/src/main.c:1927-1930`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyDataRecorderFlags {
    recording: bool,
    autostart: bool,
    autostop: bool,
}

impl FloatOutBoyDataRecorderFlags {
    /// Return inactive data-recorder flags.
    #[must_use]
    pub const fn inactive() -> Self {
        Self {
            recording: false,
            autostart: false,
            autostop: false,
        }
    }

    /// Return flags with recording enabled.
    #[must_use]
    pub const fn with_recording(mut self) -> Self {
        self.recording = true;
        self
    }

    /// Return flags with autostart enabled.
    #[must_use]
    pub const fn with_autostart(mut self) -> Self {
        self.autostart = true;
        self
    }

    /// Return flags with autostop enabled.
    #[must_use]
    pub const fn with_autostop(mut self) -> Self {
        self.autostop = true;
        self
    }

    pub(crate) const fn extra_flags_compat(self, fatal_error: FloatOutBoyFatalErrorState) -> u8 {
        let fatal = match fatal_error {
            FloatOutBoyFatalErrorState::None => 0,
            FloatOutBoyFatalErrorState::Present => 0x8,
        };
        let autostop = if self.autostop { 0x4 } else { 0 };
        let autostart = if self.autostart { 0x2 } else { 0 };
        let recording = if self.recording { 0x1 } else { 0 };
        fatal | autostop | autostart | recording
    }
}

/// Float Out Boy fatal-error state for realtime-data extra flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyFatalErrorState {
    /// No fatal error is active.
    None,
    /// Fatal error is active.
    Present,
}
