//! Documented VESC firmware function-table slots used by Rust packages.

use crate::{c_vesc_if, image::NativeAddress};

/// One entry in the VESC firmware function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
}

/// Complete metadata for one entry in the pinned VESC firmware table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfManifestEntry {
    pub(crate) slot: VescIfSlot,
    pub(crate) header_line: usize,
    pub(crate) declaration: &'static str,
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
