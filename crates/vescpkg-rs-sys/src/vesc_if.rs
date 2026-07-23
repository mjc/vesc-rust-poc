//! Documented VESC firmware function-table slots used by Rust packages.

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
    pub const fn empty() -> Self {
        Self {
            bits: [0; PRESENCE_WORD_COUNT],
        }
    }

    /// Inspect pointer-sized table words, preserving holes and scalar entries.
    pub fn from_words(words: &[usize]) -> Self {
        let mut presence = Self::empty();
        for (index, entry) in VescIfAbi::ALL_ENTRIES.iter().enumerate() {
            if index >= words.len() || (entry.is_callable() && words[index] == 0) {
                continue;
            }
            presence.set(index);
        }
        presence
    }

    /// Return whether a slot was observed as present.
    pub const fn contains(self, slot: VescIfSlot) -> bool {
        self.contains_index(slot.slot_index())
    }

    /// Return whether a slot index was observed as present.
    pub const fn contains_index(self, index: usize) -> bool {
        index < VescIfAbi::FIELD_COUNT && (self.bits[index / 64] & (1_u64 << (index % 64))) != 0
    }

    /// Check a required capability and preserve the slot identity in the error.
    pub const fn require(self, capability: &'static str, slot: VescIfSlot) -> Result<(), AbiError> {
        if self.contains(slot) {
            Ok(())
        } else {
            Err(AbiError::MissingRequired { capability, slot })
        }
    }

    /// Check an optional capability without exposing raw table access to callers.
    pub const fn optional(
        self,
        capability: &'static str,
        slot: VescIfSlot,
    ) -> Result<(), AbiError> {
        if self.contains(slot) {
            Ok(())
        } else {
            Err(AbiError::Unsupported { capability, slot })
        }
    }

    /// Return whether every callable slot in a revision profile is present.
    pub fn supports_revision(self, revision: Stm32AbiRevision) -> bool {
        let Some(slot_count) = revision.minimum_slot_count() else {
            return false;
        };
        VescIfAbi::ALL_ENTRIES[..slot_count]
            .iter()
            .enumerate()
            .all(|(index, entry)| !entry.is_callable() || self.contains_index(index))
    }

    /// Infer the strongest descriptive profile supported by observed presence.
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

    pub(crate) const fn set(&mut self, index: usize) {
        self.bits[index / 64] |= 1_u64 << (index % 64);
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

    /// Probe controller input support as an optional subsystem.
    pub const fn inputs(self) -> Result<VescIfCapability, AbiError> {
        match self.inputs_slots(false) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Inputs,
            }),
            Err(error) => Err(error),
        }
    }

    /// Require controller input support for a constructor that cannot operate without it.
    pub const fn require_inputs(self) -> Result<VescIfCapability, AbiError> {
        match self.inputs_slots(true) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Inputs,
            }),
            Err(error) => Err(error),
        }
    }

    const fn inputs_slots(self, required: bool) -> Result<(), AbiError> {
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
    pub const fn can(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Can, "CAN", VescIfAbi::CAN_TRANSMIT_SID)
    }

    /// Require CAN support for a constructor that cannot operate without it.
    pub const fn require_can(self) -> Result<VescIfCapability, AbiError> {
        self.required(VescIfSubsystem::Can, "CAN", VescIfAbi::CAN_TRANSMIT_SID)
    }

    /// Probe NVM support as an optional subsystem.
    pub const fn nvm(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Nvm, "NVM", VescIfAbi::READ_NVM)
    }

    /// Probe FOC audio support as an optional subsystem.
    pub const fn audio(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Audio, "FOC audio", VescIfAbi::FOC_BEEP)
    }

    /// Probe UART support as an optional subsystem.
    pub const fn uart(self) -> Result<VescIfCapability, AbiError> {
        self.optional(VescIfSubsystem::Uart, "UART", VescIfAbi::UART_START)
    }

    /// Require settings support for a constructor that needs configuration access.
    pub const fn require_settings(self) -> Result<VescIfCapability, AbiError> {
        match self.settings_slots(true) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Settings,
            }),
            Err(error) => Err(error),
        }
    }

    /// Probe settings support when a package can operate without configuration access.
    pub const fn settings(self) -> Result<VescIfCapability, AbiError> {
        match self.settings_slots(false) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Settings,
            }),
            Err(error) => Err(error),
        }
    }

    /// Probe the complete firmware IMU surface as an optional subsystem.
    pub const fn imu(self) -> Result<VescIfCapability, AbiError> {
        match self.imu_slots(false) {
            Ok(()) => Ok(VescIfCapability {
                subsystem: VescIfSubsystem::Imu,
            }),
            Err(error) => Err(error),
        }
    }

    const fn imu_slots(self, required: bool) -> Result<(), AbiError> {
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
        ];
        let mut index = 0;
        while index < checks.len() {
            let result = if required {
                self.presence.require("IMU", checks[index])
            } else {
                self.presence.optional("IMU", checks[index])
            };
            if let Err(error) = result {
                return Err(error);
            }
            index += 1;
        }
        Ok(())
    }

    const fn settings_slots(self, required: bool) -> Result<(), AbiError> {
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

    const fn optional(
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

    const fn required(
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
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Return the 32-bit firmware byte offset for this slot.
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
