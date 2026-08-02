//! Float Out Boy hardware LED mode.
//!
//! C map: `third_party/float-out-boy/src/conf/datatypes.h:36-60`.

/// Float Out Boy hardware LED mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedMode {
    /// LEDs are disabled.
    Off = 0,
    /// Internal/status LEDs are enabled.
    Internal = 1,
    /// External LCM LEDs are enabled.
    External = 2,
    /// Internal/status and external LCM LEDs are enabled.
    Both = 3,
}

impl FloatOutBoyLedMode {
    /// Return the Float Out Boy `v1.2.1` hardware LED mode ID.
    ///
    /// C map: `third_party/float-out-boy/src/conf/datatypes.h:36-60`.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }

    pub(crate) const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    pub(crate) const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}

impl TryFrom<u8> for FloatOutBoyLedMode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Off),
            1 => Ok(Self::Internal),
            2 => Ok(Self::External),
            3 => Ok(Self::Both),
            _ => Err(value),
        }
    }
}
