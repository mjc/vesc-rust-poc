//! CAN bus semantic tokens.

/// VESC CAN controller ID.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanControllerId(u8);

impl CanControllerId {
    /// Wrap a raw CAN controller ID.
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    /// Encode the controller ID for the CAN boundary.
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
    pub const fn try_new(id: u16) -> Result<Self, CanStandardIdError> {
        if id <= Self::MAX {
            Ok(Self(id))
        } else {
            Err(CanStandardIdError { value: id })
        }
    }

    /// Encode the standard CAN identifier for the protocol boundary.
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// Error returned when a standard CAN ID is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStandardIdError {
    value: u16,
}

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
    pub const fn try_new(id: u32) -> Result<Self, CanExtendedIdError> {
        if id <= Self::MAX {
            Ok(Self(id))
        } else {
            Err(CanExtendedIdError { value: id })
        }
    }

    /// Encode the extended CAN identifier for the protocol boundary.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Error returned when an extended CAN ID is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExtendedIdError {
    value: u32,
}

/// Classic CAN payload length in bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CanPayloadLen(u8);

impl CanPayloadLen {
    /// Highest classic CAN payload length.
    pub const MAX: u8 = 8;

    /// Create a checked CAN payload length.
    pub const fn try_new(len: u8) -> Result<Self, CanPayloadLenError> {
        if len <= Self::MAX {
            Ok(Self(len))
        } else {
            Err(CanPayloadLenError { value: len })
        }
    }

    /// Encode the payload length for the protocol boundary.
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// Error returned when a CAN payload length is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanPayloadLenError {
    value: u8,
}

impl CanPayloadLenError {
    /// Return the rejected length.
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
