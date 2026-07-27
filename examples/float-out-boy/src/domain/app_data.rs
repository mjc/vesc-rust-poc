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
#[repr(u8)]
pub enum FloatOutBoyAppDataCommand {
    /// Version/package info.
    Info = 0,
    /// Realtime data request.
    GetRealtimeData = 1,
    /// Runtime tune without EEPROM write.
    RuntimeTune = 2,
    /// Reset tune defaults without EEPROM write.
    TuneDefaults = 3,
    /// Save config to EEPROM.
    ConfigSave = 4,
    /// Restore config from EEPROM.
    ConfigRestore = 5,
    /// Runtime startup/config change.
    TuneOther = 6,
    /// Idle motor movement.
    RcMove = 7,
    /// Booster settings.
    Booster = 8,
    /// Print verbose info.
    PrintInfo = 9,
    /// Compact all-data response request.
    GetAllData = 10,
    /// Testing/tuning experiment command.
    Experiment = 11,
    /// Lock/disable command.
    Lock = 12,
    /// Hand-test mode command.
    HandTest = 13,
    /// Tilt tuning command.
    TuneTilt = 14,
    /// Lights-control command.
    LightsControl = 20,
    /// Flywheel toggle command.
    Flywheel = 22,
    /// LCM poll.
    LcmPoll = 24,
    /// LCM light-info request.
    LcmLightInfo = 25,
    /// LCM light-control command.
    LcmLightControl = 26,
    /// LCM device-info request.
    LcmDeviceInfo = 27,
    /// Charging-state command.
    ChargingState = 28,
    /// LCM battery request.
    LcmGetBattery = 29,
    /// Realtime data path.
    RealtimeData = 31,
    /// Realtime data ID list.
    RealtimeDataIds = 32,
    /// Alert list request.
    AlertsList = 35,
    /// Alert control command.
    AlertsControl = 36,
    /// Data recorder request.
    DataRecordRequest = 41,
    /// Data recorder header response.
    DataRecordHeader = 42,
    /// Data recorder sample-data response.
    DataRecordData = 43,
    /// LCM debug command reserved for external debugging.
    LcmDebug = 99,
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
            42 => Ok(Self::DataRecordHeader),
            43 => Ok(Self::DataRecordData),
            99 => Ok(Self::LcmDebug),
            value => Err(FloatOutBoyAppDataCommandError { value }),
        }
    }

    /// Return the Float Out Boy `v1.2.1` command ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminants are the source-pinned wire IDs"
    )]
    pub const fn id(self) -> u8 {
        self as u8
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

#[cfg(test)]
mod tests {
    use super::FloatOutBoyAppDataCommand;

    #[test]
    fn every_refloat_command_id_round_trips_through_the_typed_model() {
        for (command, id) in [
            (FloatOutBoyAppDataCommand::Info, 0),
            (FloatOutBoyAppDataCommand::GetRealtimeData, 1),
            (FloatOutBoyAppDataCommand::RuntimeTune, 2),
            (FloatOutBoyAppDataCommand::TuneDefaults, 3),
            (FloatOutBoyAppDataCommand::ConfigSave, 4),
            (FloatOutBoyAppDataCommand::ConfigRestore, 5),
            (FloatOutBoyAppDataCommand::TuneOther, 6),
            (FloatOutBoyAppDataCommand::RcMove, 7),
            (FloatOutBoyAppDataCommand::Booster, 8),
            (FloatOutBoyAppDataCommand::PrintInfo, 9),
            (FloatOutBoyAppDataCommand::GetAllData, 10),
            (FloatOutBoyAppDataCommand::Experiment, 11),
            (FloatOutBoyAppDataCommand::Lock, 12),
            (FloatOutBoyAppDataCommand::HandTest, 13),
            (FloatOutBoyAppDataCommand::TuneTilt, 14),
            (FloatOutBoyAppDataCommand::LightsControl, 20),
            (FloatOutBoyAppDataCommand::Flywheel, 22),
            (FloatOutBoyAppDataCommand::LcmPoll, 24),
            (FloatOutBoyAppDataCommand::LcmLightInfo, 25),
            (FloatOutBoyAppDataCommand::LcmLightControl, 26),
            (FloatOutBoyAppDataCommand::LcmDeviceInfo, 27),
            (FloatOutBoyAppDataCommand::ChargingState, 28),
            (FloatOutBoyAppDataCommand::LcmGetBattery, 29),
            (FloatOutBoyAppDataCommand::RealtimeData, 31),
            (FloatOutBoyAppDataCommand::RealtimeDataIds, 32),
            (FloatOutBoyAppDataCommand::AlertsList, 35),
            (FloatOutBoyAppDataCommand::AlertsControl, 36),
            (FloatOutBoyAppDataCommand::DataRecordRequest, 41),
            (FloatOutBoyAppDataCommand::DataRecordHeader, 42),
            (FloatOutBoyAppDataCommand::DataRecordData, 43),
            (FloatOutBoyAppDataCommand::LcmDebug, 99),
        ] {
            assert_eq!(command.id(), id);
            assert_eq!(FloatOutBoyAppDataCommand::try_from_id(id), Ok(command));
        }
    }
}
