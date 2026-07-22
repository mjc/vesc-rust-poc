//! Refloat state and alert domain types.
//!
//! C maps:
//! - `third_party/refloat/src/state.h:23-68` defines the core run/mode/stop state image.
//! - `third_party/refloat/src/main.c:61-80` defines the beep-reason IDs used in realtime data.
//! - `third_party/refloat/src/main.c:1927-1930` packs fatal/data-recorder bits into realtime extra flags.

/// Refloat top-level run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRunState {
    /// Package is disabled.
    Disabled,
    /// Package is starting up.
    Startup,
    /// Package is ready but not actively balancing.
    Ready,
    /// Package is actively running.
    Running,
}

impl RefloatRunState {
    /// Return the Refloat `v1.2.1` run-state ID.
    ///
    /// C map: `third_party/refloat/src/state.h:23-28`.
    pub const fn id(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Startup => 1,
            Self::Ready => 2,
            Self::Running => 3,
        }
    }
}

/// Refloat runtime mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatMode {
    /// Normal ride mode.
    Normal,
    /// Hand-test mode.
    HandTest,
    /// Flywheel mode.
    Flywheel,
}

impl RefloatMode {
    /// Return the Refloat `v1.2.1` mode ID.
    ///
    /// C map: `third_party/refloat/src/state.h:30-34`.
    pub const fn id(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::HandTest => 1,
            Self::Flywheel => 2,
        }
    }
}

/// Refloat stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatStopCondition {
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

impl RefloatStopCondition {
    /// Return the Refloat `v1.2.1` stop-condition ID.
    ///
    /// C map: `third_party/refloat/src/state.h:36-44`.
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

/// Refloat setpoint adjustment or pushback reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatSetpointAdjustment {
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

impl RefloatSetpointAdjustment {
    /// Return the Refloat `v1.2.1` realtime-data setpoint-adjustment ID.
    ///
    /// C map: `third_party/refloat/src/state.h:46-58`.
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

/// Refloat charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatChargingState {
    /// Not charging.
    NotCharging,
    /// Charging is active.
    Charging,
}

/// Refloat wheel-slip state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatWheelSlipState {
    /// No wheel slip detected.
    None,
    /// Wheel slip detected.
    Detected,
}

/// Refloat darkride/upside-down state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatDarkRideState {
    /// Board is upright.
    Upright,
    /// Darkride/upside-down state is active.
    Active,
}

/// Refloat beeper reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatBeepReason {
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

impl RefloatBeepReason {
    /// Return the Refloat `v1.2.1` beep-reason ID.
    ///
    /// C map: `third_party/refloat/src/main.c:61-80`.
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

/// Refloat data-recorder status flags sent in realtime data.
///
/// C map: upstream packs fatal/data-recorder bits into realtime `extra_flags`
/// at `third_party/refloat/src/main.c:1927-1930`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RefloatDataRecorderFlags {
    recording: bool,
    autostart: bool,
    autostop: bool,
}

impl RefloatDataRecorderFlags {
    /// Return inactive data-recorder flags.
    pub const fn inactive() -> Self {
        Self {
            recording: false,
            autostart: false,
            autostop: false,
        }
    }

    /// Return flags with recording enabled.
    pub const fn with_recording(mut self) -> Self {
        self.recording = true;
        self
    }

    /// Return flags with autostart enabled.
    pub const fn with_autostart(mut self) -> Self {
        self.autostart = true;
        self
    }

    /// Return flags with autostop enabled.
    pub const fn with_autostop(mut self) -> Self {
        self.autostop = true;
        self
    }

    pub(crate) const fn extra_flags_compat(self, fatal_error: RefloatFatalErrorState) -> u8 {
        let fatal = match fatal_error {
            RefloatFatalErrorState::None => 0,
            RefloatFatalErrorState::Present => 0x8,
        };
        let autostop = if self.autostop { 0x4 } else { 0 };
        let autostart = if self.autostart { 0x2 } else { 0 };
        let recording = if self.recording { 0x1 } else { 0 };
        fatal | autostop | autostart | recording
    }
}

/// Refloat fatal-error state for realtime-data extra flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatFatalErrorState {
    /// No fatal error is active.
    None,
    /// Fatal error is active.
    Present,
}
