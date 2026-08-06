//! Float Out Boy hardware LED configuration.
//!
//! C map: defaults mirror the hardware LED settings in
//! `third_party/float-out-boy/src/conf/settings.xml:3560-3863`. Float Out Boy treats the
//! mode as flags when enabling internal LEDs and the external LCM at
//! `third_party/float-out-boy/src/leds.c:795-830` and
//! `third_party/float-out-boy/src/lcm.c:27-28`.

use crate::{
    FloatOutBoyLedMode, FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedStripConfig,
};

/// Float Out Boy hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareLedsConfig {
    /// Enabled LED surfaces.
    pub mode: FloatOutBoyLedMode,
    /// STM32 output pin.
    pub pin: FloatOutBoyLedPin,
    /// Output pin electrical configuration.
    pub pin_config: FloatOutBoyLedPinConfig,
    /// Status-strip wiring.
    pub status: FloatOutBoyLedStripConfig,
    /// Front-strip wiring.
    pub front: FloatOutBoyLedStripConfig,
    /// Rear-strip wiring.
    pub rear: FloatOutBoyLedStripConfig,
}

#[cfg(any(test, feature = "test-support"))]
mod test_support;

#[cfg(test)]
mod tests;
