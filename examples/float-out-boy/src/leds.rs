//! Float Out Boy LED support types.
//!
//! These types model Float Out Boy's internal LED configuration surface. Raw config
//! field packing stays at package/config boundaries.

use vescpkg_rs::prelude::Ratio;

/// Float Out Boy hardware LED output pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedPin {
    /// STM32 pin B6.
    B6,
    /// STM32 pin B7.
    B7,
    /// STM32 pin C9.
    C9,
}

impl FloatOutBoyLedPin {
    /// Return the Float Out Boy `v1.2.1` LED pin ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::B6 => 0,
            Self::B7 => 1,
            Self::C9 => 2,
        }
    }
}

/// Float Out Boy hardware LED pin pull-up configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedPinConfig {
    /// Enable the 5V pull-up.
    PullupTo5v,
    /// Leave the LED pin without pull-up.
    NoPullup,
}

impl FloatOutBoyLedPinConfig {
    /// Return the Float Out Boy `v1.2.1` LED pin config ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::PullupTo5v => 0,
            Self::NoPullup => 1,
        }
    }
}

/// Float Out Boy LED color channel order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedColorOrder {
    /// Green, red, blue.
    Grb,
    /// Green, red, blue, white.
    Grbw,
    /// Red, green, blue.
    Rgb,
    /// White, red, green, blue.
    Wrgb,
}

impl FloatOutBoyLedColorOrder {
    /// Return the Float Out Boy `v1.2.1` LED color order ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::Grb => 0,
            Self::Grbw => 1,
            Self::Rgb => 2,
            Self::Wrgb => 3,
        }
    }
}

/// Float Out Boy named LED color.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedColor {
    /// Black/off.
    Black = 0,
    /// White using all channels.
    WhiteFull = 1,
    /// White using RGB channels.
    WhiteRgb = 2,
    /// White using the white channel.
    WhiteSingle = 3,
    /// Red.
    Red = 4,
    /// Ferrari red.
    Ferrari = 5,
    /// Flame.
    Flame = 6,
    /// Coral.
    Coral = 7,
    /// Sunset.
    Sunset = 8,
    /// Sunrise.
    Sunrise = 9,
    /// Gold.
    Gold = 10,
    /// Orange.
    Orange = 11,
    /// Yellow.
    Yellow = 12,
    /// Banana.
    Banana = 13,
    /// Lime.
    Lime = 14,
    /// Acid.
    Acid = 15,
    /// Sage.
    Sage = 16,
    /// Green.
    Green = 17,
    /// Mint.
    Mint = 18,
    /// Tiffany.
    Tiffany = 19,
    /// Cyan.
    Cyan = 20,
    /// Steel.
    Steel = 21,
    /// Sky.
    Sky = 22,
    /// Azure.
    Azure = 23,
    /// Sapphire.
    Sapphire = 24,
    /// Blue.
    Blue = 25,
    /// Violet.
    Violet = 26,
    /// Amethyst.
    Amethyst = 27,
    /// Magenta.
    Magenta = 28,
    /// Pink.
    Pink = 29,
    /// Fuchsia.
    Fuchsia = 30,
    /// Lavender.
    Lavender = 31,
}

impl FloatOutBoyLedColor {
    /// Return the Float Out Boy `v1.2.1` LED color ID.
    ///
    /// C map: these IDs follow the `enumNames` order for LED color config fields at
    /// `third_party/float-out-boy/src/conf/settings.xml:3456-3487`.
    #[must_use]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// Float Out Boy LED animation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedAnimationMode {
    /// Solid color.
    Solid,
    /// Fade between colors.
    Fade,
    /// Pulse between colors.
    Pulse,
    /// Strobe between colors.
    Strobe,
    /// Knight-rider sweep.
    KnightRider,
    /// Alternating red/blue style animation.
    Felony,
    /// Cycle rainbow colors.
    RainbowCycle,
    /// Fade rainbow colors.
    RainbowFade,
    /// Roll rainbow colors.
    RainbowRoll,
}

impl FloatOutBoyLedAnimationMode {
    /// Return the Float Out Boy `v1.2.1` LED animation mode ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::Solid => 0,
            Self::Fade => 1,
            Self::Pulse => 2,
            Self::Strobe => 3,
            Self::KnightRider => 4,
            Self::Felony => 5,
            Self::RainbowCycle => 6,
            Self::RainbowFade => 7,
            Self::RainbowRoll => 8,
        }
    }
}

/// Float Out Boy LED transition mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedTransition {
    /// Fade directly to the target bar.
    Fade,
    /// Fade out, then fade in.
    FadeOutIn,
    /// Cipher transition.
    Cipher,
    /// Monochrome cipher transition.
    MonoCipher,
}

impl FloatOutBoyLedTransition {
    /// Return the Float Out Boy `v1.2.1` LED transition ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::Fade => 0,
            Self::FadeOutIn => 1,
            Self::Cipher => 2,
            Self::MonoCipher => 3,
        }
    }
}

/// Float Out Boy LED animation speed scalar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FloatOutBoyLedAnimationSpeed(f32);

impl FloatOutBoyLedAnimationSpeed {
    /// Wrap a Float Out Boy LED animation speed value.
    #[must_use]
    pub const fn from_units(value: f32) -> Self {
        Self(value)
    }

    /// Return the Float Out Boy LED animation speed value.
    #[must_use]
    pub const fn as_units(self) -> f32 {
        self.0
    }
}

/// Float Out Boy LED bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedBarConfig {
    brightness: Ratio,
    primary_color: FloatOutBoyLedColor,
    secondary_color: FloatOutBoyLedColor,
    animation_mode: FloatOutBoyLedAnimationMode,
    animation_speed: FloatOutBoyLedAnimationSpeed,
}

impl FloatOutBoyLedBarConfig {
    /// Build a typed Float Out Boy LED bar config.
    #[must_use]
    pub const fn new(
        brightness: Ratio,
        primary_color: FloatOutBoyLedColor,
        secondary_color: FloatOutBoyLedColor,
        animation_mode: FloatOutBoyLedAnimationMode,
        animation_speed: FloatOutBoyLedAnimationSpeed,
    ) -> Self {
        Self {
            brightness,
            primary_color,
            secondary_color,
            animation_mode,
            animation_speed,
        }
    }

    /// Return the configured brightness.
    #[must_use]
    pub const fn brightness(self) -> Ratio {
        self.brightness
    }

    /// Return the primary LED color.
    #[must_use]
    pub const fn primary_color(self) -> FloatOutBoyLedColor {
        self.primary_color
    }

    /// Return the secondary LED color.
    #[must_use]
    pub const fn secondary_color(self) -> FloatOutBoyLedColor {
        self.secondary_color
    }

    /// Return the animation mode.
    #[must_use]
    pub const fn animation_mode(self) -> FloatOutBoyLedAnimationMode {
        self.animation_mode
    }

    /// Return the animation speed.
    #[must_use]
    pub const fn animation_speed(self) -> FloatOutBoyLedAnimationSpeed {
        self.animation_speed
    }
}

/// Float Out Boy status-bar idle timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FloatOutBoyStatusBarIdleTimeout(u16);

impl FloatOutBoyStatusBarIdleTimeout {
    /// Wrap a Float Out Boy status-bar idle timeout in seconds.
    #[must_use]
    pub const fn from_seconds(value: u16) -> Self {
        Self(value)
    }

    /// Return the idle timeout in seconds.
    #[must_use]
    pub const fn as_seconds(self) -> u16 {
        self.0
    }
}

/// Float Out Boy status-bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyStatusBarConfig {
    idle_timeout: FloatOutBoyStatusBarIdleTimeout,
    duty_threshold: Ratio,
    red_bar_percentage: Ratio,
    show_sensors_while_running: bool,
    brightness_headlights_on: Ratio,
    brightness_headlights_off: Ratio,
}

impl FloatOutBoyStatusBarConfig {
    /// Build a typed Float Out Boy status-bar config.
    #[must_use]
    pub const fn new(
        idle_timeout: FloatOutBoyStatusBarIdleTimeout,
        duty_threshold: Ratio,
        red_bar_percentage: Ratio,
        brightness_headlights_on: Ratio,
        brightness_headlights_off: Ratio,
    ) -> Self {
        Self {
            idle_timeout,
            duty_threshold,
            red_bar_percentage,
            show_sensors_while_running: false,
            brightness_headlights_on,
            brightness_headlights_off,
        }
    }

    /// Return this config with sensor display enabled while running.
    #[must_use]
    pub const fn showing_sensors_while_running(mut self) -> Self {
        self.show_sensors_while_running = true;
        self
    }

    /// Return the idle timeout.
    #[must_use]
    pub const fn idle_timeout(self) -> FloatOutBoyStatusBarIdleTimeout {
        self.idle_timeout
    }

    /// Return the duty threshold for switching status display.
    #[must_use]
    pub const fn duty_threshold(self) -> Ratio {
        self.duty_threshold
    }

    /// Return the red-bar percentage threshold.
    #[must_use]
    pub const fn red_bar_percentage(self) -> Ratio {
        self.red_bar_percentage
    }

    /// Return whether sensors are shown while running.
    #[must_use]
    pub const fn shows_sensors_while_running(self) -> bool {
        self.show_sensors_while_running
    }

    /// Return status brightness when headlights are on.
    #[must_use]
    pub const fn brightness_headlights_on(self) -> Ratio {
        self.brightness_headlights_on
    }

    /// Return status brightness when headlights are off.
    #[must_use]
    pub const fn brightness_headlights_off(self) -> Ratio {
        self.brightness_headlights_off
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyLiftedLedsConfig {
    lights_off: bool,
    status_on_front: bool,
}

/// Float Out Boy LEDs configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedsConfig {
    on: bool,
    headlights_on: bool,
    headlights_transition: FloatOutBoyLedTransition,
    direction_transition: FloatOutBoyLedTransition,
    lifted: FloatOutBoyLiftedLedsConfig,
    headlights: FloatOutBoyLedBarConfig,
    taillights: FloatOutBoyLedBarConfig,
    front: FloatOutBoyLedBarConfig,
    rear: FloatOutBoyLedBarConfig,
    status: FloatOutBoyStatusBarConfig,
    status_idle: FloatOutBoyLedBarConfig,
}

impl FloatOutBoyLedsConfig {
    /// Build a typed Float Out Boy LEDs config.
    #[must_use]
    pub const fn new(
        headlights: FloatOutBoyLedBarConfig,
        taillights: FloatOutBoyLedBarConfig,
        front: FloatOutBoyLedBarConfig,
        rear: FloatOutBoyLedBarConfig,
        status: FloatOutBoyStatusBarConfig,
        status_idle: FloatOutBoyLedBarConfig,
    ) -> Self {
        Self {
            on: false,
            headlights_on: false,
            headlights_transition: FloatOutBoyLedTransition::Fade,
            direction_transition: FloatOutBoyLedTransition::Fade,
            lifted: FloatOutBoyLiftedLedsConfig {
                lights_off: false,
                status_on_front: false,
            },
            headlights,
            taillights,
            front,
            rear,
            status,
            status_idle,
        }
    }

    /// Return this config with LEDs enabled.
    #[must_use]
    pub const fn enabled(mut self) -> Self {
        self.on = true;
        self
    }

    /// Return this config with headlights enabled.
    #[must_use]
    pub const fn with_headlights_on(mut self) -> Self {
        self.headlights_on = true;
        self
    }

    /// Return this config with the headlights transition set.
    #[must_use]
    pub const fn with_headlights_transition(
        mut self,
        transition: FloatOutBoyLedTransition,
    ) -> Self {
        self.headlights_transition = transition;
        self
    }

    /// Return this config with the direction transition set.
    #[must_use]
    pub const fn with_direction_transition(mut self, transition: FloatOutBoyLedTransition) -> Self {
        self.direction_transition = transition;
        self
    }

    /// Return this config with lights off while lifted.
    #[must_use]
    pub const fn lights_off_when_lifted(mut self) -> Self {
        self.lifted.lights_off = true;
        self
    }

    /// Return this config with status shown on the front while lifted.
    #[must_use]
    pub const fn status_on_front_when_lifted(mut self) -> Self {
        self.lifted.status_on_front = true;
        self
    }

    /// Return whether LEDs are enabled.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.on
    }

    /// Return whether headlights are on.
    #[must_use]
    pub const fn are_headlights_on(self) -> bool {
        self.headlights_on
    }

    /// Return the headlights transition.
    #[must_use]
    pub const fn headlights_transition(self) -> FloatOutBoyLedTransition {
        self.headlights_transition
    }

    /// Return the direction transition.
    #[must_use]
    pub const fn direction_transition(self) -> FloatOutBoyLedTransition {
        self.direction_transition
    }

    /// Return whether lights are turned off while lifted.
    #[must_use]
    pub const fn turns_lights_off_when_lifted(self) -> bool {
        self.lifted.lights_off
    }

    /// Return whether status is shown on the front while lifted.
    #[must_use]
    pub const fn shows_status_on_front_when_lifted(self) -> bool {
        self.lifted.status_on_front
    }

    /// Return the headlights LED bar config.
    #[must_use]
    pub const fn headlights(self) -> FloatOutBoyLedBarConfig {
        self.headlights
    }

    /// Return the taillights LED bar config.
    #[must_use]
    pub const fn taillights(self) -> FloatOutBoyLedBarConfig {
        self.taillights
    }

    /// Return the front LED bar config.
    #[must_use]
    pub const fn front(self) -> FloatOutBoyLedBarConfig {
        self.front
    }

    /// Return the rear LED bar config.
    #[must_use]
    pub const fn rear(self) -> FloatOutBoyLedBarConfig {
        self.rear
    }

    /// Return the status-bar config.
    #[must_use]
    pub const fn status(self) -> FloatOutBoyStatusBarConfig {
        self.status
    }

    /// Return the idle status LED bar config.
    #[must_use]
    pub const fn status_idle(self) -> FloatOutBoyLedBarConfig {
        self.status_idle
    }
}

/// Float Out Boy physical LED strip order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedStripOrder {
    /// No strip is assigned.
    None,
    /// First LED strip.
    First,
    /// Second LED strip.
    Second,
    /// Third LED strip.
    Third,
}

impl FloatOutBoyLedStripOrder {
    /// Return the Float Out Boy `v1.2.1` LED strip order ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::First => 1,
            Self::Second => 2,
            Self::Third => 3,
        }
    }
}

/// Float Out Boy LED strip configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyLedStripConfig {
    order: FloatOutBoyLedStripOrder,
    count: u8,
    color_order: FloatOutBoyLedColorOrder,
    reverse: bool,
}

impl FloatOutBoyLedStripConfig {
    /// Build a typed Float Out Boy LED strip config.
    #[must_use]
    pub const fn new(
        order: FloatOutBoyLedStripOrder,
        count: u8,
        color_order: FloatOutBoyLedColorOrder,
    ) -> Self {
        Self {
            order,
            count,
            color_order,
            reverse: false,
        }
    }

    /// Return this config with reverse ordering enabled or disabled.
    #[must_use]
    pub const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Return the physical strip order.
    #[must_use]
    pub const fn order(self) -> FloatOutBoyLedStripOrder {
        self.order
    }

    /// Return the configured LED count.
    #[must_use]
    pub const fn count(self) -> u8 {
        self.count
    }

    /// Return the configured color channel order.
    #[must_use]
    pub const fn color_order(self) -> FloatOutBoyLedColorOrder {
        self.color_order
    }

    /// Return whether LED indexing is reversed.
    #[must_use]
    pub const fn is_reversed(self) -> bool {
        self.reverse
    }
}
