#![expect(missing_docs, reason = "compact FOB LED hardware construction API")]

use super::super::{FloatOutBoyLedColorOrder, FloatOutBoyLedStripOrder};
use super::{
    FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
    FloatOutBoyLedStripConfig,
};

impl FloatOutBoyLedMode {
    #[must_use]
    pub const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    #[must_use]
    pub const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}

impl FloatOutBoyHardwareLedsConfig {
    #[must_use]
    pub const fn new(mode: FloatOutBoyLedMode) -> Self {
        Self {
            mode,
            pin: FloatOutBoyLedPin::B7,
            pin_config: FloatOutBoyLedPinConfig::PullupTo5v,
            status: FloatOutBoyLedStripConfig {
                order: FloatOutBoyLedStripOrder::First,
                count: 10,
                color_order: FloatOutBoyLedColorOrder::Grb,
                reverse: false,
            },
            front: FloatOutBoyLedStripConfig {
                order: FloatOutBoyLedStripOrder::Second,
                count: 20,
                color_order: FloatOutBoyLedColorOrder::Grb,
                reverse: false,
            },
            rear: FloatOutBoyLedStripConfig {
                order: FloatOutBoyLedStripOrder::Third,
                count: 20,
                color_order: FloatOutBoyLedColorOrder::Grb,
                reverse: false,
            },
        }
    }

    #[must_use]
    pub const fn with_pin(mut self, pin: FloatOutBoyLedPin) -> Self {
        self.pin = pin;
        self
    }

    #[must_use]
    pub const fn with_pin_config(mut self, pin_config: FloatOutBoyLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    #[must_use]
    pub const fn with_status_strip(mut self, status: FloatOutBoyLedStripConfig) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub const fn with_front_strip(mut self, front: FloatOutBoyLedStripConfig) -> Self {
        self.front = front;
        self
    }

    #[must_use]
    pub const fn with_rear_strip(mut self, rear: FloatOutBoyLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    #[must_use]
    pub const fn mode(self) -> FloatOutBoyLedMode {
        self.mode
    }

    #[must_use]
    pub const fn pin(self) -> FloatOutBoyLedPin {
        self.pin
    }

    #[must_use]
    pub const fn pin_config(self) -> FloatOutBoyLedPinConfig {
        self.pin_config
    }

    #[must_use]
    pub const fn status_strip(self) -> FloatOutBoyLedStripConfig {
        self.status
    }

    #[must_use]
    pub const fn front_strip(self) -> FloatOutBoyLedStripConfig {
        self.front
    }

    #[must_use]
    pub const fn rear_strip(self) -> FloatOutBoyLedStripConfig {
        self.rear
    }

    #[must_use]
    pub const fn uses_internal_leds(self) -> bool {
        self.mode.uses_internal_leds()
    }

    #[must_use]
    pub const fn uses_external_leds(self) -> bool {
        self.mode.uses_external_leds()
    }
}
