//! Refloat hardware LED mode.
//!
//! C map: `third_party/refloat/src/conf/datatypes.h:36-60`.

/// Refloat hardware LED mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedMode {
    /// LEDs are disabled.
    Off,
    /// Internal/status LEDs are enabled.
    Internal,
    /// External LCM LEDs are enabled.
    External,
    /// Internal/status and external LCM LEDs are enabled.
    Both,
}

impl RefloatLedMode {
    /// Return the Refloat `v1.2.1` hardware LED mode ID.
    ///
    /// C map: `third_party/refloat/src/conf/datatypes.h:36-60`.
    pub const fn id(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Internal => 0x1,
            Self::External => 0x2,
            Self::Both => 0x3,
        }
    }

    pub(crate) const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    pub(crate) const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}
