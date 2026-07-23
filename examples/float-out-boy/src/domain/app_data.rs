//! Float Out Boy app-data protocol types.
//!
//! These types own the protocol-shaped command IDs and all-data request
//! parsing, while `domain.rs` keeps the semantic payload types and wire helpers.
//!
//! Source anchors for the compatibility surface below are Float Out Boy `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/float-out-boy/src/main.c:1241-1262` defines the core app-data command IDs.
//! - `third_party/float-out-boy/src/lcm.h:27-33` and `third_party/float-out-boy/src/charging.h:25`
//!   define LCM/charging command IDs.
//! - `third_party/float-out-boy/src/main.c:2210-2215` defines `COMMAND_GET_ALLDATA`.
//! - `third_party/float-out-boy/src/main.c:1313-1399` defines the all-data response layout.

/// Float Out Boy app-data package ID; upstream writes literal `101` in
/// `third_party/float-out-boy/src/main.c:1271`, `1318`, `1881`, and `1909`.
pub const FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID: FloatOutBoyAppDataPackageId =
    FloatOutBoyAppDataPackageId::new(101);

/// Float Out Boy app-data package identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FloatOutBoyAppDataPackageId(u8);

impl FloatOutBoyAppDataPackageId {
    /// Build a package ID token from the source-backed package ID.
    const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Explicitly extract the app-data package ID.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Float Out Boy app-data command IDs.
///
/// Float Out Boy `v1.2.1` defines the core IDs in `third_party/float-out-boy/src/main.c:1241-1262`,
/// LCM IDs in `third_party/float-out-boy/src/lcm.h:27-33`, and charging state in
/// `third_party/float-out-boy/src/charging.h:25`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyAppDataCommand {
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

impl FloatOutBoyAppDataCommand {
    /// Parse a Float Out Boy app-data command ID.
    ///
    /// # Errors
    ///
    /// Returns [`FloatOutBoyAppDataCommandError`] when `id` is not one of the
    /// command bytes defined by Float Out Boy.
    pub const fn try_from_id(id: u8) -> Result<Self, FloatOutBoyAppDataCommandError> {
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
            value => Err(FloatOutBoyAppDataCommandError { value }),
        }
    }

    /// Return the Float Out Boy `v1.2.1` command ID.
    #[must_use]
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

/// Error returned when a Float Out Boy app-data command ID is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyAppDataCommandError {
    value: u8,
}

impl FloatOutBoyAppDataCommandError {
    /// Return the rejected command ID.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.value
    }
}

/// Float Out Boy all-data request mode byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FloatOutBoyAllDataMode {
    source_id: u8,
}

impl FloatOutBoyAllDataMode {
    /// Build a mode token from the upstream Float Out Boy request byte.
    #[must_use]
    pub const fn from_source_id(source_id: u8) -> Self {
        Self { source_id }
    }

    /// Build a base all-data request mode.
    #[must_use]
    pub const fn base() -> Self {
        Self::from_source_id(1)
    }

    /// Build a request mode that includes mode 2 fields.
    #[must_use]
    pub const fn with_mode2() -> Self {
        Self::from_source_id(2)
    }

    /// Build a request mode that includes mode 2 and 3 fields.
    #[must_use]
    pub const fn with_mode3() -> Self {
        Self::from_source_id(3)
    }

    /// Build a request mode that includes mode 2, 3, and 4 fields.
    #[must_use]
    pub const fn with_mode4() -> Self {
        Self::from_source_id(4)
    }

    /// Return the source request byte.
    #[must_use]
    pub const fn source_id(self) -> u8 {
        self.source_id
    }

    /// Return whether the mode includes mode 2 extension fields.
    #[must_use]
    pub const fn includes_mode2(self) -> bool {
        self.source_id >= 2
    }

    /// Return whether the mode includes mode 3 extension fields.
    #[must_use]
    pub const fn includes_mode3(self) -> bool {
        self.source_id >= 3
    }

    /// Return whether the mode includes mode 4 extension fields.
    #[must_use]
    pub const fn includes_mode4(self) -> bool {
        self.source_id >= 4
    }
}

/// Float Out Boy all-data request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyAllDataRequest {
    mode: FloatOutBoyAllDataMode,
}

impl FloatOutBoyAllDataRequest {
    /// Build an all-data request.
    #[must_use]
    pub const fn new(mode: FloatOutBoyAllDataMode) -> Self {
        Self { mode }
    }

    /// Parse a Float Out Boy `COMMAND_GET_ALLDATA` app-data packet.
    ///
    /// Upstream dispatches this command at `third_party/float-out-boy/src/main.c:2210-2215`
    /// and encodes responses in `third_party/float-out-boy/src/main.c:1313-1399`.
    ///
    /// # Errors
    ///
    /// Returns [`FloatOutBoyAllDataRequestError`] when the packet has the wrong
    /// length, package ID, or command ID.
    pub fn parse(bytes: &[u8]) -> Result<Self, FloatOutBoyAllDataRequestError> {
        let [package_id, command_id, mode] = bytes else {
            return Err(FloatOutBoyAllDataRequestError::Length {
                actual: bytes.len(),
            });
        };

        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
            return Err(FloatOutBoyAllDataRequestError::PackageId { value: *package_id });
        }

        if *command_id != FloatOutBoyAppDataCommand::GetAllData.id() {
            return Err(FloatOutBoyAllDataRequestError::Command { value: *command_id });
        }

        Ok(Self::new(FloatOutBoyAllDataMode::from_source_id(*mode)))
    }

    /// Return the requested all-data mode.
    #[must_use]
    pub const fn mode(self) -> FloatOutBoyAllDataMode {
        self.mode
    }
}

/// Error returned when a Float Out Boy all-data request cannot be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyAllDataRequestError {
    /// The request length is not the Float Out Boy `v1.2.1` three-byte shape.
    Length {
        /// Actual request byte length.
        actual: usize,
    },
    /// The package ID does not match Float Out Boy.
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
