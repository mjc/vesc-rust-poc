//! CAN bus semantic tokens.

/// VESC CAN controller ID.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanControllerId(u8);

impl CanControllerId {
    /// Wrap a raw CAN controller ID.
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    /// Encode the controller ID for the CAN boundary.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// Standard 11-bit CAN identifier.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanStandardId(u16);

impl CanStandardId {
    /// Highest standard CAN identifier.
    pub const MAX: u16 = 0x07ff;

    /// Create a checked standard CAN identifier.
    ///
    /// # Errors
    ///
    /// Returns [`CanStandardIdError`] when `id` exceeds 11 bits.
    pub const fn try_new(id: u16) -> Result<Self, CanStandardIdError> {
        if id <= Self::MAX {
            Ok(Self(id))
        } else {
            Err(CanStandardIdError { value: id })
        }
    }

    /// Encode the standard CAN identifier for the protocol boundary.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// Error returned when a standard CAN ID is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStandardIdError {
    value: u16,
}

impl core::fmt::Display for CanStandardIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "standard CAN ID {} exceeds 0x7ff", self.value)
    }
}

impl core::error::Error for CanStandardIdError {}

impl CanStandardIdError {
    /// Return the rejected ID.
    pub const fn value(self) -> u16 {
        self.value
    }
}

/// Extended 29-bit CAN identifier.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanExtendedId(u32);

impl CanExtendedId {
    /// Highest extended CAN identifier.
    pub const MAX: u32 = 0x1fff_ffff;

    /// Create a checked extended CAN identifier.
    ///
    /// # Errors
    ///
    /// Returns [`CanExtendedIdError`] when `id` exceeds 29 bits.
    pub const fn try_new(id: u32) -> Result<Self, CanExtendedIdError> {
        if id <= Self::MAX {
            Ok(Self(id))
        } else {
            Err(CanExtendedIdError { value: id })
        }
    }

    /// Encode the extended CAN identifier for the protocol boundary.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Error returned when an extended CAN ID is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExtendedIdError {
    value: u32,
}

impl core::fmt::Display for CanExtendedIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "extended CAN ID {} exceeds 0x1fff_ffff", self.value)
    }
}

impl core::error::Error for CanExtendedIdError {}

/// Classic CAN payload length in bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanPayloadLen(u8);

impl CanPayloadLen {
    /// Highest classic CAN payload length.
    pub const MAX: u8 = 8;

    /// Create a checked CAN payload length.
    ///
    /// # Errors
    ///
    /// Returns [`CanPayloadLenError`] when `len` exceeds eight bytes.
    pub const fn try_new(len: u8) -> Result<Self, CanPayloadLenError> {
        if len <= Self::MAX {
            Ok(Self(len))
        } else {
            Err(CanPayloadLenError { value: len })
        }
    }

    /// Encode the payload length for the protocol boundary.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// Error returned when a CAN payload length is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanPayloadLenError {
    value: u8,
}

impl core::fmt::Display for CanPayloadLenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "classic CAN payload length {} exceeds 8 bytes",
            self.value
        )
    }
}

impl core::error::Error for CanPayloadLenError {}

impl CanPayloadLenError {
    /// Return the rejected length.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.value
    }
}

impl CanExtendedIdError {
    /// Return the rejected ID.
    pub const fn value(self) -> u32 {
        self.value
    }
}
