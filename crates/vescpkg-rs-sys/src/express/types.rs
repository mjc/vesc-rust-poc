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

/// Native-library image kind accepted by an Express target's loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressNativeLoadKind {
    /// Execute the library in place from flash.
    Xip,
    /// Copy and relocate the library into executable/data RAM.
    Relocatable,
}

impl ExpressTarget {
    /// Parse an ESP-IDF target preprocessor define.
    pub fn from_sdkconfig_define(define: &str) -> Option<Self> {
        match define {
            "CONFIG_IDF_TARGET_ESP32C3" => Some(Self::Esp32C3),
            "CONFIG_IDF_TARGET_ESP32S3" => Some(Self::Esp32S3),
            "CONFIG_IDF_TARGET_ESP32C6" => Some(Self::Esp32C6),
            "CONFIG_IDF_TARGET_ESP32P4" => Some(Self::Esp32P4),
            _ => None,
        }
    }

    /// Return the ESP-IDF target name used by Express firmware builds.
    pub const fn target_name(self) -> &'static str {
        match self {
            Self::Esp32C3 => "esp32c3",
            Self::Esp32S3 => "esp32s3",
            Self::Esp32C6 => "esp32c6",
            Self::Esp32P4 => "esp32p4",
        }
    }

    /// Parse the canonical ESP-IDF target name.
    pub fn from_target_name(name: &str) -> Option<Self> {
        match name {
            "esp32c3" => Some(Self::Esp32C3),
            "esp32s3" => Some(Self::Esp32S3),
            "esp32c6" => Some(Self::Esp32C6),
            "esp32p4" => Some(Self::Esp32P4),
            _ => None,
        }
    }

    /// Return the `sdkconfig.h` preprocessor define required for this target.
    pub const fn sdkconfig_define(self) -> &'static str {
        match self {
            Self::Esp32C3 => "CONFIG_IDF_TARGET_ESP32C3",
            Self::Esp32S3 => "CONFIG_IDF_TARGET_ESP32S3",
            Self::Esp32C6 => "CONFIG_IDF_TARGET_ESP32C6",
            Self::Esp32P4 => "CONFIG_IDF_TARGET_ESP32P4",
        }
    }

    /// Return the native-library image kind supported by this target.
    ///
    /// The pinned Express loader only implements the relocatable container
    /// path for ESP32-S3; other mapped targets accept the XIP image form.
    pub const fn native_load_kind(self) -> ExpressNativeLoadKind {
        match self {
            Self::Esp32S3 => ExpressNativeLoadKind::Relocatable,
            Self::Esp32C3 | Self::Esp32C6 | Self::Esp32P4 => ExpressNativeLoadKind::Xip,
        }
    }

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

/// Firmware-owned flattened LispBM value storage.
#[derive(Debug)]
#[repr(C)]
pub struct ExpressFlatValueRaw {
    pub(crate) buf: *mut u8,
    pub(crate) buf_size: u32,
    pub(crate) buf_pos: u32,
}

impl ExpressFlatValueRaw {
    pub(crate) const fn empty() -> Self {
        Self {
            buf: core::ptr::null_mut(),
            buf_size: 0,
            buf_pos: 0,
        }
    }
}
