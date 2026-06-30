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

    /// Explicitly extract the raw controller ID.
    pub const fn get(self) -> u8 {
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

    /// Explicitly extract the raw standard CAN identifier.
    pub const fn get(self) -> u16 {
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

    /// Explicitly extract the raw extended CAN identifier.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Error returned when an extended CAN ID is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExtendedIdError {
    value: u32,
}

impl CanExtendedIdError {
    /// Return the rejected ID.
    pub const fn value(self) -> u32 {
        self.value
    }
}
