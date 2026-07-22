//! Float Out Boy hardware LED configuration.
//!
//! C map: defaults mirror the hardware LED settings in
//! `third_party/float-out-boy/src/conf/settings.xml:3560-3863`. Float Out Boy treats the
//! mode as flags when enabling internal LEDs and the external LCM at
//! `third_party/float-out-boy/src/leds.c:795-830` and
//! `third_party/float-out-boy/src/lcm.c:27-28`.

use crate::leds::{
    FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
    FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
};

use super::mode::FloatOutBoyLedMode;

/// Float Out Boy hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareLedsConfig {
    mode: FloatOutBoyLedMode,
    pin: FloatOutBoyLedPin,
    pin_config: FloatOutBoyLedPinConfig,
    status: FloatOutBoyLedStripConfig,
    front: FloatOutBoyLedStripConfig,
    rear: FloatOutBoyLedStripConfig,
}

impl FloatOutBoyHardwareLedsConfig {
    /// Build the hardware LED config from typed Float Out Boy LED mode.
    pub const fn new(mode: FloatOutBoyLedMode) -> Self {
        Self {
            mode,
            pin: FloatOutBoyLedPin::B7,
            pin_config: FloatOutBoyLedPinConfig::PullupTo5v,
            status: FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::First,
                10,
                FloatOutBoyLedColorOrder::Grb,
            ),
            front: FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::Second,
                20,
                FloatOutBoyLedColorOrder::Grb,
            ),
            rear: FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::Third,
                20,
                FloatOutBoyLedColorOrder::Grb,
            ),
        }
    }

    /// Return this config with the LED output pin set.
    pub const fn with_pin(mut self, pin: FloatOutBoyLedPin) -> Self {
        self.pin = pin;
        self
    }

    /// Return this config with the LED pin configuration set.
    pub const fn with_pin_config(mut self, pin_config: FloatOutBoyLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    /// Return this config with the status strip set.
    pub const fn with_status_strip(mut self, status: FloatOutBoyLedStripConfig) -> Self {
        self.status = status;
        self
    }

    /// Return this config with the front strip set.
    pub const fn with_front_strip(mut self, front: FloatOutBoyLedStripConfig) -> Self {
        self.front = front;
        self
    }

    /// Return this config with the rear strip set.
    pub const fn with_rear_strip(mut self, rear: FloatOutBoyLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    /// Return the configured LED mode.
    pub const fn mode(self) -> FloatOutBoyLedMode {
        self.mode
    }

    /// Return the configured LED output pin.
    pub const fn pin(self) -> FloatOutBoyLedPin {
        self.pin
    }

    /// Return the configured LED pin mode.
    pub const fn pin_config(self) -> FloatOutBoyLedPinConfig {
        self.pin_config
    }

    /// Return the configured status LED strip.
    pub const fn status_strip(self) -> FloatOutBoyLedStripConfig {
        self.status
    }

    /// Return the configured front LED strip.
    pub const fn front_strip(self) -> FloatOutBoyLedStripConfig {
        self.front
    }

    /// Return the configured rear LED strip.
    pub const fn rear_strip(self) -> FloatOutBoyLedStripConfig {
        self.rear
    }

    /// Return whether internal/status LEDs are enabled.
    pub const fn uses_internal_leds(self) -> bool {
        self.mode.uses_internal_leds()
    }

    /// Return whether external LCM LEDs are enabled.
    pub const fn uses_external_leds(self) -> bool {
        self.mode.uses_external_leds()
    }
}

/// Float Out Boy hardware configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareConfig {
    leds: FloatOutBoyHardwareLedsConfig,
}

impl FloatOutBoyHardwareConfig {
    /// Build a typed Float Out Boy hardware config.
    pub const fn new(leds: FloatOutBoyHardwareLedsConfig) -> Self {
        Self { leds }
    }

    /// Return the hardware LED configuration.
    pub const fn leds(self) -> FloatOutBoyHardwareLedsConfig {
        self.leds
    }
}

#[cfg(test)]
mod tests {
    use super::{FloatOutBoyHardwareConfig, FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode};
    use crate::leds::{
        FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
        FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
    };

    #[test]
    fn float_out_boy_led_mode_matches_upstream_flag_ids() {
        // C map: Float Out Boy v1.2.1 treats LED mode as flags at
        // `third_party/float-out-boy/src/leds.c:795-830` and external-LCM mode details at
        // `third_party/float-out-boy/src/lcm.c:27-28`; the typed mode IDs mirror
        // `third_party/float-out-boy/src/conf/datatypes.h:36-60`.
        let disabled = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);
        let internal = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal);
        let external = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::External);
        let both = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both);

        assert_eq!(FloatOutBoyLedMode::Off.id(), 0);
        assert_eq!(FloatOutBoyLedMode::Internal.id(), 0x1);
        assert_eq!(FloatOutBoyLedMode::External.id(), 0x2);
        assert_eq!(FloatOutBoyLedMode::Both.id(), 0x3);
        assert!(!disabled.uses_internal_leds());
        assert!(!disabled.uses_external_leds());
        assert!(internal.uses_internal_leds());
        assert!(!internal.uses_external_leds());
        assert!(!external.uses_internal_leds());
        assert!(external.uses_external_leds());
        assert!(both.uses_internal_leds());
        assert!(both.uses_external_leds());
    }

    #[test]
    fn float_out_boy_hardware_leds_default_and_overrides_match_upstream_shape() {
        // C map: Float Out Boy's default hardware LED settings come from
        // `third_party/float-out-boy/src/conf/settings.xml:3560-3863`; the mode/pin/pin-config
        // wiring follows the same flags behavior as `third_party/float-out-boy/src/leds.c:795-830`
        // and `third_party/float-out-boy/src/lcm.c:27-28`.
        let defaults = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);

        assert_eq!(defaults.pin(), FloatOutBoyLedPin::B7);
        assert_eq!(defaults.pin_config(), FloatOutBoyLedPinConfig::PullupTo5v);
        assert_eq!(
            defaults.status_strip().order(),
            FloatOutBoyLedStripOrder::First
        );
        assert_eq!(defaults.status_strip().count(), 10);
        assert_eq!(
            defaults.front_strip().order(),
            FloatOutBoyLedStripOrder::Second
        );
        assert_eq!(defaults.front_strip().count(), 20);
        assert_eq!(
            defaults.rear_strip().order(),
            FloatOutBoyLedStripOrder::Third
        );
        assert_eq!(defaults.rear_strip().count(), 20);

        let status_strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            8,
            FloatOutBoyLedColorOrder::Grbw,
        );
        let front_strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Second,
            24,
            FloatOutBoyLedColorOrder::Rgb,
        );
        let rear_strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Third,
            24,
            FloatOutBoyLedColorOrder::Grb,
        )
        .with_reverse(true);

        let hardware_leds = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both)
            .with_pin(FloatOutBoyLedPin::C9)
            .with_pin_config(FloatOutBoyLedPinConfig::NoPullup)
            .with_status_strip(status_strip)
            .with_front_strip(front_strip)
            .with_rear_strip(rear_strip);
        let hardware = FloatOutBoyHardwareConfig::new(hardware_leds);

        assert_eq!(hardware.leds().mode(), FloatOutBoyLedMode::Both);
        assert_eq!(hardware.leds().pin(), FloatOutBoyLedPin::C9);
        assert_eq!(
            hardware.leds().pin_config(),
            FloatOutBoyLedPinConfig::NoPullup
        );
        assert_eq!(
            hardware.leds().status_strip().color_order(),
            FloatOutBoyLedColorOrder::Grbw
        );
        assert_eq!(
            hardware.leds().front_strip().color_order(),
            FloatOutBoyLedColorOrder::Rgb
        );
        assert!(hardware.leds().rear_strip().is_reversed());
    }
}
