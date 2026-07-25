//! Documented VESC firmware function-table slots used by Rust packages.

#![expect(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::question_mark,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "capability probes are a compact manifest-facing API"
)]

use crate::{c_vesc_if, image::NativeAddress};

const PRESENCE_WORD_COUNT: usize = c_vesc_if::FIELD_COUNT.div_ceil(64);

/// One entry in the VESC firmware function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
    header_line: usize,
}

/// Whether a manifest entry is a callable function pointer or a scalar ABI word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSlotKind {
    /// A function-pointer entry that can be called when present.
    Function,
    /// A scalar ABI word or other non-callable entry.
    Scalar,
}

/// ABI family that owns a manifest entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSlotFamily {
    /// The STM32 controller-resident package ABI.
    Stm32,
}

/// Nullability represented by a generated STM32 table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSlotNullability {
    /// A function pointer may be absent in a tail or partial table.
    NullableFunction,
    /// A scalar table word is represented directly and is not a callable slot.
    NonNullableScalar,
}

/// Safety classification for the raw manifest surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSlotSafety {
    /// Calling the raw function pointer requires the sys-layer safety contract.
    RawFunction,
    /// The entry is a scalar ABI word rather than a callable pointer.
    ScalarWord,
}

/// Complete metadata for one entry in the pinned VESC firmware table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfManifestEntry {
    pub(crate) slot: VescIfSlot,
    pub(crate) kind: VescIfSlotKind,
    pub(crate) signature: &'static str,
}

impl VescIfManifestEntry {
    /// Return the slot identity and 32-bit offset.
    #[must_use]
    pub const fn slot(self) -> VescIfSlot {
        self.slot
    }

    /// Return the originating C declaration name from the pinned header.
    pub const fn c_decl(self) -> &'static str {
        self.slot.name()
    }

    /// Return whether the entry is callable through a function pointer.
    pub const fn kind(self) -> VescIfSlotKind {
        self.kind
    }

    /// Return the ABI family that owns this manifest entry.
    pub const fn family(self) -> VescIfSlotFamily {
        VescIfSlotFamily::Stm32
    }

    /// Return the minimum ordered ABI profile containing this entry.
    pub const fn since(self) -> Stm32AbiRevision {
        self.slot.minimum_revision()
    }

    /// Return whether this entry is nullable in the generated table.
    pub const fn nullability(self) -> VescIfSlotNullability {
        match self.kind {
            VescIfSlotKind::Function => VescIfSlotNullability::NullableFunction,
            VescIfSlotKind::Scalar => VescIfSlotNullability::NonNullableScalar,
        }
    }

    /// Return the bindgen-rendered Rust type for this ABI table entry.
    pub const fn signature(self) -> &'static str {
        self.signature
    }

    /// Return the raw safety class of this manifest entry.
    pub const fn safety(self) -> VescIfSlotSafety {
        match self.kind {
            VescIfSlotKind::Function => VescIfSlotSafety::RawFunction,
            VescIfSlotKind::Scalar => VescIfSlotSafety::ScalarWord,
        }
    }

    /// Return whether the entry is callable through a function pointer.
    #[must_use]
    pub const fn is_callable(self) -> bool {
        matches!(self.kind, VescIfSlotKind::Function)
    }
}

/// Observed slot presence for one concrete VESC firmware table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfPresence {
    bits: [u64; PRESENCE_WORD_COUNT],
}

impl VescIfPresence {
    /// Construct a bitmap with no observed entries.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            bits: [0; PRESENCE_WORD_COUNT],
        }
    }

    /// Inspect pointer-sized table words, preserving holes and scalar entries.
    #[must_use]
    pub fn from_words(words: &[usize]) -> Self {
        let mut presence = Self::empty();
        for (index, entry) in VescIfAbi::ALL_ENTRIES.iter().enumerate() {
            let Some(word) = words.get(index) else {
                continue;
            };
            if entry.is_callable() && *word == 0 {
                continue;
            }
            presence.set(index);
        }
        presence
    }

    /// Return whether a slot was observed as present.
    #[must_use]
    pub fn contains(self, slot: VescIfSlot) -> bool {
        self.contains_index(slot.slot_index())
    }

    /// Return whether a slot index was observed as present.
    #[must_use]
    pub fn contains_index(self, index: usize) -> bool {
        if index >= VescIfAbi::FIELD_COUNT {
            return false;
        }
        let Some(bits) = self.bits.get(index / 64) else {
            return false;
        };
        (*bits & (1_u64 << (index % 64))) != 0
    }

    /// Check a required capability and preserve the slot identity in the error.
    ///
    /// # Errors
    ///
    /// Returns [`AbiError::MissingRequired`] when `slot` is absent.
    pub fn require(self, capability: &'static str, slot: VescIfSlot) -> Result<(), AbiError> {
        if self.contains(slot) {
            Ok(())
        } else {
            Err(AbiError::MissingRequired { capability, slot })
        }
    }

    /// Check an optional capability without exposing raw table access to callers.
    ///
    /// # Errors
    ///
    /// Returns [`AbiError::Unsupported`] when `slot` is absent.
    pub fn optional(self, capability: &'static str, slot: VescIfSlot) -> Result<(), AbiError> {
        if self.contains(slot) {
            Ok(())
        } else {
            Err(AbiError::Unsupported { capability, slot })
        }
    }

    /// Return whether every callable slot in a revision profile is present.
    #[must_use]
    pub fn supports_revision(self, revision: Stm32AbiRevision) -> bool {
        revision.minimum_slot_count().is_some_and(|slot_count| {
            VescIfAbi::ALL_ENTRIES
                .get(..slot_count)
                .is_some_and(|entries| {
                    entries
                        .iter()
                        .enumerate()
                        .all(|(index, entry)| !entry.is_callable() || self.contains_index(index))
                })
        })
    }

    /// Infer the strongest descriptive profile supported by observed presence.
    #[must_use]
    pub fn revision(self) -> Stm32AbiRevision {
        if self.supports_revision(Stm32AbiRevision::Firmware606) {
            Stm32AbiRevision::Firmware606
        } else if self.supports_revision(Stm32AbiRevision::Firmware605) {
            Stm32AbiRevision::Firmware605
        } else if self.supports_revision(Stm32AbiRevision::Base) {
            Stm32AbiRevision::Base
        } else {
            Stm32AbiRevision::UnknownCompatible
        }
    }

    pub(crate) fn set(&mut self, index: usize) {
        if let Some(word) = self.bits.get_mut(index / 64) {
            *word |= 1_u64 << (index % 64);
        }
    }
}

/// Descriptive STM32 ABI profile; observed slot presence remains authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stm32AbiRevision {
    /// The table before the firmware 6.05 extension.
    Base,
    /// The table including the firmware 6.05 extension.
    Firmware605,
    /// The table including the firmware 6.06 extension.
    Firmware606,
    /// A table whose observed shape does not match a known profile.
    UnknownCompatible,
}

impl Stm32AbiRevision {
    /// Return the minimum ordered table width represented by this profile.
    #[must_use]
    pub const fn minimum_slot_count(self) -> Option<usize> {
        match self {
            Self::Base => Some(VescIfAbi::BASE_SLOT_COUNT),
            Self::Firmware605 => Some(VescIfAbi::FIRMWARE_605_SLOT_COUNT),
            Self::Firmware606 => Some(VescIfAbi::FIELD_COUNT),
            Self::UnknownCompatible => None,
        }
    }
}

/// Error returned when a required or optional ABI capability is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiError {
    /// A minimum-ABI capability was absent.
    MissingRequired {
        /// Human-readable capability name.
        capability: &'static str,
        /// Manifest slot required by the capability.
        slot: VescIfSlot,
    },
    /// An optional capability was absent.
    Unsupported {
        /// Human-readable capability name.
        capability: &'static str,
        /// Manifest slot probed by the capability.
        slot: VescIfSlot,
    },
}

/// Safe subsystem names exposed by a concrete firmware table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSubsystem {
    /// Complete motor telemetry and command support.
    Motor,
    /// Controller input and output-safety support.
    Inputs,
    /// Controller-area network support.
    Can,
    /// Non-volatile memory support.
    Nvm,
    /// FOC audio support.
    Audio,
    /// UART support.
    Uart,
    /// Firmware settings support.
    Settings,
    /// Firmware IMU and attitude-estimation support.
    Imu,
    /// Explicitly unsafe open-loop FOC control support.
    AdvancedFoc,
}

/// A capability-bearing handle for one available subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfCapability {
    subsystem: VescIfSubsystem,
}

impl VescIfCapability {
    /// Return the subsystem represented by this handle.
    pub const fn subsystem(self) -> VescIfSubsystem {
        self.subsystem
    }
}

/// Named subsystem capabilities derived from observed table presence.
///
/// This keeps callers from assembling a public bag of booleans or naming raw
/// slots. The observed pointers remain authoritative; the revision is only a
/// descriptive summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfCapabilities {
    presence: VescIfPresence,
}

impl VescIfCapabilities {
    /// Construct named capabilities from one observed table snapshot.
    pub const fn new(presence: VescIfPresence) -> Self {
        Self { presence }
    }

    /// Return the underlying observed presence snapshot for diagnostics.
    pub const fn presence(self) -> VescIfPresence {
        self.presence
    }

    /// Return the descriptive revision inferred from observed pointers.
    pub fn revision(self) -> Stm32AbiRevision {
        self.presence.revision()
    }

    /// Probe the complete motor telemetry and command surface.
    pub fn motor(self) -> Result<VescIfCapability, AbiError> {
        let checks = [
            VescIfAbi::MC_GET_RPM,
            VescIfAbi::MC_GET_SPEED,
            VescIfAbi::MC_GET_TOT_CURRENT_FILTERED,
            VescIfAbi::MC_GET_TOT_CURRENT,
            VescIfAbi::MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED,
            VescIfAbi::MC_GET_TOT_CURRENT_DIRECTIONAL,
            VescIfAbi::GET_CFG_FLOAT,
            VescIfAbi::GET_CFG_INT,
            VescIfAbi::MC_GET_TOT_CURRENT_IN_FILTERED,
            VescIfAbi::MC_GET_TOT_CURRENT_IN,
            VescIfAbi::MC_STAT_POWER_AVG,
            VescIfAbi::MC_STAT_POWER_MAX,
            VescIfAbi::MC_STAT_SPEED_AVG,
            VescIfAbi::MC_STAT_SPEED_MAX,
            VescIfAbi::MC_STAT_CURRENT_AVG,
            VescIfAbi::MC_STAT_CURRENT_MAX,
            VescIfAbi::MC_STAT_TEMP_MOSFET_AVG,
            VescIfAbi::MC_STAT_TEMP_MOSFET_MAX,
            VescIfAbi::MC_STAT_TEMP_MOTOR_AVG,
            VescIfAbi::MC_STAT_TEMP_MOTOR_MAX,
            VescIfAbi::MC_STAT_COUNT_TIME,
            VescIfAbi::MC_GET_DUTY_CYCLE_NOW,
            VescIfAbi::MC_GET_PID_POS_SET,
            VescIfAbi::MC_GET_PID_POS_NOW,
            VescIfAbi::MC_TEMP_FET_FILTERED,
            VescIfAbi::MC_TEMP_MOTOR_FILTERED,
            VescIfAbi::MC_GET_ODOMETER,
            VescIfAbi::MC_GET_AMP_HOURS,
            VescIfAbi::MC_GET_AMP_HOURS_CHARGED,
            VescIfAbi::MC_GET_WATT_HOURS,
            VescIfAbi::MC_GET_WATT_HOURS_CHARGED,
            VescIfAbi::MC_GET_BATTERY_LEVEL,
            VescIfAbi::MC_GET_FAULT,
            VescIfAbi::MC_FAULT_TO_STRING,
            VescIfAbi::MC_GET_INPUT_VOLTAGE_FILTERED,
            VescIfAbi::MC_GET_TACHOMETER_VALUE,
            VescIfAbi::MC_GET_TACHOMETER_ABS_VALUE,
            VescIfAbi::MC_GET_SAMPLING_FREQUENCY_NOW,
            VescIfAbi::MC_GET_DISTANCE_ABS,
            VescIfAbi::MC_GET_DISTANCE,
            VescIfAbi::MC_DCCAL_DONE,
            VescIfAbi::MC_GET_MOTOR_THREAD,
            VescIfAbi::MC_SELECT_MOTOR_THREAD,
            VescIfAbi::TIMEOUT_RESET,
            VescIfAbi::MC_SET_CURRENT_OFF_DELAY,
            VescIfAbi::MC_SET_CURRENT,
            VescIfAbi::MC_SET_CURRENT_REL,
            VescIfAbi::MC_SET_BRAKE_CURRENT_REL,
            VescIfAbi::MC_SET_PID_SPEED,
            VescIfAbi::MC_SET_PID_POS,
            VescIfAbi::MC_SET_DUTY,
            VescIfAbi::MC_SET_DUTY_NORAMP,
            VescIfAbi::MC_SET_BRAKE_CURRENT,
            VescIfAbi::MC_SET_HANDBRAKE,
            VescIfAbi::MC_SET_HANDBRAKE_REL,
            VescIfAbi::MC_SET_PWM_CALLBACK,
            VescIfAbi::MC_STAT_RESET,
            VescIfAbi::MC_UPDATE_PID_POS_OFFSET,
            VescIfAbi::MC_SET_ODOMETER,
            VescIfAbi::MC_SET_TACHOMETER_VALUE,
            VescIfAbi::MC_RELEASE_MOTOR,
            VescIfAbi::MC_WAIT_FOR_MOTOR_RELEASE,
        ];
        let mut index = 0;
        while index < checks.len() {
            if let Err(error) = self.presence.require("motor", checks[index]) {
                return Err(error);
            }
            index += 1;
        }
        Ok(VescIfCapability {
            subsystem: VescIfSubsystem::Motor,
        })
    }

    /// Probe controller input support as an optional subsystem.
    pub fn inputs(self) -> Result<VescIfCapability, AbiError> {
        match self.inputs_slots(false) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Inputs,
            }),
            Err(error) => Err(error),
        }
    }

    /// Require controller input support for a constructor that cannot operate without it.
    pub fn require_inputs(self) -> Result<VescIfCapability, AbiError> {
        match self.inputs_slots(true) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Inputs,
            }),
            Err(error) => Err(error),
        }
    }

    fn inputs_slots(self, required: bool) -> Result<(), AbiError> {
        let checks = [
            VescIfAbi::GET_REMOTE_STATE,
            VescIfAbi::TIMEOUT_RESET,
            VescIfAbi::TIMEOUT_HAS_TIMEOUT,
            VescIfAbi::TIMEOUT_SECS_SINCE_UPDATE,
        ];
        let mut index = 0;
        while index < checks.len() {
            let result = if required {
                self.presence.require("inputs", checks[index])
            } else {
                self.presence.optional("inputs", checks[index])
            };
            if let Err(error) = result {
                return Err(error);
            }
            index += 1;
        }
        Ok(())
    }

    /// Probe CAN support as an optional subsystem.
    pub fn can(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Can, "CAN", VescIfAbi::CAN_TRANSMIT_SID)
    }

    /// Require CAN support for a constructor that cannot operate without it.
    pub fn require_can(self) -> Result<VescIfCapability, AbiError> {
        self.required(VescIfSubsystem::Can, "CAN", VescIfAbi::CAN_TRANSMIT_SID)
    }

    /// Probe NVM support as an optional subsystem.
    pub fn nvm(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Nvm, "NVM", VescIfAbi::READ_NVM)
    }

    /// Probe FOC audio support as an optional subsystem.
    pub fn audio(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Audio, "FOC audio", VescIfAbi::FOC_BEEP)
    }

    /// Probe UART support as an optional subsystem.
    pub fn uart(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Uart, "UART", VescIfAbi::UART_START)
    }

    /// Require settings support for a constructor that needs configuration access.
    pub fn require_settings(self) -> Result<VescIfCapability, AbiError> {
        match self.settings_slots(true) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Settings,
            }),
            Err(error) => Err(error),
        }
    }

    /// Probe settings support when a package can operate without configuration access.
    pub fn settings(self) -> Result<VescIfCapability, AbiError> {
        match self.settings_slots(false) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Settings,
            }),
            Err(error) => Err(error),
        }
    }

    /// Probe the complete firmware IMU surface as an optional subsystem.
    pub fn imu(self) -> Result<VescIfCapability, AbiError> {
        match self.imu_slots() {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Imu,
            }),
            Err(error) => Err(error),
        }
    }

    /// Probe the complete explicitly unsafe open-loop FOC surface.
    pub fn advanced_foc(self) -> Result<VescIfCapability, AbiError> {
        let checks = [
            VescIfAbi::FOC_SET_OPENLOOP_CURRENT,
            VescIfAbi::FOC_SET_OPENLOOP_PHASE,
            VescIfAbi::FOC_SET_OPENLOOP_DUTY,
            VescIfAbi::FOC_SET_OPENLOOP_DUTY_PHASE,
        ];
        let mut index = 0;
        while index < checks.len() {
            if let Err(error) = self.presence.optional("advanced FOC", checks[index]) {
                return Err(error);
            }
            index += 1;
        }
        Ok(VescIfCapability {
            subsystem: VescIfSubsystem::AdvancedFoc,
        })
    }

    fn imu_slots(self) -> Result<(), AbiError> {
        let checks = [
            VescIfAbi::IMU_STARTUP_DONE,
            VescIfAbi::IMU_GET_ROLL,
            VescIfAbi::IMU_GET_PITCH,
            VescIfAbi::IMU_GET_YAW,
            VescIfAbi::IMU_GET_RPY,
            VescIfAbi::IMU_GET_ACCEL,
            VescIfAbi::IMU_GET_GYRO,
            VescIfAbi::IMU_GET_MAG,
            VescIfAbi::IMU_DEROTATE,
            VescIfAbi::IMU_GET_ACCEL_DEROTATED,
            VescIfAbi::IMU_GET_GYRO_DEROTATED,
            VescIfAbi::IMU_GET_QUATERNIONS,
            VescIfAbi::IMU_GET_CALIBRATION,
            VescIfAbi::IMU_SET_YAW,
            VescIfAbi::IMU_SET_READ_CALLBACK,
        ];
        let mut index = 0;
        while index < checks.len() {
            let result = self.presence.optional("IMU", checks[index]);
            if let Err(error) = result {
                return Err(error);
            }
            index += 1;
        }
        Ok(())
    }

    fn settings_slots(self, required: bool) -> Result<(), AbiError> {
        let checks = [
            VescIfAbi::GET_CFG_FLOAT,
            VescIfAbi::GET_CFG_INT,
            VescIfAbi::SET_CFG_FLOAT,
            VescIfAbi::SET_CFG_INT,
            VescIfAbi::STORE_CFG,
        ];
        let mut index = 0;
        while index < checks.len() {
            let result = if required {
                self.presence.require("settings", checks[index])
            } else {
                self.presence.optional("settings", checks[index])
            };
            if let Err(error) = result {
                return Err(error);
            }
            index += 1;
        }
        Ok(())
    }

    fn optional(
        self,
        subsystem: VescIfSubsystem,
        capability: &'static str,
        slot: VescIfSlot,
    ) -> Result<VescIfCapability, AbiError> {
        match self.presence.optional(capability, slot) {
            Ok(()) => Ok(VescIfCapability { subsystem }),
            Err(error) => Err(error),
        }
    }

    fn required(
        self,
        subsystem: VescIfSubsystem,
        capability: &'static str,
        slot: VescIfSlot,
    ) -> Result<VescIfCapability, AbiError> {
        match self.presence.require(capability, slot) {
            Ok(()) => Ok(VescIfCapability { subsystem }),
            Err(error) => Err(error),
        }
    }
}

impl AbiError {
    /// Return the human-readable capability name carried by this error.
    pub const fn capability(self) -> &'static str {
        match self {
            Self::MissingRequired { capability, .. } | Self::Unsupported { capability, .. } => {
                capability
            }
        }
    }

    /// Return the manifest slot that was required or probed.
    pub const fn slot(self) -> VescIfSlot {
        match self {
            Self::MissingRequired { slot, .. } | Self::Unsupported { slot, .. } => slot,
        }
    }
}

impl VescIfSlot {
    /// Create a named slot at the given 32-bit byte offset.
    #[must_use]
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self::with_header_line(name, offset, 0)
    }

    /// Create a named slot with its source line in the pinned ABI header.
    pub const fn with_header_line(name: &'static str, offset: usize, header_line: usize) -> Self {
        Self {
            name,
            offset,
            header_line,
        }
    }

    /// Return the firmware symbol name for this slot.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Return the 32-bit firmware byte offset for this slot.
    #[must_use]
    pub const fn vesc32_byte_offset(self) -> usize {
        self.offset
    }

    /// Return the 1-based line containing this slot in [`VescIfAbi::SOURCE_HEADER`].
    ///
    /// Hand-constructed slots created with [`Self::new`] have no source anchor and return zero.
    pub const fn header_line(self) -> usize {
        self.header_line
    }

    /// Return the minimum ordered ABI profile containing this slot.
    ///
    /// This is descriptive table-shape metadata. Actual slot presence remains authoritative.
    pub const fn minimum_revision(self) -> Stm32AbiRevision {
        let index = self.slot_index();
        if index < VescIfAbi::BASE_SLOT_COUNT {
            Stm32AbiRevision::Base
        } else if index < VescIfAbi::FIRMWARE_605_SLOT_COUNT {
            Stm32AbiRevision::Firmware605
        } else {
            Stm32AbiRevision::Firmware606
        }
    }

    /// Return the corresponding host byte offset for a pointer-sized table.
    pub const fn host_byte_offset(self, pointer_size: usize) -> usize {
        self.slot_index() * pointer_size
    }

    /// Return the slot index in the 32-bit function table.
    #[must_use]
    pub const fn slot_index(self) -> usize {
        self.offset / 4
    }
}

/// ABI anchor and slot metadata for the VESC firmware function table.
pub struct VescIfAbi;

macro_rules! define_vesc_if_abi {
    ($($const_name:ident => $slot_name:literal, $slot_offset:expr, $header_line:expr),+ $(,)?) => {
        impl VescIfAbi {
            /// Base address of the firmware function table on VESC targets.
            pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
            /// Number of entries in the pinned upstream `vesc_c_if` table.
            pub const FIELD_COUNT: usize = c_vesc_if::FIELD_COUNT;
            /// Number of callable function-pointer entries in the manifest.
            pub const CALLABLE_SLOT_COUNT: usize = c_vesc_if::CALLABLE_SLOT_COUNT;
            /// Number of generated raw resolvers for callable entries.
            pub const RAW_SHIM_COUNT: usize = c_vesc_if::RAW_SHIM_COUNT;
            /// Callable slots in the same order as the generated raw resolvers.
            pub const RAW_SHIM_SLOTS: [VescIfSlot; Self::RAW_SHIM_COUNT] =
                c_vesc_if::RAW_SHIM_SLOTS;
            /// Bindgen-rendered signatures in the same order as the generated raw resolvers.
            pub const RAW_SHIM_SIGNATURES: [&'static str; Self::RAW_SHIM_COUNT] =
                c_vesc_if::RAW_SHIM_SIGNATURES;
            /// First slot added by the firmware 6.05 interface extension.
            pub const BASE_SLOT_COUNT: usize = c_vesc_if::FIRMWARE_605_FIRST_SLOT;
            /// First slot added by the firmware 6.06 interface extension.
            pub const FIRMWARE_605_SLOT_COUNT: usize = c_vesc_if::FIRMWARE_606_FIRST_SLOT;
            /// Complete ordered manifest of every entry in the pinned `VESC_IF` table.
            ///
            /// `ALL_SLOTS` is the authoritative layout inventory and is generated directly
            /// from the pinned header. The named constants below remain compatibility aliases.
            pub const ALL_SLOTS: [VescIfSlot; Self::FIELD_COUNT] = c_vesc_if::ALL_SLOTS;
            /// Complete kind and offset metadata for every ABI slot.
            pub const ALL_ENTRIES: [VescIfManifestEntry; Self::FIELD_COUNT] =
                c_vesc_if::ALL_ENTRIES;
            /// Repository containing the pinned ABI header.
            pub const SOURCE_REPOSITORY: &str = c_vesc_if::HEADER_REPO;
            /// Commit containing the pinned ABI header.
            pub const SOURCE_COMMIT: &str = c_vesc_if::HEADER_COMMIT;
            /// Workspace-relative path to the pinned ABI header.
            pub const SOURCE_HEADER: &str = c_vesc_if::HEADER_PATH;
            /// Number of entries in the complete generated manifest.
            #[deprecated(note = "use FIELD_COUNT; the manifest contains every ABI entry")]
            pub const USED_SLOT_COUNT: usize = Self::FIELD_COUNT;

            $(
                #[doc = concat!("Slot for `", $slot_name, "`.")]
                pub const $const_name: VescIfSlot = VescIfSlot::with_header_line(
                    $slot_name,
                    $slot_offset,
                    $header_line,
                );
            )+

            /// Complete slot projection retained under the compatibility name.
            #[deprecated(note = "use ALL_SLOTS; this compatibility alias is not a subset")]
            pub const USED_SLOTS: [VescIfSlot; Self::FIELD_COUNT] = Self::ALL_SLOTS;
        }
    };
}

c_vesc_if::define_vesc_if_manifest_constants!(define_vesc_if_abi);
