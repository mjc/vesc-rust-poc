//! Refloat hardware LED configuration.
//!
//! C map: defaults mirror the hardware LED settings in
//! `third_party/refloat/src/conf/settings.xml:3560-3863`. Refloat treats the
//! mode as flags when enabling internal LEDs and the external LCM at
//! `third_party/refloat/src/leds.c:795-830` and
//! `third_party/refloat/src/lcm.c:27-28`.

use crate::leds::{
    RefloatLedColorOrder, RefloatLedPin, RefloatLedPinConfig, RefloatLedStripConfig,
    RefloatLedStripOrder,
};

use super::mode::RefloatLedMode;

/// Refloat hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatHardwareLedsConfig {
    mode: RefloatLedMode,
    pin: RefloatLedPin,
    pin_config: RefloatLedPinConfig,
    status: RefloatLedStripConfig,
    front: RefloatLedStripConfig,
    rear: RefloatLedStripConfig,
}

impl RefloatHardwareLedsConfig {
    /// Build the hardware LED config from typed Refloat LED mode.
    pub const fn new(mode: RefloatLedMode) -> Self {
        Self {
            mode,
            pin: RefloatLedPin::B7,
            pin_config: RefloatLedPinConfig::PullupTo5v,
            status: RefloatLedStripConfig::new(
                RefloatLedStripOrder::First,
                10,
                RefloatLedColorOrder::Grb,
            ),
            front: RefloatLedStripConfig::new(
                RefloatLedStripOrder::Second,
                20,
                RefloatLedColorOrder::Grb,
            ),
            rear: RefloatLedStripConfig::new(
                RefloatLedStripOrder::Third,
                20,
                RefloatLedColorOrder::Grb,
            ),
        }
    }

    /// Return this config with the LED output pin set.
    pub const fn with_pin(mut self, pin: RefloatLedPin) -> Self {
        self.pin = pin;
        self
    }

    /// Return this config with the LED pin configuration set.
    pub const fn with_pin_config(mut self, pin_config: RefloatLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    /// Return this config with the status strip set.
    pub const fn with_status_strip(mut self, status: RefloatLedStripConfig) -> Self {
        self.status = status;
        self
    }

    /// Return this config with the front strip set.
    pub const fn with_front_strip(mut self, front: RefloatLedStripConfig) -> Self {
        self.front = front;
        self
    }

    /// Return this config with the rear strip set.
    pub const fn with_rear_strip(mut self, rear: RefloatLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    /// Return the configured LED mode.
    pub const fn mode(self) -> RefloatLedMode {
        self.mode
    }

    /// Return the configured LED output pin.
    pub const fn pin(self) -> RefloatLedPin {
        self.pin
    }

    /// Return the configured LED pin mode.
    pub const fn pin_config(self) -> RefloatLedPinConfig {
        self.pin_config
    }

    /// Return the configured status LED strip.
    pub const fn status_strip(self) -> RefloatLedStripConfig {
        self.status
    }

    /// Return the configured front LED strip.
    pub const fn front_strip(self) -> RefloatLedStripConfig {
        self.front
    }

    /// Return the configured rear LED strip.
    pub const fn rear_strip(self) -> RefloatLedStripConfig {
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

/// Refloat hardware configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatHardwareConfig {
    leds: RefloatHardwareLedsConfig,
}

impl RefloatHardwareConfig {
    /// Build a typed Refloat hardware config.
    pub const fn new(leds: RefloatHardwareLedsConfig) -> Self {
        Self { leds }
    }

    /// Return the hardware LED configuration.
    pub const fn leds(self) -> RefloatHardwareLedsConfig {
        self.leds
    }
}

#[cfg(test)]
mod tests {
    use super::{RefloatHardwareConfig, RefloatHardwareLedsConfig, RefloatLedMode};
    use crate::leds::{
        RefloatLedColorOrder, RefloatLedPin, RefloatLedPinConfig, RefloatLedStripConfig,
        RefloatLedStripOrder,
    };

    #[test]
    fn refloat_led_mode_matches_upstream_flag_ids() {
        // C map: Refloat v1.2.1 treats LED mode as flags at
        // `third_party/refloat/src/leds.c:795-830` and external-LCM mode details at
        // `third_party/refloat/src/lcm.c:27-28`; the typed mode IDs mirror
        // `third_party/refloat/src/conf/datatypes.h:36-60`.
        let disabled = RefloatHardwareLedsConfig::new(RefloatLedMode::Off);
        let internal = RefloatHardwareLedsConfig::new(RefloatLedMode::Internal);
        let external = RefloatHardwareLedsConfig::new(RefloatLedMode::External);
        let both = RefloatHardwareLedsConfig::new(RefloatLedMode::Both);

        assert_eq!(RefloatLedMode::Off.id(), 0);
        assert_eq!(RefloatLedMode::Internal.id(), 0x1);
        assert_eq!(RefloatLedMode::External.id(), 0x2);
        assert_eq!(RefloatLedMode::Both.id(), 0x3);
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
    fn refloat_hardware_leds_default_and_overrides_match_upstream_shape() {
        // C map: Refloat's default hardware LED settings come from
        // `third_party/refloat/src/conf/settings.xml:3560-3863`; the mode/pin/pin-config
        // wiring follows the same flags behavior as `third_party/refloat/src/leds.c:795-830`
        // and `third_party/refloat/src/lcm.c:27-28`.
        let defaults = RefloatHardwareLedsConfig::new(RefloatLedMode::Off);

        assert_eq!(defaults.pin(), RefloatLedPin::B7);
        assert_eq!(defaults.pin_config(), RefloatLedPinConfig::PullupTo5v);
        assert_eq!(defaults.status_strip().order(), RefloatLedStripOrder::First);
        assert_eq!(defaults.status_strip().count(), 10);
        assert_eq!(defaults.front_strip().order(), RefloatLedStripOrder::Second);
        assert_eq!(defaults.front_strip().count(), 20);
        assert_eq!(defaults.rear_strip().order(), RefloatLedStripOrder::Third);
        assert_eq!(defaults.rear_strip().count(), 20);

        let status_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::First, 8, RefloatLedColorOrder::Grbw);
        let front_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::Second, 24, RefloatLedColorOrder::Rgb);
        let rear_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::Third, 24, RefloatLedColorOrder::Grb)
                .with_reverse(true);

        let hardware_leds = RefloatHardwareLedsConfig::new(RefloatLedMode::Both)
            .with_pin(RefloatLedPin::C9)
            .with_pin_config(RefloatLedPinConfig::NoPullup)
            .with_status_strip(status_strip)
            .with_front_strip(front_strip)
            .with_rear_strip(rear_strip);
        let hardware = RefloatHardwareConfig::new(hardware_leds);

        assert_eq!(hardware.leds().mode(), RefloatLedMode::Both);
        assert_eq!(hardware.leds().pin(), RefloatLedPin::C9);
        assert_eq!(hardware.leds().pin_config(), RefloatLedPinConfig::NoPullup);
        assert_eq!(
            hardware.leds().status_strip().color_order(),
            RefloatLedColorOrder::Grbw
        );
        assert_eq!(
            hardware.leds().front_strip().color_order(),
            RefloatLedColorOrder::Rgb
        );
        assert!(hardware.leds().rear_strip().is_reversed());
    }
}
