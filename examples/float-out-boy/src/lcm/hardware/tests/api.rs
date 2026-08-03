use super::*;
use crate::leds::FloatOutBoyLedColorOrder;

impl FloatOutBoyLedMode {
    pub(crate) const fn uses_internal_leds(self) -> bool {
        matches!(self, Self::Internal | Self::Both)
    }

    pub(crate) const fn uses_external_leds(self) -> bool {
        matches!(self, Self::External | Self::Both)
    }
}

impl FloatOutBoyHardwareLedsConfig {
    pub(crate) const fn new(mode: FloatOutBoyLedMode) -> Self {
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

    pub(crate) const fn with_pin(mut self, pin: FloatOutBoyLedPin) -> Self {
        self.pin = pin;
        self
    }

    pub(crate) const fn with_pin_config(mut self, pin_config: FloatOutBoyLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    pub(crate) const fn with_status_strip(mut self, status: FloatOutBoyLedStripConfig) -> Self {
        self.status = status;
        self
    }

    pub(crate) const fn with_front_strip(mut self, front: FloatOutBoyLedStripConfig) -> Self {
        self.front = front;
        self
    }

    pub(crate) const fn with_rear_strip(mut self, rear: FloatOutBoyLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    pub(crate) const fn mode(self) -> FloatOutBoyLedMode {
        self.mode
    }

    pub(crate) const fn pin(self) -> FloatOutBoyLedPin {
        self.pin
    }

    pub(crate) const fn pin_config(self) -> FloatOutBoyLedPinConfig {
        self.pin_config
    }

    pub(crate) const fn status_strip(self) -> FloatOutBoyLedStripConfig {
        self.status
    }

    pub(crate) const fn front_strip(self) -> FloatOutBoyLedStripConfig {
        self.front
    }

    pub(crate) const fn rear_strip(self) -> FloatOutBoyLedStripConfig {
        self.rear
    }

    pub(crate) const fn uses_internal_leds(self) -> bool {
        self.mode.uses_internal_leds()
    }

    pub(crate) const fn uses_external_leds(self) -> bool {
        self.mode.uses_external_leds()
    }
}
