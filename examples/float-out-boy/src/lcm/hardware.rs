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

/// Logical role of one configured internal LED strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyLedStripRole {
    /// Status/sensor strip.
    Status,
    /// Front strip.
    Front,
    /// Rear strip.
    Rear,
}

/// A validated source-compatible ordering of the internal strips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyInternalLedLayout {
    roles: [FloatOutBoyLedStripRole; 3],
    role_count: usize,
    status_offset: Option<usize>,
    front_offset: Option<usize>,
    rear_offset: Option<usize>,
    pixel_count: usize,
}

impl FloatOutBoyInternalLedLayout {
    /// Return configured nonempty roles in physical order.
    #[must_use]
    pub fn roles(&self) -> &[FloatOutBoyLedStripRole] {
        self.roles.get(..self.role_count).unwrap_or_default()
    }

    /// Return one role's offset in the shared physical pixel sequence.
    #[must_use]
    pub const fn offset(self, role: FloatOutBoyLedStripRole) -> Option<usize> {
        match role {
            FloatOutBoyLedStripRole::Status => self.status_offset,
            FloatOutBoyLedStripRole::Front => self.front_offset,
            FloatOutBoyLedStripRole::Rear => self.rear_offset,
        }
    }

    /// Return the complete physical pixel count.
    #[must_use]
    pub const fn pixel_count(self) -> usize {
        self.pixel_count
    }
}

/// Internal LED layout validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyInternalLedLayoutError {
    /// Selected front and rear strips exceed Refloat's 60-pixel renderer map.
    FrontAndRearCountExceedsMaximum,
}

impl core::fmt::Display for FloatOutBoyInternalLedLayoutError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FrontAndRearCountExceedsMaximum => {
                formatter.write_str("front and rear LED count exceeds 60")
            }
        }
    }
}

impl core::error::Error for FloatOutBoyInternalLedLayoutError {}

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
    #[must_use]
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
    #[must_use]
    pub const fn with_pin(mut self, pin: FloatOutBoyLedPin) -> Self {
        self.pin = pin;
        self
    }

    /// Return this config with the LED pin configuration set.
    #[must_use]
    pub const fn with_pin_config(mut self, pin_config: FloatOutBoyLedPinConfig) -> Self {
        self.pin_config = pin_config;
        self
    }

    /// Return this config with the status strip set.
    #[must_use]
    pub const fn with_status_strip(mut self, status: FloatOutBoyLedStripConfig) -> Self {
        self.status = status;
        self
    }

    /// Return this config with the front strip set.
    #[must_use]
    pub const fn with_front_strip(mut self, front: FloatOutBoyLedStripConfig) -> Self {
        self.front = front;
        self
    }

    /// Return this config with the rear strip set.
    #[must_use]
    pub const fn with_rear_strip(mut self, rear: FloatOutBoyLedStripConfig) -> Self {
        self.rear = rear;
        self
    }

    /// Return the configured LED mode.
    #[must_use]
    pub const fn mode(self) -> FloatOutBoyLedMode {
        self.mode
    }

    /// Return the configured LED output pin.
    #[must_use]
    pub const fn pin(self) -> FloatOutBoyLedPin {
        self.pin
    }

    /// Return the configured LED pin mode.
    #[must_use]
    pub const fn pin_config(self) -> FloatOutBoyLedPinConfig {
        self.pin_config
    }

    /// Return the configured status LED strip.
    #[must_use]
    pub const fn status_strip(self) -> FloatOutBoyLedStripConfig {
        self.status
    }

    /// Return the configured front LED strip.
    #[must_use]
    pub const fn front_strip(self) -> FloatOutBoyLedStripConfig {
        self.front
    }

    /// Return the configured rear LED strip.
    #[must_use]
    pub const fn rear_strip(self) -> FloatOutBoyLedStripConfig {
        self.rear
    }

    /// Return whether internal/status LEDs are enabled.
    #[must_use]
    pub const fn uses_internal_leds(self) -> bool {
        self.mode.uses_internal_leds()
    }

    /// Return whether external LCM LEDs are enabled.
    #[must_use]
    pub const fn uses_external_leds(self) -> bool {
        self.mode.uses_external_leds()
    }

    /// Build Refloat's nonempty status/front/rear physical strip ordering.
    ///
    /// # Errors
    ///
    /// Returns an error when selected front and rear strips exceed Refloat's
    /// 60-pixel animation-map limit.
    pub fn internal_layout(
        self,
    ) -> Result<FloatOutBoyInternalLedLayout, FloatOutBoyInternalLedLayoutError> {
        let mut layout = FloatOutBoyInternalLedLayout {
            roles: [FloatOutBoyLedStripRole::Status; 3],
            role_count: 0,
            status_offset: None,
            front_offset: None,
            rear_offset: None,
            pixel_count: 0,
        };

        for order in [
            FloatOutBoyLedStripOrder::First,
            FloatOutBoyLedStripOrder::Second,
            FloatOutBoyLedStripOrder::Third,
        ] {
            let selected = if self.status.order() == order && self.status.count() > 0 {
                Some((FloatOutBoyLedStripRole::Status, self.status.count()))
            } else if self.front.order() == order && self.front.count() > 0 {
                Some((FloatOutBoyLedStripRole::Front, self.front.count()))
            } else if self.rear.order() == order && self.rear.count() > 0 {
                Some((FloatOutBoyLedStripRole::Rear, self.rear.count()))
            } else {
                None
            };
            let Some((role, count)) = selected else {
                continue;
            };
            let Some(slot) = layout.roles.get_mut(layout.role_count) else {
                continue;
            };
            *slot = role;
            match role {
                FloatOutBoyLedStripRole::Status => {
                    layout.status_offset = Some(layout.pixel_count);
                }
                FloatOutBoyLedStripRole::Front => {
                    layout.front_offset = Some(layout.pixel_count);
                }
                FloatOutBoyLedStripRole::Rear => {
                    layout.rear_offset = Some(layout.pixel_count);
                }
            }
            layout.role_count = layout.role_count.saturating_add(1);
            layout.pixel_count = layout.pixel_count.saturating_add(usize::from(count));
        }

        let front_count = layout
            .front_offset
            .map_or(0, |_| usize::from(self.front.count()));
        let rear_count = layout
            .rear_offset
            .map_or(0, |_| usize::from(self.rear.count()));
        if front_count.saturating_add(rear_count) > 60 {
            return Err(FloatOutBoyInternalLedLayoutError::FrontAndRearCountExceedsMaximum);
        }

        Ok(layout)
    }
}

/// Float Out Boy hardware configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareConfig {
    leds: FloatOutBoyHardwareLedsConfig,
}

impl FloatOutBoyHardwareConfig {
    /// Build a typed Float Out Boy hardware config.
    #[must_use]
    pub const fn new(leds: FloatOutBoyHardwareLedsConfig) -> Self {
        Self { leds }
    }

    /// Return the hardware LED configuration.
    #[must_use]
    pub const fn leds(self) -> FloatOutBoyHardwareLedsConfig {
        self.leds
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FloatOutBoyHardwareConfig, FloatOutBoyHardwareLedsConfig,
        FloatOutBoyInternalLedLayoutError, FloatOutBoyLedMode, FloatOutBoyLedStripRole,
    };
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

    #[test]
    fn internal_layout_orders_nonempty_strips_with_refloat_priority() {
        let first = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            2,
            FloatOutBoyLedColorOrder::Grb,
        );
        let second = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Second,
            3,
            FloatOutBoyLedColorOrder::Rgb,
        );
        let duplicate_first = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            4,
            FloatOutBoyLedColorOrder::Grbw,
        );
        let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_status_strip(first)
            .with_front_strip(duplicate_first)
            .with_rear_strip(second);

        let layout = config.internal_layout().expect("valid internal layout");

        assert_eq!(
            layout.roles(),
            &[
                FloatOutBoyLedStripRole::Status,
                FloatOutBoyLedStripRole::Rear
            ]
        );
        assert_eq!(layout.offset(FloatOutBoyLedStripRole::Status), Some(0));
        assert_eq!(layout.offset(FloatOutBoyLedStripRole::Rear), Some(2));
        assert_eq!(layout.offset(FloatOutBoyLedStripRole::Front), None);
        assert_eq!(layout.pixel_count(), 5);
    }

    #[test]
    fn internal_layout_matches_refloat_priority_for_every_order_assignment() {
        let orders = [
            FloatOutBoyLedStripOrder::None,
            FloatOutBoyLedStripOrder::First,
            FloatOutBoyLedStripOrder::Second,
            FloatOutBoyLedStripOrder::Third,
        ];

        for status_order in orders {
            for front_order in orders {
                for rear_order in orders {
                    let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
                        .with_status_strip(FloatOutBoyLedStripConfig::new(
                            status_order,
                            2,
                            FloatOutBoyLedColorOrder::Grb,
                        ))
                        .with_front_strip(FloatOutBoyLedStripConfig::new(
                            front_order,
                            3,
                            FloatOutBoyLedColorOrder::Grb,
                        ))
                        .with_rear_strip(FloatOutBoyLedStripConfig::new(
                            rear_order,
                            5,
                            FloatOutBoyLedColorOrder::Grb,
                        ));
                    let layout = config.internal_layout().expect("small layout is valid");
                    let candidates = [
                        (FloatOutBoyLedStripRole::Status, status_order, 2_usize),
                        (FloatOutBoyLedStripRole::Front, front_order, 3_usize),
                        (FloatOutBoyLedStripRole::Rear, rear_order, 5_usize),
                    ];
                    let mut expected_roles = std::vec::Vec::new();
                    let mut expected_offsets = [None; 3];
                    let mut expected_count = 0;

                    for order in [
                        FloatOutBoyLedStripOrder::First,
                        FloatOutBoyLedStripOrder::Second,
                        FloatOutBoyLedStripOrder::Third,
                    ] {
                        if let Some((role, _, count)) = candidates
                            .iter()
                            .find(|(_, candidate, _)| *candidate == order)
                        {
                            expected_roles.push(*role);
                            let index = match role {
                                FloatOutBoyLedStripRole::Status => 0,
                                FloatOutBoyLedStripRole::Front => 1,
                                FloatOutBoyLedStripRole::Rear => 2,
                            };
                            expected_offsets[index] = Some(expected_count);
                            expected_count += count;
                        }
                    }

                    assert_eq!(layout.roles(), expected_roles);
                    assert_eq!(
                        [
                            layout.offset(FloatOutBoyLedStripRole::Status),
                            layout.offset(FloatOutBoyLedStripRole::Front),
                            layout.offset(FloatOutBoyLedStripRole::Rear),
                        ],
                        expected_offsets
                    );
                    assert_eq!(layout.pixel_count(), expected_count);
                }
            }
        }
    }

    #[test]
    fn internal_layout_rejects_only_selected_front_rear_overflow() {
        let disabled = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            255,
            FloatOutBoyLedColorOrder::Grb,
        );
        let front = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            31,
            FloatOutBoyLedColorOrder::Grb,
        );
        let rear = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Second,
            30,
            FloatOutBoyLedColorOrder::Grb,
        );
        let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_status_strip(disabled)
            .with_front_strip(front)
            .with_rear_strip(rear);

        assert_eq!(
            config.internal_layout(),
            Err(FloatOutBoyInternalLedLayoutError::FrontAndRearCountExceedsMaximum)
        );

        let empty = config
            .with_front_strip(disabled)
            .with_rear_strip(disabled)
            .internal_layout()
            .expect("order none omits even nonzero strips");
        assert!(empty.roles().is_empty());
        assert_eq!(empty.pixel_count(), 0);
    }
}
