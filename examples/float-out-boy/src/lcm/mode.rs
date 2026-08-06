//! Float Out Boy hardware LED mode.
//!
//! C map: `third_party/float-out-boy/src/conf/datatypes.h:36-60`.

wire_enum! {
/// Float Out Boy hardware LED mode.
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
}

impl FloatOutBoyLedMode {
    pub(crate) const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    pub(crate) const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}
