//! Hardware LED mode owned by Float Out Boy.
//!
//! C map: `third_party/float-out-boy/src/conf/datatypes.h:36-60`.

vescpkg_rs::wire_enum! {
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
