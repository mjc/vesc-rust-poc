//! Refloat-specific ride-domain types.
//!
//! These types compose the reusable `vescpkg-rs` package-author units and
//! semantic wrappers into Refloat concepts. Raw firmware/app-data primitives
//! should stay at explicit boundary conversions.
//!
//! Source anchors for the compatibility surface below are Refloat `v1.2.1`
//! (`0ef6e99d8701`):
//! - `src/main.c:1241-1262` defines the core app-data command IDs.
//! - `src/lcm.h:27-33` and `src/charging.h:25` define LCM/charging command IDs.
//! - `src/main.c:1313-1399` defines `COMMAND_GET_ALLDATA` response layout.
//! - `src/main.c:1876-1901` defines realtime-data ID-list packet layout.
//! - `src/main.c:1190-1205` defines startup `Data` initialization order.

use vescpkg_rs::prelude::{
    AdcDecodedLevel, AmpHoursCharged, AmpHoursDischarged, AngleDegrees, AngleRadians,
    BatteryCurrent, BatteryLevel, BatteryVoltage, Charge, Current, DirectionalMotorCurrent,
    Distance, DutyCycle, ElectricalSpeed, Energy, ImuAngularRate, ImuPitch, ImuRoll, ImuYaw,
    MosfetTemperature, MotorCurrent, MotorTemperature, OdometerMeters, Ratio, Rpm, SignedRatio,
    Speed, SystemTimestamp, Temperature, TripDistance, VehicleSpeed, Voltage, WattHoursCharged,
    WattHoursDischarged,
};

/// Refloat app-data package ID; upstream writes literal `101` in
/// `src/main.c:1271`, `1318`, `1881`, and `1909`.
pub const REFLOAT_APP_DATA_PACKAGE_ID: RefloatAppDataPackageId = RefloatAppDataPackageId::new(101);

/// Refloat app-data package identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RefloatAppDataPackageId(u8);

impl RefloatAppDataPackageId {
    /// Build a package ID token from the source-backed package ID.
    const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Explicitly extract the app-data package ID.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Refloat app-data command IDs.
///
/// Refloat `v1.2.1` defines the core IDs in `src/main.c:1241-1262`, LCM IDs in
/// `src/lcm.h:27-33`, and charging state in `src/charging.h:25`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatAppDataCommand {
    /// Version/package info.
    Info,
    /// Realtime data request.
    GetRealtimeData,
    /// Runtime tune without EEPROM write.
    RuntimeTune,
    /// Reset tune defaults without EEPROM write.
    TuneDefaults,
    /// Save config to EEPROM.
    ConfigSave,
    /// Restore config from EEPROM.
    ConfigRestore,
    /// Runtime startup/config change.
    TuneOther,
    /// Idle motor movement.
    RcMove,
    /// Booster settings.
    Booster,
    /// Print verbose info.
    PrintInfo,
    /// Compact all-data response request.
    GetAllData,
    /// Testing/tuning experiment command.
    Experiment,
    /// Lock/disable command.
    Lock,
    /// Hand-test mode command.
    HandTest,
    /// Tilt tuning command.
    TuneTilt,
    /// Lights-control command.
    LightsControl,
    /// Flywheel toggle command.
    Flywheel,
    /// LCM poll.
    LcmPoll,
    /// LCM light-info request.
    LcmLightInfo,
    /// LCM light-control command.
    LcmLightControl,
    /// LCM device-info request.
    LcmDeviceInfo,
    /// Charging-state command.
    ChargingState,
    /// LCM battery request.
    LcmGetBattery,
    /// Realtime data path.
    RealtimeData,
    /// Realtime data ID list.
    RealtimeDataIds,
    /// Alert list request.
    AlertsList,
    /// Alert control command.
    AlertsControl,
    /// Data recorder request.
    DataRecordRequest,
    /// LCM debug command reserved for external debugging.
    LcmDebug,
}

impl RefloatAppDataCommand {
    /// Parse a Refloat app-data command ID.
    pub const fn try_from_id(id: u8) -> Result<Self, RefloatAppDataCommandError> {
        match id {
            0 => Ok(Self::Info),
            1 => Ok(Self::GetRealtimeData),
            2 => Ok(Self::RuntimeTune),
            3 => Ok(Self::TuneDefaults),
            4 => Ok(Self::ConfigSave),
            5 => Ok(Self::ConfigRestore),
            6 => Ok(Self::TuneOther),
            7 => Ok(Self::RcMove),
            8 => Ok(Self::Booster),
            9 => Ok(Self::PrintInfo),
            10 => Ok(Self::GetAllData),
            11 => Ok(Self::Experiment),
            12 => Ok(Self::Lock),
            13 => Ok(Self::HandTest),
            14 => Ok(Self::TuneTilt),
            20 => Ok(Self::LightsControl),
            22 => Ok(Self::Flywheel),
            24 => Ok(Self::LcmPoll),
            25 => Ok(Self::LcmLightInfo),
            26 => Ok(Self::LcmLightControl),
            27 => Ok(Self::LcmDeviceInfo),
            28 => Ok(Self::ChargingState),
            29 => Ok(Self::LcmGetBattery),
            31 => Ok(Self::RealtimeData),
            32 => Ok(Self::RealtimeDataIds),
            35 => Ok(Self::AlertsList),
            36 => Ok(Self::AlertsControl),
            41 => Ok(Self::DataRecordRequest),
            99 => Ok(Self::LcmDebug),
            value => Err(RefloatAppDataCommandError { value }),
        }
    }

    /// Return the Refloat `v1.2.1` command ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::GetRealtimeData => 1,
            Self::RuntimeTune => 2,
            Self::TuneDefaults => 3,
            Self::ConfigSave => 4,
            Self::ConfigRestore => 5,
            Self::TuneOther => 6,
            Self::RcMove => 7,
            Self::Booster => 8,
            Self::PrintInfo => 9,
            Self::GetAllData => 10,
            Self::Experiment => 11,
            Self::Lock => 12,
            Self::HandTest => 13,
            Self::TuneTilt => 14,
            Self::LightsControl => 20,
            Self::Flywheel => 22,
            Self::LcmPoll => 24,
            Self::LcmLightInfo => 25,
            Self::LcmLightControl => 26,
            Self::LcmDeviceInfo => 27,
            Self::ChargingState => 28,
            Self::LcmGetBattery => 29,
            Self::RealtimeData => 31,
            Self::RealtimeDataIds => 32,
            Self::AlertsList => 35,
            Self::AlertsControl => 36,
            Self::DataRecordRequest => 41,
            Self::LcmDebug => 99,
        }
    }
}

/// Error returned when a Refloat app-data command ID is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatAppDataCommandError {
    value: u8,
}

impl RefloatAppDataCommandError {
    /// Return the rejected command ID.
    pub const fn value(self) -> u8 {
        self.value
    }
}

/// Refloat all-data request mode byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RefloatAllDataMode {
    source_id: u8,
}

impl RefloatAllDataMode {
    /// Build a mode token from the upstream Refloat request byte.
    pub const fn from_source_id(source_id: u8) -> Self {
        Self { source_id }
    }

    /// Build a base all-data request mode.
    pub const fn base() -> Self {
        Self::from_source_id(1)
    }

    /// Build a request mode that includes mode 2 fields.
    pub const fn with_mode2() -> Self {
        Self::from_source_id(2)
    }

    /// Build a request mode that includes mode 2 and 3 fields.
    pub const fn with_mode3() -> Self {
        Self::from_source_id(3)
    }

    /// Build a request mode that includes mode 2, 3, and 4 fields.
    pub const fn with_mode4() -> Self {
        Self::from_source_id(4)
    }

    /// Return the source request byte.
    pub const fn source_id(self) -> u8 {
        self.source_id
    }

    /// Return whether the mode includes mode 2 extension fields.
    pub const fn includes_mode2(self) -> bool {
        self.source_id >= 2
    }

    /// Return whether the mode includes mode 3 extension fields.
    pub const fn includes_mode3(self) -> bool {
        self.source_id >= 3
    }

    /// Return whether the mode includes mode 4 extension fields.
    pub const fn includes_mode4(self) -> bool {
        self.source_id >= 4
    }
}

/// Refloat all-data request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatAllDataRequest {
    mode: RefloatAllDataMode,
}

impl RefloatAllDataRequest {
    /// Build an all-data request.
    pub const fn new(mode: RefloatAllDataMode) -> Self {
        Self { mode }
    }

    /// Parse a Refloat `COMMAND_GET_ALLDATA` app-data packet.
    ///
    /// Upstream dispatches this command at `src/main.c:2210-2215` and encodes
    /// responses in `src/main.c:1313-1399`.
    pub fn parse(bytes: &[u8]) -> Result<Self, RefloatAllDataRequestError> {
        let [package_id, command_id, mode] = bytes else {
            return Err(RefloatAllDataRequestError::Length {
                actual: bytes.len(),
            });
        };

        if *package_id != REFLOAT_APP_DATA_PACKAGE_ID.get() {
            return Err(RefloatAllDataRequestError::PackageId { value: *package_id });
        }

        if *command_id != RefloatAppDataCommand::GetAllData.id() {
            return Err(RefloatAllDataRequestError::Command { value: *command_id });
        }

        Ok(Self::new(RefloatAllDataMode::from_source_id(*mode)))
    }

    /// Return the requested all-data mode.
    pub const fn mode(self) -> RefloatAllDataMode {
        self.mode
    }
}

/// Error returned when a Refloat all-data request cannot be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefloatAllDataRequestError {
    /// The request length is not the Refloat `v1.2.1` three-byte shape.
    Length {
        /// Actual request byte length.
        actual: usize,
    },
    /// The package ID does not match Refloat.
    PackageId {
        /// Rejected package ID.
        value: u8,
    },
    /// The command ID is not `COMMAND_GET_ALLDATA`.
    Command {
        /// Rejected command ID.
        value: u8,
    },
}

/// Refloat footpad sensor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FootpadSensorState {
    /// No footpad sensor is active.
    None,
    /// Left footpad sensor is active.
    Left,
    /// Right footpad sensor is active.
    Right,
    /// Both footpad sensors are active.
    Both,
}

impl FootpadSensorState {
    /// Return the Refloat `v1.2.1` footpad state ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Both => 3,
        }
    }

    /// Return the Refloat app-data switch compatibility value.
    pub const fn switch_compat(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left | Self::Right => 1,
            Self::Both => 2,
        }
    }
}

/// Refloat footpad ADC sample and decoded state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootpadSensorSample {
    adc1: AdcDecodedLevel,
    adc2: AdcDecodedLevel,
    state: FootpadSensorState,
}

impl FootpadSensorSample {
    /// Build a footpad sensor sample from typed ADC levels and decoded state.
    pub const fn new(
        adc1: AdcDecodedLevel,
        adc2: AdcDecodedLevel,
        state: FootpadSensorState,
    ) -> Self {
        Self { adc1, adc2, state }
    }

    /// Return the typed ADC1 level.
    pub const fn adc1(self) -> AdcDecodedLevel {
        self.adc1
    }

    /// Return the typed ADC2 level.
    pub const fn adc2(self) -> AdcDecodedLevel {
        self.adc2
    }

    /// Return the decoded footpad sensor state.
    pub const fn state(self) -> FootpadSensorState {
        self.state
    }
}

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

    const fn is_float_state_tiltback(self) -> bool {
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

    const fn extra_flags_compat(self, fatal_error: RefloatFatalErrorState) -> u8 {
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

/// Refloat hardware LED mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedMode {
    /// LEDs are disabled.
    Off,
    /// Internal/status LEDs are enabled.
    Internal,
    /// External LCM LEDs are enabled.
    External,
    /// Internal/status and external LCM LEDs are enabled.
    Both,
}

impl RefloatLedMode {
    /// Return the Refloat `v1.2.1` hardware LED mode ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Internal => 0x1,
            Self::External => 0x2,
            Self::Both => 0x3,
        }
    }

    const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}

/// Refloat hardware LED output pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedPin {
    /// STM32 pin B6.
    B6,
    /// STM32 pin B7.
    B7,
    /// STM32 pin C9.
    C9,
}

impl RefloatLedPin {
    /// Return the Refloat `v1.2.1` LED pin ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::B6 => 0,
            Self::B7 => 1,
            Self::C9 => 2,
        }
    }
}

/// Refloat hardware LED pin pull-up configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedPinConfig {
    /// Enable the 5V pull-up.
    PullupTo5v,
    /// Leave the LED pin without pull-up.
    NoPullup,
}

impl RefloatLedPinConfig {
    /// Return the Refloat `v1.2.1` LED pin config ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::PullupTo5v => 0,
            Self::NoPullup => 1,
        }
    }
}

/// Refloat LED color channel order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedColorOrder {
    /// Green, red, blue.
    Grb,
    /// Green, red, blue, white.
    Grbw,
    /// Red, green, blue.
    Rgb,
    /// White, red, green, blue.
    Wrgb,
}

impl RefloatLedColorOrder {
    /// Return the Refloat `v1.2.1` LED color order ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Grb => 0,
            Self::Grbw => 1,
            Self::Rgb => 2,
            Self::Wrgb => 3,
        }
    }
}

/// Refloat named LED color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedColor {
    /// Black/off.
    Black,
    /// White using all channels.
    WhiteFull,
    /// White using RGB channels.
    WhiteRgb,
    /// White using the white channel.
    WhiteSingle,
    /// Red.
    Red,
    /// Ferrari red.
    Ferrari,
    /// Flame.
    Flame,
    /// Coral.
    Coral,
    /// Sunset.
    Sunset,
    /// Sunrise.
    Sunrise,
    /// Gold.
    Gold,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Banana.
    Banana,
    /// Lime.
    Lime,
    /// Acid.
    Acid,
    /// Sage.
    Sage,
    /// Green.
    Green,
    /// Mint.
    Mint,
    /// Tiffany.
    Tiffany,
    /// Cyan.
    Cyan,
    /// Steel.
    Steel,
    /// Sky.
    Sky,
    /// Azure.
    Azure,
    /// Sapphire.
    Sapphire,
    /// Blue.
    Blue,
    /// Violet.
    Violet,
    /// Amethyst.
    Amethyst,
    /// Magenta.
    Magenta,
    /// Pink.
    Pink,
    /// Fuchsia.
    Fuchsia,
    /// Lavender.
    Lavender,
}

impl RefloatLedColor {
    /// Return the Refloat `v1.2.1` LED color ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Black => 0,
            Self::WhiteFull => 1,
            Self::WhiteRgb => 2,
            Self::WhiteSingle => 3,
            Self::Red => 4,
            Self::Ferrari => 5,
            Self::Flame => 6,
            Self::Coral => 7,
            Self::Sunset => 8,
            Self::Sunrise => 9,
            Self::Gold => 10,
            Self::Orange => 11,
            Self::Yellow => 12,
            Self::Banana => 13,
            Self::Lime => 14,
            Self::Acid => 15,
            Self::Sage => 16,
            Self::Green => 17,
            Self::Mint => 18,
            Self::Tiffany => 19,
            Self::Cyan => 20,
            Self::Steel => 21,
            Self::Sky => 22,
            Self::Azure => 23,
            Self::Sapphire => 24,
            Self::Blue => 25,
            Self::Violet => 26,
            Self::Amethyst => 27,
            Self::Magenta => 28,
            Self::Pink => 29,
            Self::Fuchsia => 30,
            Self::Lavender => 31,
        }
    }
}

/// Refloat LED animation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedAnimationMode {
    /// Solid color.
    Solid,
    /// Fade between colors.
    Fade,
    /// Pulse between colors.
    Pulse,
    /// Strobe between colors.
    Strobe,
    /// Knight-rider sweep.
    KnightRider,
    /// Alternating red/blue style animation.
    Felony,
    /// Cycle rainbow colors.
    RainbowCycle,
    /// Fade rainbow colors.
    RainbowFade,
    /// Roll rainbow colors.
    RainbowRoll,
}

impl RefloatLedAnimationMode {
    /// Return the Refloat `v1.2.1` LED animation mode ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Solid => 0,
            Self::Fade => 1,
            Self::Pulse => 2,
            Self::Strobe => 3,
            Self::KnightRider => 4,
            Self::Felony => 5,
            Self::RainbowCycle => 6,
            Self::RainbowFade => 7,
            Self::RainbowRoll => 8,
        }
    }
}

/// Refloat LED transition mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedTransition {
    /// Fade directly to the target bar.
    Fade,
    /// Fade out, then fade in.
    FadeOutIn,
    /// Cipher transition.
    Cipher,
    /// Monochrome cipher transition.
    MonoCipher,
}

impl RefloatLedTransition {
    /// Return the Refloat `v1.2.1` LED transition ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Fade => 0,
            Self::FadeOutIn => 1,
            Self::Cipher => 2,
            Self::MonoCipher => 3,
        }
    }
}

/// Refloat LED animation speed scalar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatLedAnimationSpeed(f32);

impl RefloatLedAnimationSpeed {
    /// Wrap a Refloat LED animation speed value.
    pub const fn from_units(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat LED animation speed value.
    pub const fn as_units(self) -> f32 {
        self.0
    }
}

/// Refloat LED bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatLedBarConfig {
    brightness: Ratio,
    primary_color: RefloatLedColor,
    secondary_color: RefloatLedColor,
    animation_mode: RefloatLedAnimationMode,
    animation_speed: RefloatLedAnimationSpeed,
}

impl RefloatLedBarConfig {
    /// Build a typed Refloat LED bar config.
    pub const fn new(
        brightness: Ratio,
        primary_color: RefloatLedColor,
        secondary_color: RefloatLedColor,
        animation_mode: RefloatLedAnimationMode,
        animation_speed: RefloatLedAnimationSpeed,
    ) -> Self {
        Self {
            brightness,
            primary_color,
            secondary_color,
            animation_mode,
            animation_speed,
        }
    }

    /// Return the configured brightness.
    pub const fn brightness(self) -> Ratio {
        self.brightness
    }

    /// Return the primary LED color.
    pub const fn primary_color(self) -> RefloatLedColor {
        self.primary_color
    }

    /// Return the secondary LED color.
    pub const fn secondary_color(self) -> RefloatLedColor {
        self.secondary_color
    }

    /// Return the animation mode.
    pub const fn animation_mode(self) -> RefloatLedAnimationMode {
        self.animation_mode
    }

    /// Return the animation speed.
    pub const fn animation_speed(self) -> RefloatLedAnimationSpeed {
        self.animation_speed
    }
}

/// Refloat status-bar idle timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RefloatStatusBarIdleTimeout(u16);

impl RefloatStatusBarIdleTimeout {
    /// Wrap a Refloat status-bar idle timeout in seconds.
    pub const fn from_seconds(value: u16) -> Self {
        Self(value)
    }

    /// Return the idle timeout in seconds.
    pub const fn as_seconds(self) -> u16 {
        self.0
    }
}

/// Refloat status-bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatStatusBarConfig {
    idle_timeout: RefloatStatusBarIdleTimeout,
    duty_threshold: Ratio,
    red_bar_percentage: Ratio,
    show_sensors_while_running: bool,
    brightness_headlights_on: Ratio,
    brightness_headlights_off: Ratio,
}

impl RefloatStatusBarConfig {
    /// Build a typed Refloat status-bar config.
    pub const fn new(
        idle_timeout: RefloatStatusBarIdleTimeout,
        duty_threshold: Ratio,
        red_bar_percentage: Ratio,
        brightness_headlights_on: Ratio,
        brightness_headlights_off: Ratio,
    ) -> Self {
        Self {
            idle_timeout,
            duty_threshold,
            red_bar_percentage,
            show_sensors_while_running: false,
            brightness_headlights_on,
            brightness_headlights_off,
        }
    }

    /// Return this config with sensor display enabled while running.
    pub const fn showing_sensors_while_running(mut self) -> Self {
        self.show_sensors_while_running = true;
        self
    }

    /// Return the idle timeout.
    pub const fn idle_timeout(self) -> RefloatStatusBarIdleTimeout {
        self.idle_timeout
    }

    /// Return the duty threshold for switching status display.
    pub const fn duty_threshold(self) -> Ratio {
        self.duty_threshold
    }

    /// Return the red-bar percentage threshold.
    pub const fn red_bar_percentage(self) -> Ratio {
        self.red_bar_percentage
    }

    /// Return whether sensors are shown while running.
    pub const fn shows_sensors_while_running(self) -> bool {
        self.show_sensors_while_running
    }

    /// Return status brightness when headlights are on.
    pub const fn brightness_headlights_on(self) -> Ratio {
        self.brightness_headlights_on
    }

    /// Return status brightness when headlights are off.
    pub const fn brightness_headlights_off(self) -> Ratio {
        self.brightness_headlights_off
    }
}

/// Refloat LEDs configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatLedsConfig {
    on: bool,
    headlights_on: bool,
    headlights_transition: RefloatLedTransition,
    direction_transition: RefloatLedTransition,
    lights_off_when_lifted: bool,
    status_on_front_when_lifted: bool,
    headlights: RefloatLedBarConfig,
    taillights: RefloatLedBarConfig,
    front: RefloatLedBarConfig,
    rear: RefloatLedBarConfig,
    status: RefloatStatusBarConfig,
    status_idle: RefloatLedBarConfig,
}

impl RefloatLedsConfig {
    /// Build a typed Refloat LEDs config.
    pub const fn new(
        headlights: RefloatLedBarConfig,
        taillights: RefloatLedBarConfig,
        front: RefloatLedBarConfig,
        rear: RefloatLedBarConfig,
        status: RefloatStatusBarConfig,
        status_idle: RefloatLedBarConfig,
    ) -> Self {
        Self {
            on: false,
            headlights_on: false,
            headlights_transition: RefloatLedTransition::Fade,
            direction_transition: RefloatLedTransition::Fade,
            lights_off_when_lifted: false,
            status_on_front_when_lifted: false,
            headlights,
            taillights,
            front,
            rear,
            status,
            status_idle,
        }
    }

    /// Return this config with LEDs enabled.
    pub const fn enabled(mut self) -> Self {
        self.on = true;
        self
    }

    /// Return this config with headlights enabled.
    pub const fn with_headlights_on(mut self) -> Self {
        self.headlights_on = true;
        self
    }

    /// Return this config with the headlights transition set.
    pub const fn with_headlights_transition(mut self, transition: RefloatLedTransition) -> Self {
        self.headlights_transition = transition;
        self
    }

    /// Return this config with the direction transition set.
    pub const fn with_direction_transition(mut self, transition: RefloatLedTransition) -> Self {
        self.direction_transition = transition;
        self
    }

    /// Return this config with lights off while lifted.
    pub const fn lights_off_when_lifted(mut self) -> Self {
        self.lights_off_when_lifted = true;
        self
    }

    /// Return this config with status shown on the front while lifted.
    pub const fn status_on_front_when_lifted(mut self) -> Self {
        self.status_on_front_when_lifted = true;
        self
    }

    /// Return whether LEDs are enabled.
    pub const fn is_enabled(self) -> bool {
        self.on
    }

    /// Return whether headlights are on.
    pub const fn are_headlights_on(self) -> bool {
        self.headlights_on
    }

    /// Return the headlights transition.
    pub const fn headlights_transition(self) -> RefloatLedTransition {
        self.headlights_transition
    }

    /// Return the direction transition.
    pub const fn direction_transition(self) -> RefloatLedTransition {
        self.direction_transition
    }

    /// Return whether lights are turned off while lifted.
    pub const fn turns_lights_off_when_lifted(self) -> bool {
        self.lights_off_when_lifted
    }

    /// Return whether status is shown on the front while lifted.
    pub const fn shows_status_on_front_when_lifted(self) -> bool {
        self.status_on_front_when_lifted
    }

    /// Return the headlights LED bar config.
    pub const fn headlights(self) -> RefloatLedBarConfig {
        self.headlights
    }

    /// Return the taillights LED bar config.
    pub const fn taillights(self) -> RefloatLedBarConfig {
        self.taillights
    }

    /// Return the front LED bar config.
    pub const fn front(self) -> RefloatLedBarConfig {
        self.front
    }

    /// Return the rear LED bar config.
    pub const fn rear(self) -> RefloatLedBarConfig {
        self.rear
    }

    /// Return the status-bar config.
    pub const fn status(self) -> RefloatStatusBarConfig {
        self.status
    }

    /// Return the idle status LED bar config.
    pub const fn status_idle(self) -> RefloatLedBarConfig {
        self.status_idle
    }
}

/// Refloat physical LED strip order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedStripOrder {
    /// No strip is assigned.
    None,
    /// First LED strip.
    First,
    /// Second LED strip.
    Second,
    /// Third LED strip.
    Third,
}

impl RefloatLedStripOrder {
    /// Return the Refloat `v1.2.1` LED strip order ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::First => 1,
            Self::Second => 2,
            Self::Third => 3,
        }
    }
}

/// Refloat LED strip configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatLedStripConfig {
    order: RefloatLedStripOrder,
    count: u8,
    color_order: RefloatLedColorOrder,
    reverse: bool,
}

impl RefloatLedStripConfig {
    /// Build a typed Refloat LED strip config.
    pub const fn new(
        order: RefloatLedStripOrder,
        count: u8,
        color_order: RefloatLedColorOrder,
    ) -> Self {
        Self {
            order,
            count,
            color_order,
            reverse: false,
        }
    }

    /// Return this config with reverse ordering enabled or disabled.
    pub const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Return the physical strip order.
    pub const fn order(self) -> RefloatLedStripOrder {
        self.order
    }

    /// Return the configured LED count.
    pub const fn count(self) -> u8 {
        self.count
    }

    /// Return the configured color channel order.
    pub const fn color_order(self) -> RefloatLedColorOrder {
        self.color_order
    }

    /// Return whether LED indexing is reversed.
    pub const fn is_reversed(self) -> bool {
        self.reverse
    }
}

/// Refloat hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatHardwareLedsConfig {
    mode: RefloatLedMode,
    pin: RefloatLedPin,
    pin_config: RefloatLedPinConfig,
    status: RefloatLedStripConfig,
    front: RefloatLedStripConfig,
    rear: RefloatLedStripConfig,
}

impl RefloatHardwareLedsConfig {
    /// Build the hardware LED config from typed Refloat LED mode.
    pub const fn new(mode: RefloatLedMode) -> Self {
        Self {
            mode,
            pin: RefloatLedPin::B7,
            pin_config: RefloatLedPinConfig::PullupTo5v,
            status: RefloatLedStripConfig::new(
                RefloatLedStripOrder::First,
                10,
                RefloatLedColorOrder::Grb,
            ),
            front: RefloatLedStripConfig::new(
                RefloatLedStripOrder::Second,
                20,
                RefloatLedColorOrder::Grb,
            ),
            rear: RefloatLedStripConfig::new(
                RefloatLedStripOrder::Third,
                20,
                RefloatLedColorOrder::Grb,
            ),
        }
    }

    /// Return this config with the LED output pin set.
    pub const fn with_pin(mut self, pin: RefloatLedPin) -> Self {
        self.pin = pin;
        self
    }

    /// Return this config with the LED pin configuration set.
    pub const fn with_pin_config(mut self, pin_config: RefloatLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    /// Return this config with the status strip set.
    pub const fn with_status_strip(mut self, status: RefloatLedStripConfig) -> Self {
        self.status = status;
        self
    }

    /// Return this config with the front strip set.
    pub const fn with_front_strip(mut self, front: RefloatLedStripConfig) -> Self {
        self.front = front;
        self
    }

    /// Return this config with the rear strip set.
    pub const fn with_rear_strip(mut self, rear: RefloatLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    /// Return the configured LED mode.
    pub const fn mode(self) -> RefloatLedMode {
        self.mode
    }

    /// Return the configured LED output pin.
    pub const fn pin(self) -> RefloatLedPin {
        self.pin
    }

    /// Return the configured LED pin mode.
    pub const fn pin_config(self) -> RefloatLedPinConfig {
        self.pin_config
    }

    /// Return the configured status LED strip.
    pub const fn status_strip(self) -> RefloatLedStripConfig {
        self.status
    }

    /// Return the configured front LED strip.
    pub const fn front_strip(self) -> RefloatLedStripConfig {
        self.front
    }

    /// Return the configured rear LED strip.
    pub const fn rear_strip(self) -> RefloatLedStripConfig {
        self.rear
    }

    /// Return whether internal/status LEDs are enabled.
    pub const fn uses_internal_leds(self) -> bool {
        self.mode.uses_internal_leds()
    }

    /// Return whether external LCM LEDs are enabled.
    pub const fn uses_external_leds(self) -> bool {
        self.mode.uses_external_leds()
    }
}

/// Refloat hardware configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatHardwareConfig {
    leds: RefloatHardwareLedsConfig,
}

impl RefloatHardwareConfig {
    /// Build a typed Refloat hardware config.
    pub const fn new(leds: RefloatHardwareLedsConfig) -> Self {
        Self { leds }
    }

    /// Return the hardware LED configuration.
    pub const fn leds(self) -> RefloatHardwareLedsConfig {
        self.leds
    }
}

/// Refloat realtime-data items that are always sent.
///
/// The ID-list packet format is described in upstream `src/main.c:1884-1898`.
pub const REFLOAT_REALTIME_DATA_ITEMS: [RefloatRealtimeDataItem; 16] = [
    RefloatRealtimeDataItem::MotorSpeed,
    RefloatRealtimeDataItem::MotorErpm,
    RefloatRealtimeDataItem::MotorCurrent,
    RefloatRealtimeDataItem::MotorDirectionalCurrent,
    RefloatRealtimeDataItem::MotorFilteredCurrent,
    RefloatRealtimeDataItem::MotorDutyCycle,
    RefloatRealtimeDataItem::MotorBatteryVoltage,
    RefloatRealtimeDataItem::MotorBatteryCurrent,
    RefloatRealtimeDataItem::MotorMosfetTemperature,
    RefloatRealtimeDataItem::MotorTemperature,
    RefloatRealtimeDataItem::ImuPitch,
    RefloatRealtimeDataItem::ImuBalancePitch,
    RefloatRealtimeDataItem::ImuRoll,
    RefloatRealtimeDataItem::FootpadAdc1,
    RefloatRealtimeDataItem::FootpadAdc2,
    RefloatRealtimeDataItem::RemoteInput,
];

/// Refloat realtime-data items sent only while running.
///
/// Upstream appends this second ID set after the always-sent set in
/// `src/main.c:1892-1898`.
pub const REFLOAT_REALTIME_RUNTIME_ITEMS: [RefloatRealtimeDataItem; 10] = [
    RefloatRealtimeDataItem::Setpoint,
    RefloatRealtimeDataItem::AtrSetpoint,
    RefloatRealtimeDataItem::BrakeTiltSetpoint,
    RefloatRealtimeDataItem::TorqueTiltSetpoint,
    RefloatRealtimeDataItem::TurnTiltSetpoint,
    RefloatRealtimeDataItem::RemoteSetpoint,
    RefloatRealtimeDataItem::BalanceCurrent,
    RefloatRealtimeDataItem::AtrAccelDiff,
    RefloatRealtimeDataItem::AtrSpeedBoost,
    RefloatRealtimeDataItem::BoosterCurrent,
];

/// Refloat realtime-data items recorded by the data recorder.
///
/// This list mirrors the port's current data-recorder model; re-check against
/// upstream `src/data_recorder.c` before treating it as hardware parity.
pub const REFLOAT_REALTIME_RECORDED_ITEMS: [RefloatRealtimeDataItem; 10] = [
    RefloatRealtimeDataItem::MotorErpm,
    RefloatRealtimeDataItem::MotorDirectionalCurrent,
    RefloatRealtimeDataItem::MotorDutyCycle,
    RefloatRealtimeDataItem::MotorBatteryVoltage,
    RefloatRealtimeDataItem::ImuPitch,
    RefloatRealtimeDataItem::ImuBalancePitch,
    RefloatRealtimeDataItem::Setpoint,
    RefloatRealtimeDataItem::AtrSetpoint,
    RefloatRealtimeDataItem::TorqueTiltSetpoint,
    RefloatRealtimeDataItem::BalanceCurrent,
];

/// Refloat realtime-data item group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataItemGroup {
    /// Always sent in realtime data.
    Always,
    /// Sent only while the board is running.
    Runtime,
}

/// Refloat data-recorder policy for a realtime-data item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataRecordPolicy {
    /// Send in realtime data only.
    SendOnly,
    /// Send in realtime data and record in the data recorder.
    Record,
}

/// Refloat realtime-data item ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataItem {
    /// `motor.speed`.
    MotorSpeed,
    /// `motor.erpm`.
    MotorErpm,
    /// `motor.current`.
    MotorCurrent,
    /// `motor.dir_current`.
    MotorDirectionalCurrent,
    /// `motor.filt_current`.
    MotorFilteredCurrent,
    /// `motor.duty_cycle`.
    MotorDutyCycle,
    /// `motor.batt_voltage`.
    MotorBatteryVoltage,
    /// `motor.batt_current`.
    MotorBatteryCurrent,
    /// `motor.mosfet_temp`.
    MotorMosfetTemperature,
    /// `motor.motor_temp`.
    MotorTemperature,
    /// `imu.pitch`.
    ImuPitch,
    /// `imu.balance_pitch`.
    ImuBalancePitch,
    /// `imu.roll`.
    ImuRoll,
    /// `footpad.adc1`.
    FootpadAdc1,
    /// `footpad.adc2`.
    FootpadAdc2,
    /// `remote.input`.
    RemoteInput,
    /// `setpoint`.
    Setpoint,
    /// `atr.setpoint`.
    AtrSetpoint,
    /// `brake_tilt.setpoint`.
    BrakeTiltSetpoint,
    /// `torque_tilt.setpoint`.
    TorqueTiltSetpoint,
    /// `turn_tilt.setpoint`.
    TurnTiltSetpoint,
    /// `remote.setpoint`.
    RemoteSetpoint,
    /// `balance_current`.
    BalanceCurrent,
    /// `atr.accel_diff`.
    AtrAccelDiff,
    /// `atr.speed_boost`.
    AtrSpeedBoost,
    /// `booster.current`.
    BoosterCurrent,
}

impl RefloatRealtimeDataItem {
    /// Return the Refloat `v1.2.1` realtime-data string ID.
    pub const fn id(self) -> &'static str {
        match self {
            Self::MotorSpeed => "motor.speed",
            Self::MotorErpm => "motor.erpm",
            Self::MotorCurrent => "motor.current",
            Self::MotorDirectionalCurrent => "motor.dir_current",
            Self::MotorFilteredCurrent => "motor.filt_current",
            Self::MotorDutyCycle => "motor.duty_cycle",
            Self::MotorBatteryVoltage => "motor.batt_voltage",
            Self::MotorBatteryCurrent => "motor.batt_current",
            Self::MotorMosfetTemperature => "motor.mosfet_temp",
            Self::MotorTemperature => "motor.motor_temp",
            Self::ImuPitch => "imu.pitch",
            Self::ImuBalancePitch => "imu.balance_pitch",
            Self::ImuRoll => "imu.roll",
            Self::FootpadAdc1 => "footpad.adc1",
            Self::FootpadAdc2 => "footpad.adc2",
            Self::RemoteInput => "remote.input",
            Self::Setpoint => "setpoint",
            Self::AtrSetpoint => "atr.setpoint",
            Self::BrakeTiltSetpoint => "brake_tilt.setpoint",
            Self::TorqueTiltSetpoint => "torque_tilt.setpoint",
            Self::TurnTiltSetpoint => "turn_tilt.setpoint",
            Self::RemoteSetpoint => "remote.setpoint",
            Self::BalanceCurrent => "balance_current",
            Self::AtrAccelDiff => "atr.accel_diff",
            Self::AtrSpeedBoost => "atr.speed_boost",
            Self::BoosterCurrent => "booster.current",
        }
    }

    /// Return the Refloat `v1.2.1` realtime-data group.
    pub const fn group(self) -> RefloatRealtimeDataItemGroup {
        match self {
            Self::Setpoint
            | Self::AtrSetpoint
            | Self::BrakeTiltSetpoint
            | Self::TorqueTiltSetpoint
            | Self::TurnTiltSetpoint
            | Self::RemoteSetpoint
            | Self::BalanceCurrent
            | Self::AtrAccelDiff
            | Self::AtrSpeedBoost
            | Self::BoosterCurrent => RefloatRealtimeDataItemGroup::Runtime,
            Self::MotorSpeed
            | Self::MotorErpm
            | Self::MotorCurrent
            | Self::MotorDirectionalCurrent
            | Self::MotorFilteredCurrent
            | Self::MotorDutyCycle
            | Self::MotorBatteryVoltage
            | Self::MotorBatteryCurrent
            | Self::MotorMosfetTemperature
            | Self::MotorTemperature
            | Self::ImuPitch
            | Self::ImuBalancePitch
            | Self::ImuRoll
            | Self::FootpadAdc1
            | Self::FootpadAdc2
            | Self::RemoteInput => RefloatRealtimeDataItemGroup::Always,
        }
    }

    /// Return the Refloat `v1.2.1` data-recorder policy.
    pub const fn record_policy(self) -> RefloatRealtimeDataRecordPolicy {
        match self {
            Self::MotorErpm
            | Self::MotorDirectionalCurrent
            | Self::MotorDutyCycle
            | Self::MotorBatteryVoltage
            | Self::ImuPitch
            | Self::ImuBalancePitch
            | Self::Setpoint
            | Self::AtrSetpoint
            | Self::TorqueTiltSetpoint
            | Self::BalanceCurrent => RefloatRealtimeDataRecordPolicy::Record,
            Self::MotorSpeed
            | Self::MotorCurrent
            | Self::MotorFilteredCurrent
            | Self::MotorBatteryCurrent
            | Self::MotorMosfetTemperature
            | Self::MotorTemperature
            | Self::ImuRoll
            | Self::FootpadAdc1
            | Self::FootpadAdc2
            | Self::RemoteInput
            | Self::BrakeTiltSetpoint
            | Self::TurnTiltSetpoint
            | Self::RemoteSetpoint
            | Self::AtrAccelDiff
            | Self::AtrSpeedBoost
            | Self::BoosterCurrent => RefloatRealtimeDataRecordPolicy::SendOnly,
        }
    }
}

/// Refloat `motor.filt_current` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeFilteredMotorCurrent(DirectionalMotorCurrent);

impl RefloatRealtimeFilteredMotorCurrent {
    /// Build a typed Refloat filtered-current value.
    pub const fn new(current: DirectionalMotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed filtered current without erasing it to a primitive.
    pub const fn current(self) -> DirectionalMotorCurrent {
        self.0
    }
}

/// Refloat `imu.balance_pitch` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBalancePitch(AngleRadians);

impl RefloatRealtimeBalancePitch {
    /// Build a typed Refloat balance-pitch value.
    pub const fn new(angle: AngleRadians) -> Self {
        Self(angle)
    }

    /// Return the typed balance-pitch angle without erasing it to a primitive.
    pub const fn angle(self) -> AngleRadians {
        self.0
    }
}

/// Refloat `remote.input` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeRemoteInput(SignedRatio);

impl RefloatRealtimeRemoteInput {
    /// Build a typed Refloat remote-input value.
    pub const fn new(ratio: SignedRatio) -> Self {
        Self(ratio)
    }

    /// Return the typed remote input without erasing it to a primitive.
    pub const fn ratio(self) -> SignedRatio {
        self.0
    }
}

/// Refloat realtime motor-current values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorCurrents {
    motor: MotorCurrent,
    directional: DirectionalMotorCurrent,
    filtered: RefloatRealtimeFilteredMotorCurrent,
    battery: BatteryCurrent,
}

impl RefloatRealtimeMotorCurrents {
    /// Build typed Refloat realtime current values.
    pub const fn new(
        motor: MotorCurrent,
        directional: DirectionalMotorCurrent,
        filtered: RefloatRealtimeFilteredMotorCurrent,
        battery: BatteryCurrent,
    ) -> Self {
        Self {
            motor,
            directional,
            filtered,
            battery,
        }
    }

    /// Return `motor.current`.
    pub const fn motor(self) -> MotorCurrent {
        self.motor
    }

    /// Return `motor.dir_current`.
    pub const fn directional(self) -> DirectionalMotorCurrent {
        self.directional
    }

    /// Return `motor.filt_current`.
    pub const fn filtered(self) -> RefloatRealtimeFilteredMotorCurrent {
        self.filtered
    }

    /// Return `motor.batt_current`.
    pub const fn battery(self) -> BatteryCurrent {
        self.battery
    }
}

/// Refloat realtime motor-temperature values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorTemperatures {
    mosfet: MosfetTemperature,
    motor: MotorTemperature,
}

impl RefloatRealtimeMotorTemperatures {
    /// Build typed Refloat realtime motor-temperature values.
    pub const fn new(mosfet: MosfetTemperature, motor: MotorTemperature) -> Self {
        Self { mosfet, motor }
    }

    /// Return `motor.mosfet_temp`.
    pub const fn mosfet(self) -> MosfetTemperature {
        self.mosfet
    }

    /// Return `motor.motor_temp`.
    pub const fn motor(self) -> MotorTemperature {
        self.motor
    }
}

/// Refloat realtime motor payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorPayload {
    speed: VehicleSpeed,
    electrical_speed: ElectricalSpeed,
    currents: RefloatRealtimeMotorCurrents,
    duty_cycle: DutyCycle,
    battery_voltage: BatteryVoltage,
    temperatures: RefloatRealtimeMotorTemperatures,
}

impl RefloatRealtimeMotorPayload {
    /// Build typed Refloat realtime motor values.
    pub const fn new(
        speed: VehicleSpeed,
        electrical_speed: ElectricalSpeed,
        currents: RefloatRealtimeMotorCurrents,
        duty_cycle: DutyCycle,
        battery_voltage: BatteryVoltage,
        temperatures: RefloatRealtimeMotorTemperatures,
    ) -> Self {
        Self {
            speed,
            electrical_speed,
            currents,
            duty_cycle,
            battery_voltage,
            temperatures,
        }
    }

    /// Return `motor.speed`.
    pub const fn speed(self) -> VehicleSpeed {
        self.speed
    }

    /// Return `motor.erpm`.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return grouped motor-current values.
    pub const fn currents(self) -> RefloatRealtimeMotorCurrents {
        self.currents
    }

    /// Return `motor.duty_cycle`.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return `motor.batt_voltage`.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }

    /// Return grouped motor-temperature values.
    pub const fn temperatures(self) -> RefloatRealtimeMotorTemperatures {
        self.temperatures
    }
}

/// Refloat realtime IMU payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeImuPayload {
    pitch: ImuPitch,
    balance_pitch: RefloatRealtimeBalancePitch,
    roll: ImuRoll,
}

impl RefloatRealtimeImuPayload {
    /// Build typed Refloat realtime IMU values.
    pub const fn new(
        pitch: ImuPitch,
        balance_pitch: RefloatRealtimeBalancePitch,
        roll: ImuRoll,
    ) -> Self {
        Self {
            pitch,
            balance_pitch,
            roll,
        }
    }

    /// Return `imu.pitch`.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }

    /// Return `imu.balance_pitch`.
    pub const fn balance_pitch(self) -> RefloatRealtimeBalancePitch {
        self.balance_pitch
    }

    /// Return `imu.roll`.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }
}

/// Refloat realtime payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeAlwaysPayload {
    motor: RefloatRealtimeMotorPayload,
    imu: RefloatRealtimeImuPayload,
    footpad: FootpadSensorSample,
    remote_input: RefloatRealtimeRemoteInput,
}

impl RefloatRealtimeAlwaysPayload {
    /// Build typed Refloat realtime values that are always sent.
    pub const fn new(
        motor: RefloatRealtimeMotorPayload,
        imu: RefloatRealtimeImuPayload,
        footpad: FootpadSensorSample,
        remote_input: RefloatRealtimeRemoteInput,
    ) -> Self {
        Self {
            motor,
            imu,
            footpad,
            remote_input,
        }
    }

    /// Return the source-backed item contract for this payload section.
    pub const fn item_contract(self) -> [RefloatRealtimeDataItem; 16] {
        REFLOAT_REALTIME_DATA_ITEMS
    }

    /// Return grouped motor values.
    pub const fn motor(self) -> RefloatRealtimeMotorPayload {
        self.motor
    }

    /// Return grouped IMU values.
    pub const fn imu(self) -> RefloatRealtimeImuPayload {
        self.imu
    }

    /// Return grouped footpad values.
    pub const fn footpad(self) -> FootpadSensorSample {
        self.footpad
    }

    /// Return `remote.input`.
    pub const fn remote_input(self) -> RefloatRealtimeRemoteInput {
        self.remote_input
    }
}

/// Refloat runtime setpoint angle value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeRuntimeSetpoint(AngleDegrees);

impl RefloatRealtimeRuntimeSetpoint {
    /// Build a typed Refloat runtime setpoint value.
    pub const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    /// Return the typed setpoint angle without erasing it to a primitive.
    pub const fn angle(self) -> AngleDegrees {
        self.0
    }
}

/// Refloat runtime setpoint values sent only while running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimeSetpoints {
    board: RefloatRealtimeRuntimeSetpoint,
    atr: RefloatRealtimeRuntimeSetpoint,
    brake_tilt: RefloatRealtimeRuntimeSetpoint,
    torque_tilt: RefloatRealtimeRuntimeSetpoint,
    turn_tilt: RefloatRealtimeRuntimeSetpoint,
    remote: RefloatRealtimeRuntimeSetpoint,
}

impl RefloatRealtimeRuntimeSetpoints {
    /// Build typed Refloat runtime setpoint values.
    pub const fn new(
        board: RefloatRealtimeRuntimeSetpoint,
        atr: RefloatRealtimeRuntimeSetpoint,
        brake_tilt: RefloatRealtimeRuntimeSetpoint,
        torque_tilt: RefloatRealtimeRuntimeSetpoint,
        turn_tilt: RefloatRealtimeRuntimeSetpoint,
        remote: RefloatRealtimeRuntimeSetpoint,
    ) -> Self {
        Self {
            board,
            atr,
            brake_tilt,
            torque_tilt,
            turn_tilt,
            remote,
        }
    }

    /// Return `setpoint`.
    pub const fn board(self) -> RefloatRealtimeRuntimeSetpoint {
        self.board
    }

    /// Return `atr.setpoint`.
    pub const fn atr(self) -> RefloatRealtimeRuntimeSetpoint {
        self.atr
    }

    /// Return `brake_tilt.setpoint`.
    pub const fn brake_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.brake_tilt
    }

    /// Return `torque_tilt.setpoint`.
    pub const fn torque_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.torque_tilt
    }

    /// Return `turn_tilt.setpoint`.
    pub const fn turn_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.turn_tilt
    }

    /// Return `remote.setpoint`.
    pub const fn remote(self) -> RefloatRealtimeRuntimeSetpoint {
        self.remote
    }
}

/// Refloat `balance_current` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBalanceCurrent(MotorCurrent);

impl RefloatRealtimeBalanceCurrent {
    /// Build a typed Refloat balance-current value.
    pub const fn new(current: MotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed balance current without erasing it to a primitive.
    pub const fn current(self) -> MotorCurrent {
        self.0
    }
}

/// Refloat `atr.accel_diff` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatRealtimeAtrAccelerationDiff(f32);

impl RefloatRealtimeAtrAccelerationDiff {
    /// Build a typed Refloat ATR acceleration-difference value from ERPM delta units.
    pub const fn from_erpm_delta(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat ATR acceleration-difference value in ERPM delta units.
    pub const fn as_erpm_delta(self) -> f32 {
        self.0
    }
}

/// Refloat `atr.speed_boost` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatRealtimeAtrSpeedBoost(f32);

impl RefloatRealtimeAtrSpeedBoost {
    /// Build a typed Refloat ATR speed-boost value.
    pub const fn from_units(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat ATR speed-boost value.
    pub const fn as_units(self) -> f32 {
        self.0
    }
}

/// Refloat runtime ATR payload values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimeAtrPayload {
    accel_diff: RefloatRealtimeAtrAccelerationDiff,
    speed_boost: RefloatRealtimeAtrSpeedBoost,
}

impl RefloatRealtimeRuntimeAtrPayload {
    /// Build typed Refloat runtime ATR payload values.
    pub const fn new(
        accel_diff: RefloatRealtimeAtrAccelerationDiff,
        speed_boost: RefloatRealtimeAtrSpeedBoost,
    ) -> Self {
        Self {
            accel_diff,
            speed_boost,
        }
    }

    /// Return `atr.accel_diff`.
    pub const fn accel_diff(self) -> RefloatRealtimeAtrAccelerationDiff {
        self.accel_diff
    }

    /// Return `atr.speed_boost`.
    pub const fn speed_boost(self) -> RefloatRealtimeAtrSpeedBoost {
        self.speed_boost
    }
}

/// Refloat `booster.current` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBoosterCurrent(MotorCurrent);

impl RefloatRealtimeBoosterCurrent {
    /// Build a typed Refloat booster-current value.
    pub const fn new(current: MotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed booster current without erasing it to a primitive.
    pub const fn current(self) -> MotorCurrent {
        self.0
    }
}

/// Refloat realtime payload values sent only while running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimePayload {
    setpoints: RefloatRealtimeRuntimeSetpoints,
    balance_current: RefloatRealtimeBalanceCurrent,
    atr: RefloatRealtimeRuntimeAtrPayload,
    booster_current: RefloatRealtimeBoosterCurrent,
}

impl RefloatRealtimeRuntimePayload {
    /// Build typed Refloat realtime values sent only while running.
    pub const fn new(
        setpoints: RefloatRealtimeRuntimeSetpoints,
        balance_current: RefloatRealtimeBalanceCurrent,
        atr: RefloatRealtimeRuntimeAtrPayload,
        booster_current: RefloatRealtimeBoosterCurrent,
    ) -> Self {
        Self {
            setpoints,
            balance_current,
            atr,
            booster_current,
        }
    }

    /// Return the source-backed item contract for this payload section.
    pub const fn item_contract(self) -> [RefloatRealtimeDataItem; 10] {
        REFLOAT_REALTIME_RUNTIME_ITEMS
    }

    /// Return grouped runtime setpoint values.
    pub const fn setpoints(self) -> RefloatRealtimeRuntimeSetpoints {
        self.setpoints
    }

    /// Return `balance_current`.
    pub const fn balance_current(self) -> RefloatRealtimeBalanceCurrent {
        self.balance_current
    }

    /// Return grouped ATR runtime values.
    pub const fn atr(self) -> RefloatRealtimeRuntimeAtrPayload {
        self.atr
    }

    /// Return `booster.current`.
    pub const fn booster_current(self) -> RefloatRealtimeBoosterCurrent {
        self.booster_current
    }
}

/// Refloat `charging.current` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeChargingCurrent(BatteryCurrent);

impl RefloatRealtimeChargingCurrent {
    /// Build a typed Refloat charging-current value.
    pub const fn new(current: BatteryCurrent) -> Self {
        Self(current)
    }

    /// Return the typed charging current without erasing it to a primitive.
    pub const fn current(self) -> BatteryCurrent {
        self.0
    }
}

/// Refloat `charging.voltage` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeChargingVoltage(BatteryVoltage);

impl RefloatRealtimeChargingVoltage {
    /// Build a typed Refloat charging-voltage value.
    pub const fn new(voltage: BatteryVoltage) -> Self {
        Self(voltage)
    }

    /// Return the typed charging voltage without erasing it to a primitive.
    pub const fn voltage(self) -> BatteryVoltage {
        self.0
    }
}

/// Refloat realtime charging payload values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeChargingPayload {
    current: RefloatRealtimeChargingCurrent,
    voltage: RefloatRealtimeChargingVoltage,
}

impl RefloatRealtimeChargingPayload {
    /// Build typed Refloat realtime charging values.
    pub const fn new(
        current: RefloatRealtimeChargingCurrent,
        voltage: RefloatRealtimeChargingVoltage,
    ) -> Self {
        Self { current, voltage }
    }

    /// Return `charging.current`.
    pub const fn current(self) -> RefloatRealtimeChargingCurrent {
        self.current
    }

    /// Return `charging.voltage`.
    pub const fn voltage(self) -> RefloatRealtimeChargingVoltage {
        self.voltage
    }
}

/// Refloat alert ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatAlertId {
    /// Firmware fault alert.
    FirmwareFault,
}

impl RefloatAlertId {
    /// Return the Refloat `v1.2.1` alert ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::FirmwareFault => 1,
        }
    }

    const fn mask(self) -> u32 {
        1 << (self.id() - 1)
    }
}

/// Refloat active-alert mask appended to realtime data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RefloatRealtimeAlertMask(u32);

impl RefloatRealtimeAlertMask {
    /// Build an empty active-alert mask.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Return a copy with the alert marked active.
    pub const fn with_alert(self, alert: RefloatAlertId) -> Self {
        Self(self.0 | alert.mask())
    }

    /// Return whether the alert is active.
    pub const fn contains(self, alert: RefloatAlertId) -> bool {
        self.0 & alert.mask() != 0
    }

    /// Return the Refloat-compatible active-alert mask.
    pub const fn active_alert_mask_compat(self) -> u32 {
        self.0
    }
}

/// Refloat reserved realtime tail flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RefloatRealtimeReservedFlags(u32);

impl RefloatRealtimeReservedFlags {
    /// Build the currently empty Refloat realtime extra-flags field.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Return the Refloat-compatible extra-flags value.
    pub const fn extra_flags_compat(self) -> u32 {
        self.0
    }
}

/// Refloat firmware fault code appended to realtime data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RefloatFirmwareFaultCode(u8);

impl RefloatFirmwareFaultCode {
    /// Build a firmware fault-code token from the app-data compatible byte.
    pub const fn from_compat_code(code: u8) -> Self {
        Self(code)
    }

    /// Return the app-data compatible firmware fault-code byte.
    pub const fn compat_code(self) -> u8 {
        self.0
    }
}

/// Refloat realtime tail fields appended after conditional payload values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRealtimeTail {
    active_alerts: RefloatRealtimeAlertMask,
    reserved_flags: RefloatRealtimeReservedFlags,
    firmware_fault_code: RefloatFirmwareFaultCode,
}

impl RefloatRealtimeTail {
    /// Build typed Refloat realtime tail fields.
    pub const fn new(
        active_alerts: RefloatRealtimeAlertMask,
        reserved_flags: RefloatRealtimeReservedFlags,
        firmware_fault_code: RefloatFirmwareFaultCode,
    ) -> Self {
        Self {
            active_alerts,
            reserved_flags,
            firmware_fault_code,
        }
    }

    /// Return active alerts.
    pub const fn active_alerts(self) -> RefloatRealtimeAlertMask {
        self.active_alerts
    }

    /// Return reserved extra flags.
    pub const fn reserved_flags(self) -> RefloatRealtimeReservedFlags {
        self.reserved_flags
    }

    /// Return firmware fault code.
    pub const fn firmware_fault_code(self) -> RefloatFirmwareFaultCode {
        self.firmware_fault_code
    }
}

/// Refloat compact all-data attitude fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataAttitude {
    balance_pitch: RefloatRealtimeBalancePitch,
    roll: ImuRoll,
    pitch: ImuPitch,
}

impl RefloatAllDataAttitude {
    /// Build typed compact all-data attitude fields.
    pub const fn new(
        balance_pitch: RefloatRealtimeBalancePitch,
        roll: ImuRoll,
        pitch: ImuPitch,
    ) -> Self {
        Self {
            balance_pitch,
            roll,
            pitch,
        }
    }

    /// Return balance pitch.
    pub const fn balance_pitch(self) -> RefloatRealtimeBalancePitch {
        self.balance_pitch
    }

    /// Return IMU roll.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }

    /// Return IMU pitch.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }
}

/// Refloat compact all-data status fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatAllDataStatus {
    ride_state: RefloatRideState,
    beep_reason: RefloatBeepReason,
}

impl RefloatAllDataStatus {
    /// Build typed compact all-data status fields.
    pub const fn new(ride_state: RefloatRideState, beep_reason: RefloatBeepReason) -> Self {
        Self {
            ride_state,
            beep_reason,
        }
    }

    /// Return ride state.
    pub const fn ride_state(self) -> RefloatRideState {
        self.ride_state
    }

    /// Return beep reason.
    pub const fn beep_reason(self) -> RefloatBeepReason {
        self.beep_reason
    }
}

/// Refloat compact all-data FOC ID current state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefloatFocIdCurrent {
    /// A measured FOC ID current is available.
    Measured(MotorCurrent),
    /// Refloat will emit its source-backed unavailable marker during encoding.
    Unavailable,
}

impl RefloatFocIdCurrent {
    /// Build a measured FOC ID current value.
    pub const fn measured(current: MotorCurrent) -> Self {
        Self::Measured(current)
    }

    /// Build an unavailable FOC ID current marker.
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    /// Return the measured current, when available.
    pub const fn as_measured(self) -> Option<MotorCurrent> {
        match self {
            Self::Measured(current) => Some(current),
            Self::Unavailable => None,
        }
    }
}

/// Refloat compact all-data motor fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMotorPayload {
    battery_voltage: BatteryVoltage,
    electrical_speed: ElectricalSpeed,
    vehicle_speed: VehicleSpeed,
    motor_current: MotorCurrent,
    battery_current: BatteryCurrent,
    duty_cycle: DutyCycle,
    foc_id_current: RefloatFocIdCurrent,
}

impl RefloatAllDataMotorPayload {
    /// Build typed compact all-data motor fields.
    pub const fn new(
        battery_voltage: BatteryVoltage,
        electrical_speed: ElectricalSpeed,
        vehicle_speed: VehicleSpeed,
        motor_current: MotorCurrent,
        battery_current: BatteryCurrent,
        duty_cycle: DutyCycle,
        foc_id_current: RefloatFocIdCurrent,
    ) -> Self {
        Self {
            battery_voltage,
            electrical_speed,
            vehicle_speed,
            motor_current,
            battery_current,
            duty_cycle,
            foc_id_current,
        }
    }

    /// Return battery voltage.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }

    /// Return motor fields with refreshed battery voltage.
    pub const fn with_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            battery_voltage,
            electrical_speed: self.electrical_speed,
            vehicle_speed: self.vehicle_speed,
            motor_current: self.motor_current,
            battery_current: self.battery_current,
            duty_cycle: self.duty_cycle,
            foc_id_current: self.foc_id_current,
        }
    }

    /// Return electrical speed.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return vehicle speed.
    pub const fn vehicle_speed(self) -> VehicleSpeed {
        self.vehicle_speed
    }

    /// Return motor current.
    pub const fn motor_current(self) -> MotorCurrent {
        self.motor_current
    }

    /// Return battery current.
    pub const fn battery_current(self) -> BatteryCurrent {
        self.battery_current
    }

    /// Return duty cycle.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return FOC ID current state.
    pub const fn foc_id_current(self) -> RefloatFocIdCurrent {
        self.foc_id_current
    }
}

/// Refloat compact all-data base payload fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataBasePayload {
    balance_current: RefloatRealtimeBalanceCurrent,
    attitude: RefloatAllDataAttitude,
    status: RefloatAllDataStatus,
    footpad: FootpadSensorSample,
    setpoints: RefloatRealtimeRuntimeSetpoints,
    booster_current: RefloatRealtimeBoosterCurrent,
    motor: RefloatAllDataMotorPayload,
}

impl RefloatAllDataBasePayload {
    /// Build typed compact all-data base payload fields.
    pub const fn new(
        balance_current: RefloatRealtimeBalanceCurrent,
        attitude: RefloatAllDataAttitude,
        status: RefloatAllDataStatus,
        footpad: FootpadSensorSample,
        setpoints: RefloatRealtimeRuntimeSetpoints,
        booster_current: RefloatRealtimeBoosterCurrent,
        motor: RefloatAllDataMotorPayload,
    ) -> Self {
        Self {
            balance_current,
            attitude,
            status,
            footpad,
            setpoints,
            booster_current,
            motor,
        }
    }

    /// Return the Refloat app-data command this payload belongs to.
    pub const fn command(self) -> RefloatAppDataCommand {
        RefloatAppDataCommand::GetAllData
    }

    /// Encode the compact all-data base response bytes.
    pub fn encode_base_response(&self, mode: u8) -> [u8; 34] {
        let mut buffer = [0; 34];
        let mut ind = 0;

        refloat_push_u8(&mut buffer, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
        refloat_push_u8(&mut buffer, &mut ind, self.command().id());
        refloat_push_u8(&mut buffer, &mut ind, mode);
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.balance_current.current().current().as_amps(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.attitude.balance_pitch().angle().as_radians(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.attitude.roll().angle().as_radians(),
            10.0,
        );

        let ride_state = self.status.ride_state;
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            (ride_state.float_state_compat() & 0x0f)
                + (ride_state.setpoint_adjustment_compat() << 4),
        );

        let handtest = matches!(ride_state.mode, RefloatMode::HandTest);
        let switch_state = self.footpad.state().switch_compat() | u8::from(handtest) << 3;
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            (switch_state & 0x0f) + (self.status.beep_reason.id() << 4),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_scaled_u8(self.footpad.adc1().ratio().as_ratio(), 50.0),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_scaled_u8(self.footpad.adc2().ratio().as_ratio(), 50.0),
        );

        [
            self.setpoints.board(),
            self.setpoints.atr(),
            self.setpoints.brake_tilt(),
            self.setpoints.torque_tilt(),
            self.setpoints.turn_tilt(),
            self.setpoints.remote(),
        ]
        .into_iter()
        .map(|setpoint| refloat_offset_scaled_u8(setpoint.angle().as_degrees(), 5.0, 128.0))
        .for_each(|value| refloat_push_u8(&mut buffer, &mut ind, value));

        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.attitude.pitch().angle().as_radians(),
            10.0,
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_offset_scaled_u8(
                self.booster_current.current().current().as_amps(),
                1.0,
                128.0,
            ),
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.battery_voltage().voltage().as_volts(),
            10.0,
        );
        refloat_push_i16(
            &mut buffer,
            &mut ind,
            self.motor
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute() as i16,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.vehicle_speed().speed().as_meters_per_second(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.motor_current().current().as_amps(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.battery_current().current().as_amps(),
            10.0,
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_offset_scaled_u8(self.motor.duty_cycle().ratio().as_ratio(), 100.0, 128.0),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            self.motor
                .foc_id_current()
                .as_measured()
                .map_or(222, |current| {
                    refloat_scaled_u8(current.current().as_amps().abs(), 3.0)
                }),
        );

        buffer
    }

    /// Encode the compact all-data mode 4 response bytes.
    pub fn encode_mode4_response(
        &self,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> [u8; 58] {
        self.encode_mode4_response_for_mode(4, mode2, mode3, mode4)
    }

    /// Encode the compact all-data mode 2 response bytes.
    pub fn encode_mode2_response(
        &self,
        mode: RefloatAllDataMode,
        mode2: RefloatAllDataMode2Payload,
    ) -> [u8; 41] {
        let mut buffer = [0; 41];
        let base = self.encode_base_response(mode.source_id());
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);

        buffer
    }

    /// Encode the compact all-data mode 3 response bytes.
    pub fn encode_mode3_response(
        &self,
        mode: RefloatAllDataMode,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
    ) -> [u8; 54] {
        let mut buffer = [0; 54];
        let base = self.encode_base_response(mode.source_id());
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);
        refloat_append_all_data_mode3(&mut buffer, &mut ind, mode3);

        buffer
    }

    fn encode_mode4_response_for_mode(
        &self,
        mode: u8,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> [u8; 58] {
        let mut buffer = [0; 58];
        let base = self.encode_base_response(mode);
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);
        refloat_append_all_data_mode3(&mut buffer, &mut ind, mode3);
        refloat_append_all_data_mode4(&mut buffer, &mut ind, mode4);

        buffer
    }

    /// Return balance current.
    pub const fn balance_current(self) -> RefloatRealtimeBalanceCurrent {
        self.balance_current
    }

    /// Return attitude fields.
    pub const fn attitude(self) -> RefloatAllDataAttitude {
        self.attitude
    }

    /// Return status fields.
    pub const fn status(self) -> RefloatAllDataStatus {
        self.status
    }

    /// Return footpad sample.
    pub const fn footpad(self) -> FootpadSensorSample {
        self.footpad
    }

    /// Return runtime setpoints.
    pub const fn setpoints(self) -> RefloatRealtimeRuntimeSetpoints {
        self.setpoints
    }

    /// Return booster current.
    pub const fn booster_current(self) -> RefloatRealtimeBoosterCurrent {
        self.booster_current
    }

    /// Return motor payload.
    pub const fn motor(self) -> RefloatAllDataMotorPayload {
        self.motor
    }

    /// Return base all-data fields with refreshed motor battery voltage.
    pub const fn with_motor_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            balance_current: self.balance_current,
            attitude: self.attitude,
            status: self.status,
            footpad: self.footpad,
            setpoints: self.setpoints,
            booster_current: self.booster_current,
            motor: self.motor.with_battery_voltage(battery_voltage),
        }
    }
}

/// Refloat all-data payload snapshot used to answer compact all-data requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataPayloads {
    base: RefloatAllDataBasePayload,
    mode2: RefloatAllDataMode2Payload,
    mode3: RefloatAllDataMode3Payload,
    mode4: RefloatAllDataMode4Payload,
}

impl RefloatAllDataPayloads {
    /// Build a complete all-data payload snapshot.
    pub const fn new(
        base: RefloatAllDataBasePayload,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> Self {
        Self {
            base,
            mode2,
            mode3,
            mode4,
        }
    }

    /// Build the Refloat `v1.2.1` startup all-data snapshot after `data_init`.
    ///
    /// Upstream zeroes and initializes `Data` in `src/main.c:1190-1205`; this
    /// Rust snapshot is a test/default model, not proof of hardware state.
    pub const fn source_startup() -> Self {
        let zero_current = Current::from_amps(0.0);
        let zero_angle = AngleRadians::from_radians(0.0);
        let zero_motor_current = MotorCurrent::new(zero_current);
        let zero_battery_current = BatteryCurrent::new(zero_current);
        let zero_voltage = BatteryVoltage::new(Voltage::from_volts(0.0));
        let ride_state = RefloatRideState::new(
            RefloatRunState::Startup,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        Self::new(
            RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(zero_motor_current),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(zero_angle),
                    ImuRoll::new(zero_angle),
                    ImuPitch::new(zero_angle),
                ),
                RefloatAllDataStatus::new(ride_state, RefloatBeepReason::None),
                FootpadSensorSample::new(
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    FootpadSensorState::None,
                ),
                RefloatRealtimeRuntimeSetpoints::new(
                    setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
                ),
                RefloatRealtimeBoosterCurrent::new(zero_motor_current),
                RefloatAllDataMotorPayload::new(
                    zero_voltage,
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                    zero_motor_current,
                    zero_battery_current,
                    DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
                    RefloatFocIdCurrent::unavailable(),
                ),
            ),
            RefloatAllDataMode2Payload::new(
                TripDistance::new(Distance::from_meters(0.0)),
                RefloatRealtimeMotorTemperatures::new(
                    MosfetTemperature::new(Temperature::from_degrees_celsius(0.0)),
                    MotorTemperature::new(Temperature::from_degrees_celsius(0.0)),
                ),
                RefloatAllDataBatteryTemperature::unavailable(),
            ),
            RefloatAllDataMode3Payload::new(
                OdometerMeters::from_meters(0),
                AmpHoursDischarged::new(Charge::from_amp_hours(0.0)),
                AmpHoursCharged::new(Charge::from_amp_hours(0.0)),
                WattHoursDischarged::new(Energy::from_watt_hours(0.0)),
                WattHoursCharged::new(Energy::from_watt_hours(0.0)),
                BatteryLevel::new(Ratio::from_ratio_const(0.0)),
            ),
            RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(zero_battery_current),
                RefloatRealtimeChargingVoltage::new(zero_voltage),
            ),
        )
    }

    /// Encode the source-compatible response for a parsed all-data request.
    ///
    /// The byte order and mode gates mirror `cmd_send_all_data` in upstream
    /// `src/main.c:1313-1399`.
    #[inline(never)]
    pub fn encode_response(&self, request: RefloatAllDataRequest) -> RefloatAllDataResponse {
        let mode = request.mode();
        if mode.includes_mode4() {
            RefloatAllDataResponse::Mode4(self.base.encode_mode4_response_for_mode(
                mode.source_id(),
                self.mode2,
                self.mode3,
                self.mode4,
            ))
        } else if mode.includes_mode3() {
            RefloatAllDataResponse::Mode3(
                self.base
                    .encode_mode3_response(mode, self.mode2, self.mode3),
            )
        } else if mode.includes_mode2() {
            RefloatAllDataResponse::Mode2(self.base.encode_mode2_response(mode, self.mode2))
        } else {
            RefloatAllDataResponse::Base(self.base.encode_base_response(mode.source_id()))
        }
    }

    /// Return base all-data payload fields.
    pub const fn base(self) -> RefloatAllDataBasePayload {
        self.base
    }

    /// Return mode 2 all-data extension fields.
    pub const fn mode2(self) -> RefloatAllDataMode2Payload {
        self.mode2
    }

    /// Return a payload snapshot with refreshed base battery voltage.
    pub const fn with_base_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self::new(
            self.base.with_motor_battery_voltage(battery_voltage),
            self.mode2,
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed absolute-distance mode 2 data.
    pub const fn with_mode2_distance_abs(self, distance_abs: TripDistance) -> Self {
        Self::new(
            self.base,
            self.mode2.with_distance_abs(distance_abs),
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed mode 2 motor temperatures.
    pub const fn with_mode2_temperatures(
        self,
        temperatures: RefloatRealtimeMotorTemperatures,
    ) -> Self {
        Self::new(
            self.base,
            self.mode2.with_temperatures(temperatures),
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed mode 3 ride totals.
    pub const fn with_mode3_ride_totals(self, mode3: RefloatAllDataMode3Payload) -> Self {
        Self::new(self.base, self.mode2, mode3, self.mode4)
    }

    /// Return mode 3 all-data extension fields.
    pub const fn mode3(self) -> RefloatAllDataMode3Payload {
        self.mode3
    }

    /// Return a payload snapshot with refreshed mode 4 charging data.
    pub const fn with_mode4_charging(self, mode4: RefloatAllDataMode4Payload) -> Self {
        Self::new(self.base, self.mode2, self.mode3, mode4)
    }

    /// Return mode 4 all-data extension fields.
    pub const fn mode4(self) -> RefloatAllDataMode4Payload {
        self.mode4
    }
}

/// Fixed-size Refloat all-data response bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefloatAllDataResponse {
    /// Fault response bytes.
    Fault([u8; 4]),
    /// Base response bytes.
    Base([u8; 34]),
    /// Mode 2 response bytes.
    Mode2([u8; 41]),
    /// Mode 3 response bytes.
    Mode3([u8; 54]),
    /// Mode 4 response bytes.
    Mode4([u8; 58]),
}

impl RefloatAllDataResponse {
    /// Encode a Refloat all-data fault response.
    pub const fn fault(fault: RefloatFirmwareFaultCode) -> Self {
        Self::Fault([
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            69,
            fault.compat_code(),
        ])
    }

    /// Return the encoded response bytes.
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Fault(bytes) => bytes,
            Self::Base(bytes) => bytes,
            Self::Mode2(bytes) => bytes,
            Self::Mode3(bytes) => bytes,
            Self::Mode4(bytes) => bytes,
        }
    }
}

fn refloat_append_all_data_mode2(
    buffer: &mut [u8],
    ind: &mut usize,
    mode2: RefloatAllDataMode2Payload,
) {
    refloat_push_float32_auto(buffer, ind, mode2.distance_abs().distance().as_meters());
    refloat_push_u8(
        buffer,
        ind,
        refloat_nonnegative_scaled_u8(
            mode2
                .temperatures()
                .mosfet()
                .temperature()
                .as_degrees_celsius(),
            2.0,
        ),
    );
    refloat_push_u8(
        buffer,
        ind,
        refloat_nonnegative_scaled_u8(
            mode2
                .temperatures()
                .motor()
                .temperature()
                .as_degrees_celsius(),
            2.0,
        ),
    );
    refloat_push_u8(
        buffer,
        ind,
        mode2.battery_temperature().as_measured().map_or(0, |temp| {
            refloat_nonnegative_scaled_u8(temp.as_degrees_celsius(), 2.0)
        }),
    );
}

fn refloat_append_all_data_mode3(
    buffer: &mut [u8],
    ind: &mut usize,
    mode3: RefloatAllDataMode3Payload,
) {
    refloat_push_u32(buffer, ind, mode3.odometer().as_meters() as u32);
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.discharged_charge().charge().as_amp_hours(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.charged_charge().charge().as_amp_hours(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.discharged_energy().energy().as_watt_hours(),
        1.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.charged_energy().energy().as_watt_hours(),
        1.0,
    );
    refloat_push_u8(
        buffer,
        ind,
        refloat_scaled_u8(mode3.battery_level().ratio().as_ratio().min(1.25), 200.0),
    );
}

fn refloat_append_all_data_mode4(
    buffer: &mut [u8],
    ind: &mut usize,
    mode4: RefloatAllDataMode4Payload,
) {
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode4.current().current().current().as_amps(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode4.voltage().voltage().voltage().as_volts(),
        10.0,
    );
}

fn refloat_push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    if let Some(slot) = buffer.get_mut(*ind) {
        *slot = value;
    }
    *ind = ind.saturating_add(1);
}

fn refloat_push_i16(buffer: &mut [u8], ind: &mut usize, value: i16) {
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| refloat_push_u8(buffer, ind, byte));
}

fn refloat_push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| refloat_push_u8(buffer, ind, byte));
}

fn refloat_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    refloat_push_u32(buffer, ind, value.to_bits());
}

fn refloat_push_scaled_i16(buffer: &mut [u8], ind: &mut usize, value: f32, scale: f32) {
    refloat_push_i16(buffer, ind, (value * scale) as i16);
}

fn refloat_scaled_u8(value: f32, scale: f32) -> u8 {
    (value * scale) as u8
}

fn refloat_nonnegative_scaled_u8(value: f32, scale: f32) -> u8 {
    refloat_scaled_u8(value.max(0.0), scale)
}

fn refloat_offset_scaled_u8(value: f32, scale: f32, offset: f32) -> u8 {
    (value * scale + offset) as u8
}

/// Refloat all-data battery-temperature state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefloatAllDataBatteryTemperature {
    /// A measured battery temperature is available.
    Measured(Temperature),
    /// Refloat `v1.2.1` emits a zero placeholder for this field.
    Unavailable,
}

impl RefloatAllDataBatteryTemperature {
    /// Build a measured battery-temperature value.
    pub const fn measured(temperature: Temperature) -> Self {
        Self::Measured(temperature)
    }

    /// Build an unavailable battery-temperature marker.
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    /// Return the measured battery temperature, when available.
    pub const fn as_measured(self) -> Option<Temperature> {
        match self {
            Self::Measured(temperature) => Some(temperature),
            Self::Unavailable => None,
        }
    }
}

/// Refloat all-data mode 2 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode2Payload {
    distance_abs: TripDistance,
    temperatures: RefloatRealtimeMotorTemperatures,
    battery_temperature: RefloatAllDataBatteryTemperature,
}

impl RefloatAllDataMode2Payload {
    /// Build typed all-data mode 2 extension fields.
    pub const fn new(
        distance_abs: TripDistance,
        temperatures: RefloatRealtimeMotorTemperatures,
        battery_temperature: RefloatAllDataBatteryTemperature,
    ) -> Self {
        Self {
            distance_abs,
            temperatures,
            battery_temperature,
        }
    }

    /// Return absolute distance.
    pub const fn distance_abs(self) -> TripDistance {
        self.distance_abs
    }

    /// Return mode 2 fields with refreshed absolute distance.
    pub const fn with_distance_abs(self, distance_abs: TripDistance) -> Self {
        Self::new(distance_abs, self.temperatures, self.battery_temperature)
    }

    /// Return mode 2 fields with refreshed motor temperatures.
    pub const fn with_temperatures(self, temperatures: RefloatRealtimeMotorTemperatures) -> Self {
        Self::new(self.distance_abs, temperatures, self.battery_temperature)
    }

    /// Return motor temperatures.
    pub const fn temperatures(self) -> RefloatRealtimeMotorTemperatures {
        self.temperatures
    }

    /// Return battery-temperature state.
    pub const fn battery_temperature(self) -> RefloatAllDataBatteryTemperature {
        self.battery_temperature
    }
}

/// Refloat all-data mode 3 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode3Payload {
    odometer: OdometerMeters,
    discharged_charge: AmpHoursDischarged,
    charged_charge: AmpHoursCharged,
    discharged_energy: WattHoursDischarged,
    charged_energy: WattHoursCharged,
    battery_level: BatteryLevel,
}

impl RefloatAllDataMode3Payload {
    /// Build typed all-data mode 3 extension fields.
    pub const fn new(
        odometer: OdometerMeters,
        discharged_charge: AmpHoursDischarged,
        charged_charge: AmpHoursCharged,
        discharged_energy: WattHoursDischarged,
        charged_energy: WattHoursCharged,
        battery_level: BatteryLevel,
    ) -> Self {
        Self {
            odometer,
            discharged_charge,
            charged_charge,
            discharged_energy,
            charged_energy,
            battery_level,
        }
    }

    /// Return odometer distance.
    pub const fn odometer(self) -> OdometerMeters {
        self.odometer
    }

    /// Return discharged amp-hours.
    pub const fn discharged_charge(self) -> AmpHoursDischarged {
        self.discharged_charge
    }

    /// Return charged amp-hours.
    pub const fn charged_charge(self) -> AmpHoursCharged {
        self.charged_charge
    }

    /// Return discharged watt-hours.
    pub const fn discharged_energy(self) -> WattHoursDischarged {
        self.discharged_energy
    }

    /// Return charged watt-hours.
    pub const fn charged_energy(self) -> WattHoursCharged {
        self.charged_energy
    }

    /// Return battery state of charge.
    pub const fn battery_level(self) -> BatteryLevel {
        self.battery_level
    }
}

/// Refloat all-data mode 4 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode4Payload {
    current: RefloatRealtimeChargingCurrent,
    voltage: RefloatRealtimeChargingVoltage,
}

impl RefloatAllDataMode4Payload {
    /// Build typed all-data mode 4 extension fields.
    pub const fn new(
        current: RefloatRealtimeChargingCurrent,
        voltage: RefloatRealtimeChargingVoltage,
    ) -> Self {
        Self { current, voltage }
    }

    /// Return charging current.
    pub const fn current(self) -> RefloatRealtimeChargingCurrent {
        self.current
    }

    /// Return charging voltage.
    pub const fn voltage(self) -> RefloatRealtimeChargingVoltage {
        self.voltage
    }
}

/// Refloat realtime-data header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRealtimeDataHeader {
    timestamp: SystemTimestamp,
    ride_state: RefloatRideState,
    footpad_state: FootpadSensorState,
    beep_reason: RefloatBeepReason,
    fatal_error: RefloatFatalErrorState,
    data_recorder: RefloatDataRecorderFlags,
}

impl RefloatRealtimeDataHeader {
    /// Build the typed realtime-data header state.
    pub const fn new(
        timestamp: SystemTimestamp,
        ride_state: RefloatRideState,
        footpad_state: FootpadSensorState,
        beep_reason: RefloatBeepReason,
    ) -> Self {
        Self {
            timestamp,
            ride_state,
            footpad_state,
            beep_reason,
            fatal_error: RefloatFatalErrorState::None,
            data_recorder: RefloatDataRecorderFlags::inactive(),
        }
    }

    /// Return this header with fatal-error state.
    pub const fn with_fatal_error(mut self, fatal_error: RefloatFatalErrorState) -> Self {
        self.fatal_error = fatal_error;
        self
    }

    /// Return this header with data-recorder flags.
    pub const fn with_data_recorder(mut self, data_recorder: RefloatDataRecorderFlags) -> Self {
        self.data_recorder = data_recorder;
        self
    }

    /// Return the typed VESC system timestamp.
    pub const fn timestamp(self) -> SystemTimestamp {
        self.timestamp
    }

    /// Return the Refloat `v1.2.1` realtime data mask byte.
    pub const fn data_mask_compat(self) -> u8 {
        let runtime = match self.ride_state.run_state {
            RefloatRunState::Running => 0x1,
            RefloatRunState::Disabled | RefloatRunState::Startup | RefloatRunState::Ready => 0,
        };
        let charging = match self.ride_state.charging {
            RefloatChargingState::NotCharging => 0,
            RefloatChargingState::Charging => 0x2,
        };

        runtime | charging | 0x4
    }

    /// Return the Refloat `v1.2.1` realtime extra-flags byte.
    pub const fn extra_flags_compat(self) -> u8 {
        self.data_recorder.extra_flags_compat(self.fatal_error)
    }

    /// Return the Refloat `v1.2.1` realtime mode/run-state byte.
    pub const fn state_byte_compat(self) -> u8 {
        self.ride_state.mode.id() << 4 | self.ride_state.run_state.id()
    }

    /// Return the Refloat `v1.2.1` realtime footpad/ride-flags byte.
    pub const fn footpad_flags_compat(self) -> u8 {
        let charging = match self.ride_state.charging {
            RefloatChargingState::NotCharging => 0,
            RefloatChargingState::Charging => 0x20,
        };
        let darkride = match self.ride_state.darkride {
            RefloatDarkRideState::Upright => 0,
            RefloatDarkRideState::Active => 0x2,
        };
        let wheelslip = match self.ride_state.wheelslip {
            RefloatWheelSlipState::None => 0,
            RefloatWheelSlipState::Detected => 0x1,
        };

        self.footpad_state.id() << 6 | charging | darkride | wheelslip
    }

    /// Return the Refloat `v1.2.1` realtime setpoint/stop byte.
    pub const fn stop_setpoint_byte_compat(self) -> u8 {
        self.ride_state.setpoint_adjustment.id() << 4 | self.ride_state.stop_condition.id()
    }

    /// Return the Refloat `v1.2.1` beep-reason byte.
    pub const fn beep_reason_compat(self) -> u8 {
        self.beep_reason.id()
    }
}

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

    /// Return the top-level run state.
    ///
    /// Mirrors upstream `d->state.state`, read by `set_cfg` at
    /// `src/main.c:2369-2372`.
    pub const fn run_state(self) -> RefloatRunState {
        self.run_state
    }

    /// Return the runtime mode.
    ///
    /// Mirrors upstream `d->state.mode`, read by `set_cfg` at
    /// `src/main.c:2362-2365`.
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

/// Refloat IMU sample used by ride logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatImuSample {
    pitch: ImuPitch,
    roll: ImuRoll,
    yaw: ImuYaw,
    angular_rate: ImuAngularRate,
}

impl RefloatImuSample {
    /// Build a typed Refloat IMU sample.
    pub const fn new(
        pitch: ImuPitch,
        roll: ImuRoll,
        yaw: ImuYaw,
        angular_rate: ImuAngularRate,
    ) -> Self {
        Self {
            pitch,
            roll,
            yaw,
            angular_rate,
        }
    }

    /// Return typed pitch.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }

    /// Return typed roll.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }

    /// Return typed yaw.
    pub const fn yaw(self) -> ImuYaw {
        self.yaw
    }

    /// Return typed angular-rate axes.
    pub const fn angular_rate(self) -> ImuAngularRate {
        self.angular_rate
    }
}

/// Refloat motor telemetry sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatMotorTelemetry {
    electrical_speed: ElectricalSpeed,
    vehicle_speed: VehicleSpeed,
    motor_current: DirectionalMotorCurrent,
    battery_current: BatteryCurrent,
    duty_cycle: DutyCycle,
    battery_voltage: BatteryVoltage,
}

impl RefloatMotorTelemetry {
    /// Build a typed Refloat motor telemetry sample.
    pub const fn new(
        electrical_speed: ElectricalSpeed,
        vehicle_speed: VehicleSpeed,
        motor_current: DirectionalMotorCurrent,
        battery_current: BatteryCurrent,
        duty_cycle: DutyCycle,
        battery_voltage: BatteryVoltage,
    ) -> Self {
        Self {
            electrical_speed,
            vehicle_speed,
            motor_current,
            battery_current,
            duty_cycle,
            battery_voltage,
        }
    }

    /// Return typed electrical speed.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return typed vehicle speed.
    pub const fn vehicle_speed(self) -> VehicleSpeed {
        self.vehicle_speed
    }

    /// Return typed directional motor current.
    pub const fn motor_current(self) -> DirectionalMotorCurrent {
        self.motor_current
    }

    /// Return typed battery current.
    pub const fn battery_current(self) -> BatteryCurrent {
        self.battery_current
    }

    /// Return typed duty cycle.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return typed battery voltage.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }
}

/// Refloat motor-current request.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatMotorCommand {
    requested_current: MotorCurrent,
}

impl RefloatMotorCommand {
    /// Build a motor command from typed requested current.
    pub const fn new(requested_current: MotorCurrent) -> Self {
        Self { requested_current }
    }

    /// Return the typed requested current.
    pub const fn requested_current(self) -> MotorCurrent {
        self.requested_current
    }
}
