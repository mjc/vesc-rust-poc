//! Float Out Boy hardware LED configuration.
//!
//! C map: defaults mirror the hardware LED settings in
//! `third_party/float-out-boy/src/conf/settings.xml:3560-3863`. Float Out Boy treats the
//! mode as flags when enabling internal LEDs and the external LCM at
//! `third_party/float-out-boy/src/leds.c:795-830` and
//! `third_party/float-out-boy/src/lcm.c:27-28`.

use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedStripConfig};

use super::mode::FloatOutBoyLedMode;

/// Float Out Boy hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareLedsConfig {
    pub(crate) mode: FloatOutBoyLedMode,
    pub(crate) pin: FloatOutBoyLedPin,
    pub(crate) pin_config: FloatOutBoyLedPinConfig,
    pub(crate) status: FloatOutBoyLedStripConfig,
    pub(crate) front: FloatOutBoyLedStripConfig,
    pub(crate) rear: FloatOutBoyLedStripConfig,
}

#[cfg(test)]
#[path = "hardware/tests/api.rs"]
mod test_api;

#[cfg(test)]
mod tests;
