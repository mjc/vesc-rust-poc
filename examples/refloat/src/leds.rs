//! Refloat LED support types.
//!
//! These types model Refloat's internal LED configuration surface. Raw config
//! field packing stays at package/config boundaries.

use vescpkg_rs::prelude::Ratio;

/// Refloat hardware LED output pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedPin {
    /// STM32 pin B6.
    B6,
    /// STM32 pin B7.
    B7,
    /// STM32 pin C9.
    C9,
}

impl RefloatLedPin {
    /// Return the Refloat `v1.2.1` LED pin ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::B6 => 0,
            Self::B7 => 1,
            Self::C9 => 2,
        }
    }
}

/// Refloat hardware LED pin pull-up configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedPinConfig {
    /// Enable the 5V pull-up.
    PullupTo5v,
    /// Leave the LED pin without pull-up.
    NoPullup,
}

impl RefloatLedPinConfig {
    /// Return the Refloat `v1.2.1` LED pin config ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::PullupTo5v => 0,
            Self::NoPullup => 1,
        }
    }
}

/// Refloat LED color channel order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedColorOrder {
    /// Green, red, blue.
    Grb,
    /// Green, red, blue, white.
    Grbw,
    /// Red, green, blue.
    Rgb,
    /// White, red, green, blue.
    Wrgb,
}

impl RefloatLedColorOrder {
    /// Return the Refloat `v1.2.1` LED color order ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Grb => 0,
            Self::Grbw => 1,
            Self::Rgb => 2,
            Self::Wrgb => 3,
        }
    }
}

/// Refloat named LED color.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedColor {
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

impl RefloatLedColor {
    /// Return the Refloat `v1.2.1` LED color ID.
    ///
    /// C map: these IDs follow the `enumNames` order for LED color config fields at
    /// `third_party/refloat/src/conf/settings.xml:3456-3487`.
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// Refloat LED animation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedAnimationMode {
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

impl RefloatLedAnimationMode {
    /// Return the Refloat `v1.2.1` LED animation mode ID.
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

/// Refloat LED transition mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedTransition {
    /// Fade directly to the target bar.
    Fade,
    /// Fade out, then fade in.
    FadeOutIn,
    /// Cipher transition.
    Cipher,
    /// Monochrome cipher transition.
    MonoCipher,
}

impl RefloatLedTransition {
    /// Return the Refloat `v1.2.1` LED transition ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::Fade => 0,
            Self::FadeOutIn => 1,
            Self::Cipher => 2,
            Self::MonoCipher => 3,
        }
    }
}

/// Refloat LED animation speed scalar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatLedAnimationSpeed(f32);

impl RefloatLedAnimationSpeed {
    /// Wrap a Refloat LED animation speed value.
    pub const fn from_units(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat LED animation speed value.
    pub const fn as_units(self) -> f32 {
        self.0
    }
}

/// Refloat LED bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatLedBarConfig {
    brightness: Ratio,
    primary_color: RefloatLedColor,
    secondary_color: RefloatLedColor,
    animation_mode: RefloatLedAnimationMode,
    animation_speed: RefloatLedAnimationSpeed,
}

impl RefloatLedBarConfig {
    /// Build a typed Refloat LED bar config.
    pub const fn new(
        brightness: Ratio,
        primary_color: RefloatLedColor,
        secondary_color: RefloatLedColor,
        animation_mode: RefloatLedAnimationMode,
        animation_speed: RefloatLedAnimationSpeed,
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
    pub const fn brightness(self) -> Ratio {
        self.brightness
    }

    /// Return the primary LED color.
    pub const fn primary_color(self) -> RefloatLedColor {
        self.primary_color
    }

    /// Return the secondary LED color.
    pub const fn secondary_color(self) -> RefloatLedColor {
        self.secondary_color
    }

    /// Return the animation mode.
    pub const fn animation_mode(self) -> RefloatLedAnimationMode {
        self.animation_mode
    }

    /// Return the animation speed.
    pub const fn animation_speed(self) -> RefloatLedAnimationSpeed {
        self.animation_speed
    }
}

/// Refloat status-bar idle timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RefloatStatusBarIdleTimeout(u16);

impl RefloatStatusBarIdleTimeout {
    /// Wrap a Refloat status-bar idle timeout in seconds.
    pub const fn from_seconds(value: u16) -> Self {
        Self(value)
    }

    /// Return the idle timeout in seconds.
    pub const fn as_seconds(self) -> u16 {
        self.0
    }
}

/// Refloat status-bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatStatusBarConfig {
    idle_timeout: RefloatStatusBarIdleTimeout,
    duty_threshold: Ratio,
    red_bar_percentage: Ratio,
    show_sensors_while_running: bool,
    brightness_headlights_on: Ratio,
    brightness_headlights_off: Ratio,
}

impl RefloatStatusBarConfig {
    /// Build a typed Refloat status-bar config.
    pub const fn new(
        idle_timeout: RefloatStatusBarIdleTimeout,
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
    pub const fn showing_sensors_while_running(mut self) -> Self {
        self.show_sensors_while_running = true;
        self
    }

    /// Return the idle timeout.
    pub const fn idle_timeout(self) -> RefloatStatusBarIdleTimeout {
        self.idle_timeout
    }

    /// Return the duty threshold for switching status display.
    pub const fn duty_threshold(self) -> Ratio {
        self.duty_threshold
    }

    /// Return the red-bar percentage threshold.
    pub const fn red_bar_percentage(self) -> Ratio {
        self.red_bar_percentage
    }

    /// Return whether sensors are shown while running.
    pub const fn shows_sensors_while_running(self) -> bool {
        self.show_sensors_while_running
    }

    /// Return status brightness when headlights are on.
    pub const fn brightness_headlights_on(self) -> Ratio {
        self.brightness_headlights_on
    }

    /// Return status brightness when headlights are off.
    pub const fn brightness_headlights_off(self) -> Ratio {
        self.brightness_headlights_off
    }
}

/// Refloat LEDs configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatLedsConfig {
    on: bool,
    headlights_on: bool,
    headlights_transition: RefloatLedTransition,
    direction_transition: RefloatLedTransition,
    lights_off_when_lifted: bool,
    status_on_front_when_lifted: bool,
    headlights: RefloatLedBarConfig,
    taillights: RefloatLedBarConfig,
    front: RefloatLedBarConfig,
    rear: RefloatLedBarConfig,
    status: RefloatStatusBarConfig,
    status_idle: RefloatLedBarConfig,
}

impl RefloatLedsConfig {
    /// Build a typed Refloat LEDs config.
    pub const fn new(
        headlights: RefloatLedBarConfig,
        taillights: RefloatLedBarConfig,
        front: RefloatLedBarConfig,
        rear: RefloatLedBarConfig,
        status: RefloatStatusBarConfig,
        status_idle: RefloatLedBarConfig,
    ) -> Self {
        Self {
            on: false,
            headlights_on: false,
            headlights_transition: RefloatLedTransition::Fade,
            direction_transition: RefloatLedTransition::Fade,
            lights_off_when_lifted: false,
            status_on_front_when_lifted: false,
            headlights,
            taillights,
            front,
            rear,
            status,
            status_idle,
        }
    }

    /// Return this config with LEDs enabled.
    pub const fn enabled(mut self) -> Self {
        self.on = true;
        self
    }

    /// Return this config with headlights enabled.
    pub const fn with_headlights_on(mut self) -> Self {
        self.headlights_on = true;
        self
    }

    /// Return this config with the headlights transition set.
    pub const fn with_headlights_transition(mut self, transition: RefloatLedTransition) -> Self {
        self.headlights_transition = transition;
        self
    }

    /// Return this config with the direction transition set.
    pub const fn with_direction_transition(mut self, transition: RefloatLedTransition) -> Self {
        self.direction_transition = transition;
        self
    }

    /// Return this config with lights off while lifted.
    pub const fn lights_off_when_lifted(mut self) -> Self {
        self.lights_off_when_lifted = true;
        self
    }

    /// Return this config with status shown on the front while lifted.
    pub const fn status_on_front_when_lifted(mut self) -> Self {
        self.status_on_front_when_lifted = true;
        self
    }

    /// Return whether LEDs are enabled.
    pub const fn is_enabled(self) -> bool {
        self.on
    }

    /// Return whether headlights are on.
    pub const fn are_headlights_on(self) -> bool {
        self.headlights_on
    }

    /// Return the headlights transition.
    pub const fn headlights_transition(self) -> RefloatLedTransition {
        self.headlights_transition
    }

    /// Return the direction transition.
    pub const fn direction_transition(self) -> RefloatLedTransition {
        self.direction_transition
    }

    /// Return whether lights are turned off while lifted.
    pub const fn turns_lights_off_when_lifted(self) -> bool {
        self.lights_off_when_lifted
    }

    /// Return whether status is shown on the front while lifted.
    pub const fn shows_status_on_front_when_lifted(self) -> bool {
        self.status_on_front_when_lifted
    }

    /// Return the headlights LED bar config.
    pub const fn headlights(self) -> RefloatLedBarConfig {
        self.headlights
    }

    /// Return the taillights LED bar config.
    pub const fn taillights(self) -> RefloatLedBarConfig {
        self.taillights
    }

    /// Return the front LED bar config.
    pub const fn front(self) -> RefloatLedBarConfig {
        self.front
    }

    /// Return the rear LED bar config.
    pub const fn rear(self) -> RefloatLedBarConfig {
        self.rear
    }

    /// Return the status-bar config.
    pub const fn status(self) -> RefloatStatusBarConfig {
        self.status
    }

    /// Return the idle status LED bar config.
    pub const fn status_idle(self) -> RefloatLedBarConfig {
        self.status_idle
    }
}

/// Refloat physical LED strip order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatLedStripOrder {
    /// No strip is assigned.
    None,
    /// First LED strip.
    First,
    /// Second LED strip.
    Second,
    /// Third LED strip.
    Third,
}

impl RefloatLedStripOrder {
    /// Return the Refloat `v1.2.1` LED strip order ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::First => 1,
            Self::Second => 2,
            Self::Third => 3,
        }
    }
}

/// Refloat LED strip configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatLedStripConfig {
    order: RefloatLedStripOrder,
    count: u8,
    color_order: RefloatLedColorOrder,
    reverse: bool,
}

impl RefloatLedStripConfig {
    /// Build a typed Refloat LED strip config.
    pub const fn new(
        order: RefloatLedStripOrder,
        count: u8,
        color_order: RefloatLedColorOrder,
    ) -> Self {
        Self {
            order,
            count,
            color_order,
            reverse: false,
        }
    }

    /// Return this config with reverse ordering enabled or disabled.
    pub const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Return the physical strip order.
    pub const fn order(self) -> RefloatLedStripOrder {
        self.order
    }

    /// Return the configured LED count.
    pub const fn count(self) -> u8 {
        self.count
    }

    /// Return the configured color channel order.
    pub const fn color_order(self) -> RefloatLedColorOrder {
        self.color_order
    }

    /// Return whether LED indexing is reversed.
    pub const fn is_reversed(self) -> bool {
        self.reverse
    }
}
