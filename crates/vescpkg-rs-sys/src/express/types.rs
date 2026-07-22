//! Platform-neutral constants for the VESC Express native-library ABI.
//!
//! These values are mapped from the pinned `main/c_libs/vesc_c_if.h` header in
//! `vedderb/vesc_express`. Express uses a 32-bit firmware table word for both
//! data slots and function-pointer slots, even when this crate is inspected on
//! a wider host.

/// Breaking layout version at the first Express interface slot.
pub const EXPRESS_C_IF_VERSION: u32 = 1;
/// Relocatable native-library container magic.
pub const EXPRESS_NATIVE_LIB_MAGIC: u32 = 0xCAFE_BABE;
/// Relocatable native-library container magic used by the relocation format.
pub const EXPRESS_NATIVE_LIB_RELOC_MAGIC: u32 = 0xCAFE_BABF;
/// Express system tick rate from the pinned header.
pub const EXPRESS_SYSTEM_TICK_RATE_HZ: u32 = 1_000;
/// Number of words in the pinned v1 Express interface table.
pub const EXPRESS_IF_SLOT_COUNT: usize = 80;
/// Express interface table size on its 32-bit targets.
pub const EXPRESS_IF_TABLE_BYTES: usize = EXPRESS_IF_SLOT_COUNT * 4;

/// Express firmware target with a pinned interface-table address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressTarget {
    /// ESP32-C3 firmware target.
    Esp32C3,
    /// ESP32-S3 firmware target.
    Esp32S3,
    /// ESP32-C6 firmware target.
    Esp32C6,
    /// ESP32-P4 firmware target.
    Esp32P4,
}

impl ExpressTarget {
    /// Return the firmware address of the Express interface table.
    pub const fn interface_address(self) -> u32 {
        match self {
            Self::Esp32C3 => 0x3FCD_BE00,
            Self::Esp32S3 => 0x3FCE_8800,
            Self::Esp32C6 => 0x4087_B800,
            Self::Esp32P4 => 0x4FF3_A000,
        }
    }
}

/// A raw 32-bit word from the Express interface table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExpressWord(u32);

impl ExpressWord {
    /// Create a word from the firmware representation.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the firmware representation.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A non-null 32-bit target address held in an Express table slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExpressAddress(u32);

impl ExpressAddress {
    /// Create an address from the 32-bit firmware representation.
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the target address without treating it as a host pointer.
    pub const fn get(self) -> u32 {
        self.0
    }
}
