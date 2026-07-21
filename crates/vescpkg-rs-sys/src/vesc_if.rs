//! Documented VESC firmware function-table slots used by Rust packages.

use crate::{c_vesc_if, image::NativeAddress};

const PRESENCE_WORD_COUNT: usize = (c_vesc_if::FIELD_COUNT + 63) / 64;

/// One entry in the VESC firmware function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
}

/// Whether a manifest entry is a callable function pointer or a scalar ABI word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VescIfSlotKind {
    /// A function-pointer entry that can be called when present.
    Function,
    /// A scalar ABI word or other non-callable entry.
    Scalar,
}

/// Complete metadata for one entry in the pinned VESC firmware table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfManifestEntry {
    pub(crate) slot: VescIfSlot,
    pub(crate) header_line: usize,
    pub(crate) declaration: &'static str,
    pub(crate) kind: VescIfSlotKind,
}

impl VescIfManifestEntry {
    /// Return the slot identity and 32-bit offset.
    pub const fn slot(self) -> VescIfSlot {
        self.slot
    }

    /// Return the source header line for this declaration.
    pub const fn header_line(self) -> usize {
        self.header_line
    }

    /// Return the normalized C declaration captured from the pinned header.
    pub const fn declaration(self) -> &'static str {
        self.declaration
    }

    /// Return whether the entry is callable through a function pointer.
    pub const fn kind(self) -> VescIfSlotKind {
        self.kind
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

    const fn set(&mut self, index: usize) {
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

impl VescIfSlot {
    /// Create a named slot at the given 32-bit byte offset.
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self { name, offset }
    }

    /// Return the firmware symbol name for this slot.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Return the 32-bit firmware byte offset for this slot.
    pub const fn vesc32_byte_offset(self) -> usize {
        self.offset
    }

    /// Return the slot index in the 32-bit function table.
    pub const fn slot_index(self) -> usize {
        self.offset / 4
    }

    /// Convert the 32-bit slot offset into a host-byte offset.
    pub const fn host_byte_offset(self, pointer_size: usize) -> usize {
        self.slot_index() * pointer_size
    }
}

/// ABI anchor and slot metadata for the VESC firmware function table.
pub struct VescIfAbi;

macro_rules! define_vesc_if_abi {
    ($($const_name:ident => $slot_name:ident),+ $(,)?) => {
        impl VescIfAbi {
            /// Base address of the firmware function table on VESC targets.
            pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
            /// Number of entries in the pinned upstream `vesc_c_if` table.
            pub const FIELD_COUNT: usize = c_vesc_if::FIELD_COUNT;
            /// First slot added by the firmware 6.05 interface extension.
            pub const BASE_SLOT_COUNT: usize = c_vesc_if::lbm_start_flatten::INDEX;
            /// First slot added by the firmware 6.06 interface extension.
            pub const FIRMWARE_605_SLOT_COUNT: usize = c_vesc_if::thread_set_priority::INDEX;
            /// Complete ordered manifest of every entry in the pinned `VESC_IF` table.
            ///
            /// The smaller `USED_SLOTS` list below remains the compatibility surface for
            /// wrappers that this crate has typed so far. `ALL_SLOTS` is the authoritative
            /// layout inventory and is generated directly from the pinned header.
            pub const ALL_SLOTS: [VescIfSlot; Self::FIELD_COUNT] = c_vesc_if::ALL_SLOTS;
            /// Complete declaration, source-line, and offset metadata for every ABI slot.
            pub const ALL_ENTRIES: [VescIfManifestEntry; Self::FIELD_COUNT] =
                c_vesc_if::ALL_ENTRIES;
            /// Repository containing the pinned ABI header.
            pub const SOURCE_REPOSITORY: &str = c_vesc_if::HEADER_REPO;
            /// Commit containing the pinned ABI header.
            pub const SOURCE_COMMIT: &str = c_vesc_if::HEADER_COMMIT;
            /// Workspace-relative path to the pinned ABI header.
            pub const SOURCE_HEADER: &str = c_vesc_if::HEADER_PATH;
            /// Number of `VESC_IF` slots that this crate currently relies on.
            pub const USED_SLOT_COUNT: usize = count_idents!($($slot_name),+);

            $(
                #[doc = concat!("Slot for `", stringify!($slot_name), "`.")]
                pub const $const_name: VescIfSlot = VescIfSlot::new(
                    c_vesc_if::$slot_name::NAME,
                    c_vesc_if::$slot_name::VESC32_BYTE_OFFSET,
                );
            )+

            /// The set of slots that this crate currently relies on.
            pub const USED_SLOTS: [VescIfSlot; Self::USED_SLOT_COUNT] = [
                $(Self::$const_name),+
            ];
        }
    };
}

vesc_if_used_slots!(define_vesc_if_abi);
