//! Float Out Boy LED support types.
//!
//! These types model Float Out Boy's internal LED configuration surface. Raw config
//! field packing stays at package/config boundaries.

use vescpkg_rs::prelude::Ratio;

/// Float Out Boy hardware LED output pin.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedPin {
    /// STM32 pin B6.
    B6 = 0,
    /// STM32 pin B7.
    B7 = 1,
    /// STM32 pin C9.
    C9 = 2,
}

impl FloatOutBoyLedPin {
    /// Return the Float Out Boy `v1.2.1` LED pin ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// Float Out Boy hardware LED pin pull-up configuration.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedPinConfig {
    /// Enable the 5V pull-up.
    PullupTo5v = 0,
    /// Leave the LED pin without pull-up.
    NoPullup = 1,
}

impl FloatOutBoyLedPinConfig {
    /// Return the Float Out Boy `v1.2.1` LED pin config ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// Float Out Boy LED color channel order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedColorOrder {
    /// Green, red, blue.
    Grb = 0,
    /// Green, red, blue, white.
    Grbw = 1,
    /// Red, green, blue.
    Rgb = 2,
    /// White, red, green, blue.
    Wrgb = 3,
}

impl FloatOutBoyLedColorOrder {
    /// Return the Float Out Boy `v1.2.1` LED color order ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
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
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// One renderer pixel in red, green, blue, white channel order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FloatOutBoyLedPixel {
    channels: [u8; 4],
}

/// Gamma-corrected physical channels in one configured strip's wire order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyLedPhysicalChannels {
    bytes: [u8; 4],
    len: usize,
}

impl FloatOutBoyLedPhysicalChannels {
    /// Return the three or four physical channel bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }
}

const NAMED_LED_COLOR_CHANNELS: [[u8; 4]; 32] = [
    [0x00, 0x00, 0x00, 0x00],
    [0xff, 0xff, 0xff, 0xff],
    [0xff, 0xff, 0xff, 0x00],
    [0x00, 0x00, 0x00, 0xff],
    [0xff, 0x00, 0x00, 0x00],
    [0xff, 0x38, 0x00, 0x00],
    [0xff, 0x50, 0x00, 0x00],
    [0xff, 0x60, 0x40, 0x00],
    [0xff, 0x78, 0x30, 0x00],
    [0xff, 0x90, 0x40, 0x00],
    [0xff, 0x80, 0x20, 0x00],
    [0xff, 0x78, 0x00, 0x00],
    [0xff, 0xa0, 0x00, 0x00],
    [0xff, 0xb0, 0x40, 0x00],
    [0xff, 0xff, 0x00, 0x00],
    [0xa0, 0xff, 0x00, 0x00],
    [0xa0, 0xff, 0x50, 0x00],
    [0x00, 0xff, 0x00, 0x00],
    [0x00, 0xff, 0x50, 0x00],
    [0x00, 0xff, 0xc0, 0x00],
    [0x00, 0xff, 0xff, 0x00],
    [0x90, 0xc0, 0xff, 0x00],
    [0x70, 0xd0, 0xff, 0x00],
    [0x00, 0xa0, 0xff, 0x00],
    [0x00, 0x70, 0xff, 0x00],
    [0x00, 0x00, 0xff, 0x00],
    [0x80, 0x00, 0xff, 0x00],
    [0xa0, 0x60, 0xff, 0x00],
    [0xff, 0x00, 0xff, 0x00],
    [0xff, 0x00, 0xc0, 0x00],
    [0xff, 0x00, 0x70, 0x00],
    [0xff, 0x70, 0xa0, 0x00],
];

impl FloatOutBoyLedPixel {
    /// Return Refloat 1.2.1's exact channel values for a named color.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) color ID is the checked palette index"
    )]
    pub fn from_named(color: FloatOutBoyLedColor) -> Self {
        let channels = NAMED_LED_COLOR_CHANNELS
            .get(color.id() as usize)
            .copied()
            .unwrap_or_default();
        Self { channels }
    }

    /// Return the pixel as red, green, blue, white channel values.
    #[must_use]
    pub const fn channels(self) -> [u8; 4] {
        self.channels
    }

    /// Gamma-correct and reorder this pixel for one physical strip.
    #[must_use]
    pub fn physical_channels(
        self,
        order: FloatOutBoyLedColorOrder,
    ) -> FloatOutBoyLedPhysicalChannels {
        let [red, green, blue, white] = self.channels.map(refloat_led_gamma);
        let (bytes, len) = match order {
            FloatOutBoyLedColorOrder::Grb => ([green, red, blue, 0], 3),
            FloatOutBoyLedColorOrder::Grbw => ([green, red, blue, white], 4),
            FloatOutBoyLedColorOrder::Rgb => ([red, green, blue, 0], 3),
            FloatOutBoyLedColorOrder::Wrgb => ([white, red, green, blue], 4),
        };
        FloatOutBoyLedPhysicalChannels { bytes, len }
    }

    fn scaled_and_blended(self, target: Self, brightness: Ratio, blend: Ratio) -> Self {
        let brightness = brightness.as_ratio();
        let blend = blend.as_ratio();
        if blend <= 0.0 {
            return self;
        }

        let channels = core::array::from_fn(|index| {
            let target = target.channels.get(index).copied().unwrap_or_default();
            let scaled =
                crate::wire::saturating_trunc_f32_to_u8(f32::from(target) * brightness + 0.5);
            if blend >= 1.0 {
                return scaled;
            }
            let original = self.channels.get(index).copied().unwrap_or_default();
            crate::wire::saturating_trunc_f32_to_u8(
                f32::from(scaled) * blend + f32::from(original) * (1.0 - blend),
            )
        });
        Self { channels }
    }

    fn blend(first: Self, second: Self, blend: f32) -> Self {
        if blend <= 0.0 {
            return first;
        }
        if blend >= 1.0 {
            return second;
        }
        let first_weight = 1.0 - blend;
        let channels = core::array::from_fn(|index| {
            let first = first.channels.get(index).copied().unwrap_or_default();
            let second = second.channels.get(index).copied().unwrap_or_default();
            crate::wire::saturating_trunc_f32_to_u8(
                f32::from(first) * first_weight + f32::from(second) * blend,
            )
        });
        Self { channels }
    }
}

fn refloat_led_gamma(channel: u8) -> u8 {
    let channel = u16::from(channel);
    channel
        .checked_mul(channel)
        .and_then(|square| square.checked_add(channel))
        .and_then(|value| u8::try_from(value / 256).ok())
        .unwrap_or_default()
}

/// Float Out Boy LED animation mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedAnimationMode {
    /// Solid color.
    Solid = 0,
    /// Fade between colors.
    Fade = 1,
    /// Pulse between colors.
    Pulse = 2,
    /// Strobe between colors.
    Strobe = 3,
    /// Knight-rider sweep.
    KnightRider = 4,
    /// Alternating red/blue style animation.
    Felony = 5,
    /// Cycle rainbow colors.
    RainbowCycle = 6,
    /// Fade rainbow colors.
    RainbowFade = 7,
    /// Roll rainbow colors.
    RainbowRoll = 8,
}

impl FloatOutBoyLedAnimationMode {
    /// Return the Float Out Boy `v1.2.1` LED animation mode ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

/// Float Out Boy LED transition mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedTransition {
    /// Fade directly to the target bar.
    Fade = 0,
    /// Fade out, then fade in.
    FadeOutIn = 1,
    /// Cipher transition.
    Cipher = 2,
    /// Monochrome cipher transition.
    MonoCipher = 3,
}

impl FloatOutBoyLedTransition {
    /// Return the Float Out Boy `v1.2.1` LED transition ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
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
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyLedStripOrder {
    /// No strip is assigned.
    None = 0,
    /// First LED strip.
    First = 1,
    /// Second LED strip.
    Second = 2,
    /// Third LED strip.
    Third = 3,
}

impl FloatOutBoyLedStripOrder {
    /// Return the Float Out Boy `v1.2.1` LED strip order ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the firmware wire value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
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

// Refloat stores each strip length in one byte.
const MAX_LED_STRIP_PIXELS: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyHeadlightsState {
    Off,
    TransitioningOn,
    On,
    TransitioningOff,
}

/// Inputs copied into one pure 30 Hz LED state update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedUpdate {
    /// Current package run state.
    pub run_state: crate::FloatOutBoyRunState,
    /// Current package operating mode.
    pub mode: crate::FloatOutBoyMode,
    /// Whether darkride is active.
    pub darkride: bool,
    /// Current decoded footpad state.
    pub footpad: crate::FloatOutBoyFootpadState,
    /// Current board pitch in degrees.
    pub pitch_degrees: f32,
    /// Current motor distance.
    pub distance: f32,
}

/// Pure 30 Hz state for Refloat's lifted-board, footpad, and direction decisions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedDynamics {
    run_state: crate::FloatOutBoyRunState,
    board_is_upright: bool,
    on_off_fade: f32,
    left_sensor: f32,
    right_sensor: f32,
    headlights_state: FloatOutBoyHeadlightsState,
    headlights_split: f32,
    direction_forward: bool,
    direction_split: f32,
    split_distance: f32,
}

impl FloatOutBoyLedDynamics {
    /// Build source-compatible LED state before the first non-startup update.
    #[must_use]
    pub const fn new(distance: f32) -> Self {
        Self {
            run_state: crate::FloatOutBoyRunState::Startup,
            board_is_upright: false,
            on_off_fade: 0.0,
            left_sensor: 0.0,
            right_sensor: 0.0,
            headlights_state: FloatOutBoyHeadlightsState::Off,
            headlights_split: 1.0,
            direction_forward: true,
            direction_split: 1.0,
            split_distance: distance,
        }
    }

    /// Advance the pure renderer decisions by one source-rate 30 Hz tick.
    #[expect(
        clippy::too_many_lines,
        reason = "the ordered Refloat state update stays contiguous for source comparison"
    )]
    pub fn update(&mut self, config: FloatOutBoyLedsConfig, input: FloatOutBoyLedUpdate) {
        let FloatOutBoyLedUpdate {
            run_state,
            mode,
            darkride,
            footpad,
            pitch_degrees,
            distance,
        } = input;
        if matches!(run_state, crate::FloatOutBoyRunState::Startup) {
            return;
        }

        self.on_off_fade = rate_limit(
            self.on_off_fade,
            f32::from(u8::from(config.is_enabled())),
            3.0 / 30.0,
        );

        if !self.board_is_upright && pitch_degrees > 60.0 {
            self.board_is_upright = true;
        } else if self.board_is_upright && pitch_degrees < 50.0 {
            self.board_is_upright = false;
        }

        if run_state != self.run_state {
            if matches!(self.run_state, crate::FloatOutBoyRunState::Disabled) {
                self.on_off_fade = 0.0;
            }
            if matches!(run_state, crate::FloatOutBoyRunState::Running)
                && !matches!(
                    self.headlights_state,
                    FloatOutBoyHeadlightsState::TransitioningOn
                        | FloatOutBoyHeadlightsState::TransitioningOff
                )
            {
                self.direction_forward = pitch_degrees >= 0.0;
                self.direction_split = if self.direction_forward { 1.0 } else { -1.0 };
            } else if matches!(run_state, crate::FloatOutBoyRunState::Disabled) {
                self.on_off_fade = 0.0;
            }
        }
        self.run_state = run_state;

        let running = matches!(run_state, crate::FloatOutBoyRunState::Running);
        let (left, right) = if config.status().shows_sensors_while_running() || !running {
            (
                (!running
                    && matches!(
                        footpad,
                        crate::FloatOutBoyFootpadState::Left | crate::FloatOutBoyFootpadState::Both
                    ))
                    || matches!(footpad, crate::FloatOutBoyFootpadState::Left),
                (!running
                    && matches!(
                        footpad,
                        crate::FloatOutBoyFootpadState::Right
                            | crate::FloatOutBoyFootpadState::Both
                    ))
                    || matches!(footpad, crate::FloatOutBoyFootpadState::Right),
            )
        } else {
            (false, false)
        };
        self.left_sensor = rate_limit(self.left_sensor, f32::from(u8::from(left)), 10.0 / 30.0);
        self.right_sensor = rate_limit(self.right_sensor, f32::from(u8::from(right)), 10.0 / 30.0);

        let headlights_should = matches!(run_state, crate::FloatOutBoyRunState::Running)
            && !matches!(mode, crate::FloatOutBoyMode::Flywheel)
            && config.are_headlights_on();
        let headlights_on = matches!(self.headlights_state, FloatOutBoyHeadlightsState::On);
        let transitioning = matches!(
            self.headlights_state,
            FloatOutBoyHeadlightsState::TransitioningOn
                | FloatOutBoyHeadlightsState::TransitioningOff
        );
        if headlights_should != headlights_on && !transitioning {
            self.headlights_split = -1.0;
            self.headlights_state = if headlights_should {
                FloatOutBoyHeadlightsState::TransitioningOn
            } else {
                FloatOutBoyHeadlightsState::TransitioningOff
            };
        } else if transitioning {
            let direction = if headlights_should == headlights_on {
                -1.0
            } else {
                1.0
            };
            self.headlights_split =
                (self.headlights_split + direction * 2.0 / 30.0).clamp(-1.0, 1.0);
            if self.headlights_split >= 1.0
                || (headlights_should == headlights_on && self.headlights_split <= -1.0)
            {
                self.headlights_state = if headlights_should {
                    FloatOutBoyHeadlightsState::On
                } else {
                    FloatOutBoyHeadlightsState::Off
                };
                self.split_distance = distance;
            }
        }

        if matches!(run_state, crate::FloatOutBoyRunState::Running)
            && !matches!(
                self.headlights_state,
                FloatOutBoyHeadlightsState::TransitioningOn
                    | FloatOutBoyHeadlightsState::TransitioningOff
            )
        {
            let distance_change = if darkride {
                self.split_distance - distance
            } else {
                distance - self.split_distance
            };
            self.direction_split = (self.direction_split + distance_change * 2.0).clamp(-1.0, 1.0);
            self.split_distance = distance;
            if self.direction_split >= 1.0 {
                self.direction_forward = true;
            } else if self.direction_split <= -1.0 {
                self.direction_forward = false;
            }
        }
    }

    /// Return the global renderer fade.
    #[must_use]
    pub fn on_off_fade(self) -> Ratio {
        Ratio::clamped(self.on_off_fade)
    }

    /// Return the left/right footpad indicator fades.
    #[must_use]
    pub fn sensor_fades(self) -> (Ratio, Ratio) {
        (
            Ratio::clamped(self.left_sensor),
            Ratio::clamped(self.right_sensor),
        )
    }

    /// Return whether the lifted-board hysteresis is on its upright side.
    #[must_use]
    pub const fn is_board_upright(self) -> bool {
        self.board_is_upright
    }

    /// Return the current headlight transition split and settled state.
    #[must_use]
    pub const fn headlights(self) -> (f32, bool, bool) {
        (
            self.headlights_split,
            matches!(self.headlights_state, FloatOutBoyHeadlightsState::On),
            matches!(
                self.headlights_state,
                FloatOutBoyHeadlightsState::TransitioningOn
                    | FloatOutBoyHeadlightsState::TransitioningOff
            ),
        )
    }

    /// Return the current travel-direction split and settled direction.
    #[must_use]
    pub const fn direction(self) -> (f32, bool) {
        (self.direction_split, self.direction_forward)
    }
}

fn rate_limit(value: f32, target: f32, step: f32) -> f32 {
    if (target - value).abs() < step {
        target
    } else if target > value {
        value + step
    } else {
        value - step
    }
}

/// Allocation-free pixels for one configured internal LED strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatOutBoyLedStripFrame {
    config: FloatOutBoyLedStripConfig,
    pixels: [FloatOutBoyLedPixel; MAX_LED_STRIP_PIXELS],
}

/// Refloat status-bar progress source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyStatusProgress {
    /// Battery progress, with a red low end.
    Battery,
    /// Duty progress, with a red high end.
    Duty,
}

/// Shared brightness and blend inputs for one LED overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedOverlay {
    strip_brightness: Ratio,
    on_off_fade: Ratio,
    blend: Ratio,
}

impl FloatOutBoyLedOverlay {
    /// Build checked overlay inputs.
    #[must_use]
    pub const fn new(strip_brightness: Ratio, on_off_fade: Ratio, blend: Ratio) -> Self {
        Self {
            strip_brightness,
            on_off_fade,
            blend,
        }
    }
}

impl FloatOutBoyLedStripFrame {
    /// Build a cleared frame for one strip configuration.
    #[must_use]
    pub const fn new(config: FloatOutBoyLedStripConfig) -> Self {
        Self {
            config,
            pixels: [FloatOutBoyLedPixel { channels: [0; 4] }; MAX_LED_STRIP_PIXELS],
        }
    }

    /// Set one logical renderer pixel when it is inside the configured strip.
    pub fn set_logical_pixel(&mut self, index: usize, pixel: FloatOutBoyLedPixel) -> bool {
        if index >= usize::from(self.config.count()) {
            return false;
        }
        let Some(target) = self.pixels.get_mut(index) else {
            return false;
        };
        *target = pixel;
        true
    }

    /// Return one pixel in physical strip order.
    #[must_use]
    pub fn physical_pixel(&self, index: usize) -> Option<FloatOutBoyLedPixel> {
        let len = usize::from(self.config.count());
        if index >= len {
            return None;
        }
        let logical_index = if self.config.is_reversed() {
            len.checked_sub(index)?.checked_sub(1)?
        } else {
            index
        };
        self.pixels.get(logical_index).copied()
    }

    /// Render a solid bar with Refloat's brightness and blend ordering.
    pub fn render_solid(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, blend: Ratio) {
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());
        let target = FloatOutBoyLedPixel::from_named(bar.primary_color());
        self.render_target(target, brightness, blend);
    }

    /// Paint Refloat's left/right footpad indicator over this strip.
    pub fn render_footpads(
        &mut self,
        left: Ratio,
        right: Ratio,
        reverse_roles: bool,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let len = usize::from(self.config.count());
        if len == 0 {
            return;
        }
        let offset = len
            .saturating_add(1)
            .checked_div(2)
            .unwrap_or_default()
            .saturating_sub(1);
        let right_offset = len.saturating_sub(offset).saturating_sub(1);
        let (left, right) = if reverse_roles {
            (right.as_ratio(), left.as_ratio())
        } else {
            (left.as_ratio(), right.as_ratio())
        };
        let blend = Ratio::clamped(overlay.blend.as_ratio().min(left.max(right)));
        let color = FloatOutBoyLedPixel {
            channels: [0, 0xc0, 0xff, 0],
        };

        for index in 0..len {
            let mut dim = 0.0_f32;
            if left > 0.0 && index <= offset {
                dim = if index == offset { 0.6 * left } else { left };
            }
            if right > 0.0 && index >= right_offset {
                let right_dim = if index == right_offset {
                    0.6 * right
                } else {
                    right
                };
                dim = dim.max(right_dim);
            }
            let brightness = Ratio::clamped(
                overlay.strip_brightness.as_ratio() * dim * overlay.on_off_fade.as_ratio(),
            );
            self.render_pixel_blended(index, color, brightness, blend);
        }
    }

    /// Paint Refloat's battery or duty progress display over this strip.
    pub fn render_status_progress(
        &mut self,
        value: f32,
        kind: FloatOutBoyStatusProgress,
        red_percentage: Ratio,
        reverse: bool,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let len = usize::from(self.config.count());
        let len_float = f32::from(u16::try_from(len).unwrap_or_default());
        let progress = len_float * value;
        let offset = usize::from(crate::wire::saturating_trunc_f32_to_u8(progress));
        let remaining = (progress - vescpkg_rs::floor(progress)) * 0.7;
        let red_count = usize::from(crate::wire::saturating_trunc_f32_to_u8(vescpkg_rs::round(
            len_float * red_percentage.as_ratio(),
        )));
        let red_offset = len.saturating_sub(red_count);
        let red = FloatOutBoyLedPixel {
            channels: [0xff, 0x38, 0x28, 0],
        };
        let base = match kind {
            FloatOutBoyStatusProgress::Battery => FloatOutBoyLedPixel {
                channels: [0x90, 0x90, 0x90, 0],
            },
            FloatOutBoyStatusProgress::Duty => FloatOutBoyLedPixel {
                channels: [0xff, 0xb0, 0x30, 0],
            },
        };
        let low_battery = matches!(kind, FloatOutBoyStatusProgress::Battery)
            && -vescpkg_rs::floor(-progress)
                <= f32::from(u16::try_from(red_count).unwrap_or_default());

        for index in 0..len {
            let (target, dim) = if index <= offset {
                let target = if (matches!(kind, FloatOutBoyStatusProgress::Duty)
                    && index >= red_offset)
                    || low_battery
                {
                    red
                } else {
                    base
                };
                (target, if index == offset { remaining } else { 1.0 })
            } else {
                (FloatOutBoyLedPixel::default(), 1.0)
            };
            let physical_index = if reverse {
                len.saturating_sub(index).saturating_sub(1)
            } else {
                index
            };
            let brightness = Ratio::clamped(
                overlay.strip_brightness.as_ratio() * dim * overlay.on_off_fade.as_ratio(),
            );
            self.render_pixel_blended(physical_index, target, brightness, overlay.blend);
        }
    }

    /// Paint Refloat's disabled red pulse.
    pub fn render_disabled(&mut self, strip_brightness: Ratio, on_off_fade: Ratio, time: f32) {
        let len = usize::from(self.config.count());
        let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
            return;
        };
        let len_float = f32::from(len_u16);
        let time = time / 2.0;
        let progress = refloat_cosine_progress(time);
        let center = len_float / 3.0;
        let length = len_float / 2.0 - center;
        let offset = length * (1.0 - progress);
        let feather = len_float / 4.0;
        let ratio = center / length;
        let fade = if time < ratio { time / ratio } else { 1.0 };
        let red = FloatOutBoyLedPixel {
            channels: [0xff, 0, 0, 0],
        };
        let brightness = Ratio::clamped(strip_brightness.as_ratio() * on_off_fade.as_ratio());

        for index in 0..len {
            let index_float = f32::from(u16::try_from(index).unwrap_or_default());
            let start = ((index_float - offset + 1.0) / feather).clamp(0.0, 1.0);
            let end = ((len_float - offset - index_float) / feather).clamp(0.0, 1.0);
            let target = FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel::default(),
                red,
                start.min(end) * fade,
            );
            self.render_pixel(index, target, brightness);
        }
    }

    /// Paint Refloat's status-confirm pulse over this strip.
    pub fn render_confirmation(
        &mut self,
        strip_brightness: Ratio,
        on_off_fade: Ratio,
        progress: f32,
    ) {
        let len = usize::from(self.config.count());
        let len_float = f32::from(u16::try_from(len).unwrap_or_default());
        let progress = progress.min(1.0);
        let blend_time = 0.06;
        let blend = if progress <= blend_time {
            progress / blend_time
        } else if progress >= 1.0 - blend_time {
            (1.0 - progress) / blend_time
        } else {
            1.0
        };
        let period = 1.0 - blend_time;
        let half_period = period * 0.5;
        let progress = (progress - blend_time).min(period);
        let mut pulse = if progress <= half_period {
            progress / half_period * 1.5
        } else {
            (period - progress) / half_period * 1.5
        };
        if pulse > 1.0 {
            pulse = 2.0 - pulse;
        }
        pulse *= pulse;

        let sides = len_float * 0.1;
        let center = len_float * 0.125;
        let length = len_float * 0.5 - sides - center;
        let offset = sides + length * (1.0 - pulse);
        let feather = len_float * 0.25;
        let confirm = FloatOutBoyLedPixel {
            channels: [0xa0, 0x40, 0xff, 0],
        };
        let brightness = Ratio::clamped(strip_brightness.as_ratio() * on_off_fade.as_ratio());

        for index in 0..len {
            let index_float = f32::from(u16::try_from(index).unwrap_or_default());
            let distance = if index_float < len_float * 0.5 {
                index_float - offset + 1.0
            } else {
                len_float - offset - index_float
            };
            let target = FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel::default(),
                confirm,
                (distance / feather).clamp(0.0, 1.0),
            );
            self.render_pixel_blended(index, target, brightness, Ratio::clamped(blend));
        }
    }

    /// Render one currently implemented Refloat bar animation.
    pub fn render_bar(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        let time = time * bar.animation_speed().as_units();
        if matches!(bar.animation_mode(), FloatOutBoyLedAnimationMode::Felony) {
            self.render_felony(bar, on_off_fade, time);
            return;
        }
        if matches!(
            bar.animation_mode(),
            FloatOutBoyLedAnimationMode::RainbowCycle
                | FloatOutBoyLedAnimationMode::RainbowFade
                | FloatOutBoyLedAnimationMode::RainbowRoll
        ) {
            self.render_rainbow(bar, on_off_fade, time);
            return;
        }
        if matches!(bar.animation_mode(), FloatOutBoyLedAnimationMode::Pulse) {
            self.render_pulse(bar, on_off_fade, time);
            return;
        }
        if matches!(
            bar.animation_mode(),
            FloatOutBoyLedAnimationMode::KnightRider
        ) {
            self.render_knight_rider(bar, on_off_fade, time);
            return;
        }
        let target = match bar.animation_mode() {
            FloatOutBoyLedAnimationMode::Solid => {
                FloatOutBoyLedPixel::from_named(bar.primary_color())
            }
            FloatOutBoyLedAnimationMode::Fade => FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel::from_named(bar.secondary_color()),
                FloatOutBoyLedPixel::from_named(bar.primary_color()),
                refloat_cosine_progress(time),
            ),
            FloatOutBoyLedAnimationMode::Strobe => {
                let color = if vescpkg_rs::remainder(time, 2.0) >= 1.0 {
                    bar.secondary_color()
                } else {
                    bar.primary_color()
                };
                FloatOutBoyLedPixel::from_named(color)
            }
            FloatOutBoyLedAnimationMode::Pulse
            | FloatOutBoyLedAnimationMode::KnightRider
            | FloatOutBoyLedAnimationMode::Felony
            | FloatOutBoyLedAnimationMode::RainbowCycle
            | FloatOutBoyLedAnimationMode::RainbowFade
            | FloatOutBoyLedAnimationMode::RainbowRoll => return,
        };
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());
        self.render_target(target, brightness, Ratio::from_ratio_const(1.0));
    }

    /// Render one Refloat transition over the existing source frame.
    pub fn render_transition(
        &mut self,
        transition: FloatOutBoyLedTransition,
        progress: f32,
        seed: u32,
        from_bar: FloatOutBoyLedBarConfig,
        to_bar: FloatOutBoyLedBarConfig,
        on_off_fade: Ratio,
    ) {
        match transition {
            FloatOutBoyLedTransition::Fade => {
                let blend = Ratio::clamped(progress.midpoint(1.0));
                let brightness = Ratio::clamped(
                    (from_bar.brightness().as_ratio()
                        + (to_bar.brightness().as_ratio() - from_bar.brightness().as_ratio())
                            * blend.as_ratio())
                        * on_off_fade.as_ratio(),
                );
                self.render_target(transition_target(to_bar), brightness, blend);
            }
            FloatOutBoyLedTransition::FadeOutIn => {
                self.render_fade_out_in(progress, to_bar, from_bar.brightness(), on_off_fade);
            }
            FloatOutBoyLedTransition::Cipher | FloatOutBoyLedTransition::MonoCipher => {
                self.render_cipher(transition, progress, seed, from_bar, to_bar, on_off_fade);
            }
        }
    }

    fn render_fade_out_in(
        &mut self,
        progress: f32,
        to_bar: FloatOutBoyLedBarConfig,
        from_brightness: Ratio,
        on_off_fade: Ratio,
    ) {
        if progress <= 0.0 {
            let blend = Ratio::clamped(progress + 1.0);
            let brightness = Ratio::clamped(
                (from_brightness.as_ratio()
                    + (to_bar.brightness().as_ratio() - from_brightness.as_ratio())
                        * blend.as_ratio())
                    * on_off_fade.as_ratio(),
            );
            self.render_target(FloatOutBoyLedPixel::default(), brightness, blend);
            return;
        }

        let target = FloatOutBoyLedPixel::blend(
            FloatOutBoyLedPixel::default(),
            transition_target(to_bar),
            progress,
        );
        let brightness = Ratio::clamped(
            (from_brightness.as_ratio()
                + (to_bar.brightness().as_ratio() - from_brightness.as_ratio()) * progress)
                * on_off_fade.as_ratio(),
        );
        self.render_target(target, brightness, Ratio::from_ratio_const(1.0));
    }

    fn render_cipher(
        &mut self,
        transition: FloatOutBoyLedTransition,
        progress: f32,
        seed: u32,
        from_bar: FloatOutBoyLedBarConfig,
        to_bar: FloatOutBoyLedBarConfig,
        on_off_fade: Ratio,
    ) {
        const MAX_CIPHER_STRIP_PIXELS: usize = 60;
        let len = usize::from(self.config.count());
        if len == 0 || len > MAX_CIPHER_STRIP_PIXELS {
            return;
        }

        let mut map = [0_u8; MAX_CIPHER_STRIP_PIXELS * 2];
        for index in 0..len {
            let value = u8::try_from(index).unwrap_or_default();
            if let Some(first) = map.get_mut(index) {
                *first = value;
            }
            if let Some(second) = map.get_mut(index.saturating_add(len)) {
                *second = value;
            }
        }
        refloat_sattolo_shuffle(seed, map.get_mut(..len).unwrap_or_default());
        refloat_sattolo_shuffle(
            seed,
            map.get_mut(len..len.saturating_mul(2)).unwrap_or_default(),
        );

        let len_i16 = i16::try_from(len).unwrap_or_default();
        let stop = crate::wire::saturating_trunc_f32_to_i16(progress * f32::from(len_i16));
        let target = transition_target(to_bar);
        let mid_brightness = Ratio::clamped(
            from_bar
                .brightness()
                .as_ratio()
                .midpoint(to_bar.brightness().as_ratio())
                * on_off_fade.as_ratio(),
        );
        let target_brightness =
            Ratio::clamped(to_bar.brightness().as_ratio() * on_off_fade.as_ratio());

        for index in 1_i16.saturating_sub(len_i16)..=stop {
            if index <= 0 {
                let source_index = usize::try_from(index.saturating_neg()).unwrap_or_default();
                let target_index = usize::from(map.get(source_index).copied().unwrap_or_default());
                let random_seed =
                    u32::try_from(source_index.saturating_add(target_index)).unwrap_or_default();
                let random = u8::try_from(refloat_random(random_seed) % 256).unwrap_or_default();
                let pixel = if refloat_random(random_seed.wrapping_add(17)) % 8 < 3 {
                    FloatOutBoyLedPixel::default()
                } else if matches!(transition, FloatOutBoyLedTransition::MonoCipher) {
                    FloatOutBoyLedPixel::blend(
                        FloatOutBoyLedPixel::from_named(from_bar.primary_color()),
                        target,
                        f32::from(random) / 256.0,
                    )
                } else {
                    let white = u8::try_from(
                        (refloat_random(random_seed.wrapping_add(23)) % 128).wrapping_add(80),
                    )
                    .unwrap_or_default();
                    let mut channels = refloat_hue_to_pixel(random).channels();
                    for channel in channels.get_mut(..3).unwrap_or_default() {
                        *channel |= white;
                    }
                    FloatOutBoyLedPixel { channels }
                };
                self.render_pixel(target_index, pixel, mid_brightness);
            } else {
                let offset = usize::try_from(index.saturating_sub(1)).unwrap_or_default();
                let target_index = usize::from(
                    map.get(len.saturating_add(offset))
                        .copied()
                        .unwrap_or_default(),
                );
                self.render_pixel(target_index, target, target_brightness);
            }
        }
    }

    fn render_pixel(&mut self, index: usize, target: FloatOutBoyLedPixel, brightness: Ratio) {
        self.render_pixel_blended(index, target, brightness, Ratio::from_ratio_const(1.0));
    }

    fn render_pixel_blended(
        &mut self,
        index: usize,
        target: FloatOutBoyLedPixel,
        brightness: Ratio,
        blend: Ratio,
    ) {
        if let Some(pixel) = self.pixels.get_mut(index) {
            *pixel = pixel.scaled_and_blended(target, brightness, blend);
        }
    }

    fn render_pulse(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        let len = usize::from(self.config.count());
        let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
            return;
        };
        let len_float = f32::from(len_u16);
        let progress = refloat_cosine_progress(time);
        let center = len_float / 5.0;
        let length = len_float / 2.0 - center;
        let offset = length * (1.0 - progress);
        let feather = len_float / 4.0;
        let ratio = center / length;
        let fade = if time < ratio { time / ratio } else { 1.0 };
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color());
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color());
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());

        for (index, pixel) in self
            .pixels
            .get_mut(..len)
            .unwrap_or_default()
            .iter_mut()
            .enumerate()
        {
            let index = f32::from(u16::try_from(index).unwrap_or_default());
            let distance_from_start = index - offset + 1.0;
            let distance_from_end = len_float - offset - index;
            let start = (distance_from_start / feather).clamp(0.0, 1.0);
            let end = (distance_from_end / feather).clamp(0.0, 1.0);
            let target = FloatOutBoyLedPixel::blend(secondary, primary, start.min(end) * fade);
            *pixel = pixel.scaled_and_blended(target, brightness, Ratio::from_ratio_const(1.0));
        }
    }

    fn render_knight_rider(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        let len = usize::from(self.config.count());
        let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
            return;
        };
        let len_float = f32::from(len_u16);
        let tail = f32::from((len_u16 / 3).saturating_add(1));
        let time = time * 0.7;
        let backlight = if time > 0.3 { 0.08 } else { 0.0 };
        let first = len_float * vescpkg_rs::remainder(time, 2.0) - 0.5 * len_float - 1.0;
        let second = 1.5 * len_float - len_float * vescpkg_rs::remainder(time - 1.0, 2.0);
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color());
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color());
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());

        for (index, pixel) in self
            .pixels
            .get_mut(..len)
            .unwrap_or_default()
            .iter_mut()
            .enumerate()
        {
            let index = f32::from(u16::try_from(index).unwrap_or_default());
            let mut first_blend = backlight;
            let first_distance = (first - index).abs();
            if index <= first {
                if first_distance <= tail {
                    first_blend = (tail - first_distance) / tail;
                }
            } else if index < first + 1.0 {
                first_blend = first - vescpkg_rs::floor(first);
            }

            let mut second_blend = backlight;
            let second_distance = (second - index).abs();
            if index >= second {
                if second_distance <= tail {
                    second_blend = (tail - second_distance) / tail;
                }
            } else if index > second - 1.0 {
                second_blend = 1.0 - second + vescpkg_rs::floor(second);
            }

            let target =
                FloatOutBoyLedPixel::blend(secondary, primary, first_blend.max(second_blend));
            *pixel = pixel.scaled_and_blended(target, brightness, Ratio::from_ratio_const(1.0));
        }
    }

    fn render_felony(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        let phase = vescpkg_rs::remainder(time, 0.15);
        let len = usize::from(self.config.count());
        let stop = len / 2;
        let start = stop.saturating_add(len.checked_rem(2).unwrap_or_default());
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color());
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color());
        let black = FloatOutBoyLedPixel::default();
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());

        for (index, pixel) in self
            .pixels
            .get_mut(..len)
            .unwrap_or_default()
            .iter_mut()
            .enumerate()
        {
            let target = if phase < 0.05 {
                if index < stop { primary } else { black }
            } else if phase < 0.1 {
                if index >= start { secondary } else { black }
            } else if index < stop {
                secondary
            } else if index >= start {
                primary
            } else {
                black
            };
            *pixel = pixel.scaled_and_blended(target, brightness, Ratio::from_ratio_const(1.0));
        }
    }

    fn render_rainbow(&mut self, bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        let brightness = Ratio::clamped(bar.brightness().as_ratio() * on_off_fade.as_ratio());
        match bar.animation_mode() {
            FloatOutBoyLedAnimationMode::RainbowCycle => {
                let step = crate::wire::saturating_trunc_f32_to_u8(time * 10.0)
                    .checked_rem(10)
                    .unwrap_or_default();
                let hue = crate::wire::saturating_trunc_f32_to_u8(f32::from(step) * 25.5);
                self.render_target(
                    refloat_hue_to_pixel(hue),
                    brightness,
                    Ratio::from_ratio_const(1.0),
                );
            }
            FloatOutBoyLedAnimationMode::RainbowFade => {
                let hue = crate::wire::saturating_trunc_f32_to_u8(
                    vescpkg_rs::remainder(time, 1.0) * 255.0,
                );
                self.render_target(
                    refloat_hue_to_pixel(hue),
                    brightness,
                    Ratio::from_ratio_const(1.0),
                );
            }
            FloatOutBoyLedAnimationMode::RainbowRoll => {
                let len = usize::from(self.config.count());
                let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
                    return;
                };
                let offset = vescpkg_rs::remainder(time, 1.0) * 255.0;
                for (index, pixel) in self
                    .pixels
                    .get_mut(..len)
                    .unwrap_or_default()
                    .iter_mut()
                    .enumerate()
                {
                    let index = u16::try_from(index).unwrap_or_default();
                    let hue = crate::wire::saturating_trunc_f32_to_u8(
                        255.0 / f32::from(len_u16) * f32::from(index) + offset,
                    );
                    *pixel = pixel.scaled_and_blended(
                        refloat_hue_to_pixel(hue),
                        brightness,
                        Ratio::from_ratio_const(1.0),
                    );
                }
            }
            FloatOutBoyLedAnimationMode::Solid
            | FloatOutBoyLedAnimationMode::Fade
            | FloatOutBoyLedAnimationMode::Pulse
            | FloatOutBoyLedAnimationMode::Strobe
            | FloatOutBoyLedAnimationMode::KnightRider
            | FloatOutBoyLedAnimationMode::Felony => {}
        }
    }

    fn render_target(&mut self, target: FloatOutBoyLedPixel, brightness: Ratio, blend: Ratio) {
        let len = usize::from(self.config.count());
        for pixel in self.pixels.get_mut(..len).unwrap_or_default() {
            *pixel = pixel.scaled_and_blended(target, brightness, blend);
        }
    }
}

fn transition_target(bar: FloatOutBoyLedBarConfig) -> FloatOutBoyLedPixel {
    let color = if matches!(bar.animation_mode(), FloatOutBoyLedAnimationMode::Solid) {
        bar.primary_color()
    } else {
        bar.secondary_color()
    };
    FloatOutBoyLedPixel::from_named(color)
}

fn refloat_random(seed: u32) -> u32 {
    seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

fn refloat_sattolo_shuffle(seed: u32, values: &mut [u8]) {
    for index in (1..values.len()).rev() {
        let divisor = u32::try_from(index).unwrap_or(1);
        let target = usize::try_from(
            refloat_random(seed.wrapping_add(u32::try_from(index).unwrap_or_default()))
                .checked_rem(divisor)
                .unwrap_or_default(),
        )
        .unwrap_or_default();
        values.swap(index, target);
    }
}

fn refloat_cosine_progress(time: f32) -> f32 {
    let rounded = vescpkg_rs::round(time);
    let mut x = (time - rounded) * core::f32::consts::PI;
    x *= x;
    let cosine = 2.5 * x / (x + core::f32::consts::PI * core::f32::consts::PI);
    let rounded = crate::wire::saturating_trunc_f32_to_u32(rounded);
    if rounded.checked_rem(2) == Some(1) {
        1.0 - cosine
    } else {
        cosine
    }
}

fn refloat_hue_to_pixel(hue: u8) -> FloatOutBoyLedPixel {
    let normalized = f32::from(hue) / 255.0 * 3.0;
    let red = vescpkg_rs::remainder(normalized + 0.5, 3.0);
    let green = vescpkg_rs::remainder(normalized + 2.5, 3.0);
    let blue = vescpkg_rs::remainder(normalized + 1.5, 3.0);
    let tweak = |channel: f32, exponent: f32| {
        if channel < 1.0 {
            1.0 - vescpkg_rs::pow(1.0 - channel, exponent)
        } else if channel < 2.0 {
            1.0 + vescpkg_rs::pow(channel - 1.0, exponent)
        } else {
            0.0
        }
    };
    let channel =
        |value| crate::wire::saturating_trunc_f32_to_u8(refloat_cosine_progress(value) * 255.0);
    FloatOutBoyLedPixel {
        channels: [
            channel(tweak(red, 3.2)),
            channel(tweak(green, 2.4)),
            channel(tweak(blue, 2.2)),
            0,
        ],
    }
}

#[cfg(test)]
mod renderer_tests {
    use super::{FloatOutBoyLedColor, FloatOutBoyLedColorOrder, FloatOutBoyLedPixel};
    use vescpkg_rs::prelude::Ratio;

    #[test]
    fn named_led_colors_match_refloat_1_2_1_rgba_channels() {
        let cases = [
            (FloatOutBoyLedColor::Black, [0x00, 0x00, 0x00, 0x00]),
            (FloatOutBoyLedColor::WhiteFull, [0xff, 0xff, 0xff, 0xff]),
            (FloatOutBoyLedColor::WhiteRgb, [0xff, 0xff, 0xff, 0x00]),
            (FloatOutBoyLedColor::WhiteSingle, [0x00, 0x00, 0x00, 0xff]),
            (FloatOutBoyLedColor::Red, [0xff, 0x00, 0x00, 0x00]),
            (FloatOutBoyLedColor::Ferrari, [0xff, 0x38, 0x00, 0x00]),
            (FloatOutBoyLedColor::Flame, [0xff, 0x50, 0x00, 0x00]),
            (FloatOutBoyLedColor::Coral, [0xff, 0x60, 0x40, 0x00]),
            (FloatOutBoyLedColor::Sunset, [0xff, 0x78, 0x30, 0x00]),
            (FloatOutBoyLedColor::Sunrise, [0xff, 0x90, 0x40, 0x00]),
            (FloatOutBoyLedColor::Gold, [0xff, 0x80, 0x20, 0x00]),
            (FloatOutBoyLedColor::Orange, [0xff, 0x78, 0x00, 0x00]),
            (FloatOutBoyLedColor::Yellow, [0xff, 0xa0, 0x00, 0x00]),
            (FloatOutBoyLedColor::Banana, [0xff, 0xb0, 0x40, 0x00]),
            (FloatOutBoyLedColor::Lime, [0xff, 0xff, 0x00, 0x00]),
            (FloatOutBoyLedColor::Acid, [0xa0, 0xff, 0x00, 0x00]),
            (FloatOutBoyLedColor::Sage, [0xa0, 0xff, 0x50, 0x00]),
            (FloatOutBoyLedColor::Green, [0x00, 0xff, 0x00, 0x00]),
            (FloatOutBoyLedColor::Mint, [0x00, 0xff, 0x50, 0x00]),
            (FloatOutBoyLedColor::Tiffany, [0x00, 0xff, 0xc0, 0x00]),
            (FloatOutBoyLedColor::Cyan, [0x00, 0xff, 0xff, 0x00]),
            (FloatOutBoyLedColor::Steel, [0x90, 0xc0, 0xff, 0x00]),
            (FloatOutBoyLedColor::Sky, [0x70, 0xd0, 0xff, 0x00]),
            (FloatOutBoyLedColor::Azure, [0x00, 0xa0, 0xff, 0x00]),
            (FloatOutBoyLedColor::Sapphire, [0x00, 0x70, 0xff, 0x00]),
            (FloatOutBoyLedColor::Blue, [0x00, 0x00, 0xff, 0x00]),
            (FloatOutBoyLedColor::Violet, [0x80, 0x00, 0xff, 0x00]),
            (FloatOutBoyLedColor::Amethyst, [0xa0, 0x60, 0xff, 0x00]),
            (FloatOutBoyLedColor::Magenta, [0xff, 0x00, 0xff, 0x00]),
            (FloatOutBoyLedColor::Pink, [0xff, 0x00, 0xc0, 0x00]),
            (FloatOutBoyLedColor::Fuchsia, [0xff, 0x00, 0x70, 0x00]),
            (FloatOutBoyLedColor::Lavender, [0xff, 0x70, 0xa0, 0x00]),
        ];

        for (color, channels) in cases {
            assert_eq!(FloatOutBoyLedPixel::from_named(color).channels(), channels);
        }
    }

    #[test]
    fn physical_channels_apply_refloat_gamma_and_color_order() {
        let pixel = FloatOutBoyLedPixel {
            channels: [16, 64, 128, 255],
        };

        assert_eq!(
            pixel
                .physical_channels(FloatOutBoyLedColorOrder::Grb)
                .as_slice(),
            &[16, 1, 64]
        );
        assert_eq!(
            pixel
                .physical_channels(FloatOutBoyLedColorOrder::Grbw)
                .as_slice(),
            &[16, 1, 64, 255]
        );
        assert_eq!(
            pixel
                .physical_channels(FloatOutBoyLedColorOrder::Rgb)
                .as_slice(),
            &[1, 16, 64]
        );
        assert_eq!(
            pixel
                .physical_channels(FloatOutBoyLedColorOrder::Wrgb)
                .as_slice(),
            &[255, 1, 16, 64]
        );
    }

    #[test]
    fn physical_channels_match_refloat_gamma_for_every_input() {
        for channel in 0_u8..=u8::MAX {
            let pixel = FloatOutBoyLedPixel {
                channels: [channel; 4],
            };
            let widened = u16::from(channel);
            let expected = widened
                .checked_mul(widened)
                .and_then(|square| square.checked_add(widened))
                .and_then(|value| u8::try_from(value / 256).ok())
                .expect("gamma stays in u8");

            assert_eq!(
                pixel
                    .physical_channels(FloatOutBoyLedColorOrder::Wrgb)
                    .as_slice(),
                &[expected; 4]
            );
        }
    }

    #[test]
    fn strip_frame_maps_logical_pixels_to_checked_physical_order() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            3,
            FloatOutBoyLedColorOrder::Grb,
        )
        .with_reverse(true);
        let mut frame = super::FloatOutBoyLedStripFrame::new(config);
        let red = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red);
        let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);

        assert!(frame.set_logical_pixel(0, red));
        assert!(frame.set_logical_pixel(2, blue));
        assert!(!frame.set_logical_pixel(3, red));
        assert_eq!(frame.physical_pixel(0), Some(blue));
        assert_eq!(
            frame.physical_pixel(1),
            Some(FloatOutBoyLedPixel::default())
        );
        assert_eq!(frame.physical_pixel(2), Some(red));
        assert_eq!(frame.physical_pixel(3), None);
    }

    #[test]
    fn solid_bar_applies_brightness_on_off_fade_and_blend_like_refloat() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            2,
            FloatOutBoyLedColorOrder::Grb,
        );
        let mut frame = super::FloatOutBoyLedStripFrame::new(config);
        let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);
        assert!(frame.set_logical_pixel(0, blue));
        assert!(frame.set_logical_pixel(1, blue));
        let bar = super::FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(0.5),
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Black,
            super::FloatOutBoyLedAnimationMode::Solid,
            super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
        );

        frame.render_solid(
            bar,
            Ratio::from_ratio_const(0.5),
            Ratio::from_ratio_const(0.5),
        );

        let expected = FloatOutBoyLedPixel {
            channels: [32, 0, 127, 0],
        };
        assert_eq!(frame.physical_pixel(0), Some(expected));
        assert_eq!(frame.physical_pixel(1), Some(expected));
    }

    #[test]
    fn fade_and_strobe_match_refloat_time_boundaries() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            1,
            FloatOutBoyLedColorOrder::Grb,
        );
        let fade = super::FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(1.0),
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::Fade,
            super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
        );
        let strobe = super::FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(1.0),
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::Strobe,
            super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
        );

        let mut frame = super::FloatOutBoyLedStripFrame::new(config);
        frame.render_bar(fade, Ratio::from_ratio_const(1.0), 0.0);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue))
        );
        frame.render_bar(fade, Ratio::from_ratio_const(1.0), 0.5);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel {
                channels: [127, 0, 127, 0]
            })
        );
        frame.render_bar(fade, Ratio::from_ratio_const(1.0), 1.0);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
        );

        frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 0.999);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
        );
        frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 1.0);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue))
        );
        frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 2.0);
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
        );
    }

    #[test]
    fn felony_preserves_odd_center_blackout_across_all_three_phases() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            5,
            FloatOutBoyLedColorOrder::Grb,
        );
        let bar = super::FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(1.0),
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::Felony,
            super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
        );
        let red = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red);
        let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);
        let black = FloatOutBoyLedPixel::default();
        let mut frame = super::FloatOutBoyLedStripFrame::new(config);

        frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.0);
        assert_eq!(
            core::array::from_fn(|index| frame.physical_pixel(index)),
            [Some(red), Some(red), Some(black), Some(black), Some(black)]
        );
        frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.051);
        assert_eq!(
            core::array::from_fn(|index| frame.physical_pixel(index)),
            [
                Some(black),
                Some(black),
                Some(black),
                Some(blue),
                Some(blue)
            ]
        );
        frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.101);
        assert_eq!(
            core::array::from_fn(|index| frame.physical_pixel(index)),
            [Some(blue), Some(blue), Some(black), Some(red), Some(red)]
        );
    }

    #[test]
    fn rainbow_modes_match_refloat_hue_steps_and_strip_offsets() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            4,
            FloatOutBoyLedColorOrder::Grb,
        );
        let bar = |mode| {
            super::FloatOutBoyLedBarConfig::new(
                Ratio::from_ratio_const(1.0),
                FloatOutBoyLedColor::Black,
                FloatOutBoyLedColor::Black,
                mode,
                super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
            )
        };
        let mut frame = super::FloatOutBoyLedStripFrame::new(config);

        frame.render_bar(
            bar(super::FloatOutBoyLedAnimationMode::RainbowCycle),
            Ratio::from_ratio_const(1.0),
            0.9,
        );
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel {
                channels: [0x7e, 0x00, 0xfe, 0]
            })
        );

        frame.render_bar(
            bar(super::FloatOutBoyLedAnimationMode::RainbowFade),
            Ratio::from_ratio_const(1.0),
            0.25,
        );
        assert_eq!(
            frame.physical_pixel(0),
            Some(FloatOutBoyLedPixel {
                channels: [0xfe, 0x79, 0x00, 0]
            })
        );

        frame.render_bar(
            bar(super::FloatOutBoyLedAnimationMode::RainbowRoll),
            Ratio::from_ratio_const(1.0),
            0.0,
        );
        assert_eq!(
            core::array::from_fn(|index| frame.physical_pixel(index)),
            [
                Some(FloatOutBoyLedPixel {
                    channels: [0xf7, 0x00, 0xe2, 0]
                }),
                Some(FloatOutBoyLedPixel {
                    channels: [0xfe, 0x79, 0x00, 0]
                }),
                Some(FloatOutBoyLedPixel {
                    channels: [0x00, 0xff, 0x00, 0]
                }),
                Some(FloatOutBoyLedPixel {
                    channels: [0x00, 0x80, 0xfd, 0]
                }),
            ]
        );
    }

    #[test]
    fn pulse_and_knight_rider_match_refloat_spatial_frames() {
        let bar = |mode| {
            super::FloatOutBoyLedBarConfig::new(
                Ratio::from_ratio_const(1.0),
                FloatOutBoyLedColor::Red,
                FloatOutBoyLedColor::Blue,
                mode,
                super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
            )
        };
        let config = |count| {
            super::FloatOutBoyLedStripConfig::new(
                super::FloatOutBoyLedStripOrder::First,
                count,
                FloatOutBoyLedColorOrder::Grb,
            )
        };

        let mut pulse = super::FloatOutBoyLedStripFrame::new(config(5));
        pulse.render_bar(
            bar(super::FloatOutBoyLedAnimationMode::Pulse),
            Ratio::from_ratio_const(1.0),
            0.5,
        );
        assert_eq!(
            core::array::from_fn(|index| {
                pulse
                    .physical_pixel(index)
                    .map(super::FloatOutBoyLedPixel::channels)
            }),
            [
                Some([0x26, 0, 0xd8, 0]),
                Some([0xbf, 0, 0x3f, 0]),
                Some([0xbf, 0, 0x3f, 0]),
                Some([0xbf, 0, 0x3f, 0]),
                Some([0x26, 0, 0xd8, 0]),
            ]
        );

        let mut knight_rider = super::FloatOutBoyLedStripFrame::new(config(6));
        knight_rider.render_bar(
            bar(super::FloatOutBoyLedAnimationMode::KnightRider),
            Ratio::from_ratio_const(1.0),
            1.5,
        );
        assert_eq!(
            core::array::from_fn(|index| {
                knight_rider
                    .physical_pixel(index)
                    .map(super::FloatOutBoyLedPixel::channels)
            }),
            [
                Some([0x3b, 0, 0xc3, 0]),
                Some([0x90, 0, 0x6e, 0]),
                Some([0xe5, 0, 0x19, 0]),
                Some([0x4c, 0, 0xb2, 0]),
                Some([0x14, 0, 0xea, 0]),
                Some([0x14, 0, 0xea, 0]),
            ]
        );
    }

    #[test]
    fn all_transition_modes_match_refloat_frames() {
        let bar = |brightness, primary, secondary, mode| {
            super::FloatOutBoyLedBarConfig::new(
                Ratio::from_ratio_const(brightness),
                primary,
                secondary,
                mode,
                super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
            )
        };
        let from = bar(
            0.5,
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::Solid,
        );
        let to = bar(
            1.0,
            FloatOutBoyLedColor::Green,
            FloatOutBoyLedColor::Yellow,
            super::FloatOutBoyLedAnimationMode::Fade,
        );
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            4,
            FloatOutBoyLedColorOrder::Grb,
        );

        let cases = [
            (
                super::FloatOutBoyLedTransition::Fade,
                -0.5,
                7,
                [[0x87, 0x19, 0, 0]; 4],
            ),
            (
                super::FloatOutBoyLedTransition::FadeOutIn,
                0.5,
                7,
                [[0x5f, 0x3c, 0, 0]; 4],
            ),
            (
                super::FloatOutBoyLedTransition::Cipher,
                0.0,
                7,
                [
                    [0x61, 0xbf, 0x6d, 0],
                    [0, 0, 0, 0],
                    [0x61, 0xbf, 0x6d, 0],
                    [0x74, 0xbc, 0x8c, 0],
                ],
            ),
            (
                super::FloatOutBoyLedTransition::MonoCipher,
                0.0,
                7,
                [
                    [0xbf, 0x3e, 0, 0],
                    [0, 0, 0, 0],
                    [0xbf, 0x3e, 0, 0],
                    [0xbf, 0x4b, 0, 0],
                ],
            ),
        ];

        for (transition, progress, seed, expected) in cases {
            let mut frame = super::FloatOutBoyLedStripFrame::new(config);
            frame.render_solid(
                from,
                Ratio::from_ratio_const(1.0),
                Ratio::from_ratio_const(1.0),
            );
            frame.render_transition(
                transition,
                progress,
                seed,
                from,
                to,
                Ratio::from_ratio_const(1.0),
            );
            assert_eq!(
                core::array::from_fn(|index| {
                    frame
                        .physical_pixel(index)
                        .map(super::FloatOutBoyLedPixel::channels)
                        .unwrap_or_default()
                }),
                expected,
                "{transition:?}"
            );
        }
    }

    #[test]
    fn footpad_and_status_progress_pixels_match_refloat() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            5,
            FloatOutBoyLedColorOrder::Grb,
        );
        let channels = |frame: &super::FloatOutBoyLedStripFrame| {
            core::array::from_fn(|index| {
                frame
                    .physical_pixel(index)
                    .map(super::FloatOutBoyLedPixel::channels)
                    .unwrap_or_default()
            })
        };
        let overlay = super::FloatOutBoyLedOverlay::new(
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        );

        let mut footpads = super::FloatOutBoyLedStripFrame::new(config);
        footpads.render_footpads(
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(0.5),
            false,
            overlay,
        );
        assert_eq!(
            channels(&footpads),
            [
                [0, 0xc0, 0xff, 0],
                [0, 0xc0, 0xff, 0],
                [0, 0x73, 0x99, 0],
                [0, 0x60, 0x80, 0],
                [0, 0x60, 0x80, 0],
            ]
        );

        let mut battery = super::FloatOutBoyLedStripFrame::new(config);
        battery.render_status_progress(
            0.45,
            super::FloatOutBoyStatusProgress::Battery,
            Ratio::from_ratio_const(0.4),
            false,
            overlay,
        );
        assert_eq!(
            channels(&battery),
            [
                [0x90, 0x90, 0x90, 0],
                [0x90, 0x90, 0x90, 0],
                [0x19, 0x19, 0x19, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ]
        );

        let mut duty_reversed = super::FloatOutBoyLedStripFrame::new(config);
        duty_reversed.render_status_progress(
            0.8,
            super::FloatOutBoyStatusProgress::Duty,
            Ratio::from_ratio_const(0.4),
            true,
            overlay,
        );
        assert_eq!(
            channels(&duty_reversed),
            [
                [0, 0, 0, 0],
                [0xff, 0x38, 0x28, 0],
                [0xff, 0xb0, 0x30, 0],
                [0xff, 0xb0, 0x30, 0],
                [0xff, 0xb0, 0x30, 0],
            ]
        );
    }

    #[test]
    fn disabled_and_confirmation_pixels_match_refloat() {
        let config = super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            5,
            FloatOutBoyLedColorOrder::Grb,
        );
        let channels = |frame: &super::FloatOutBoyLedStripFrame| {
            core::array::from_fn(|index| {
                frame
                    .physical_pixel(index)
                    .map(super::FloatOutBoyLedPixel::channels)
                    .unwrap_or_default()
            })
        };

        let mut disabled = super::FloatOutBoyLedStripFrame::new(config);
        disabled.render_disabled(
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
            1.0,
        );
        assert_eq!(
            channels(&disabled),
            [
                [0x1d, 0, 0, 0],
                [0x3f, 0, 0, 0],
                [0x3f, 0, 0, 0],
                [0x3f, 0, 0, 0],
                [0x1d, 0, 0, 0],
            ]
        );

        let mut confirmation = super::FloatOutBoyLedStripFrame::new(config);
        confirmation.render_confirmation(
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
            0.5,
        );
        assert_eq!(
            channels(&confirmation),
            [
                [0, 0, 0, 0],
                [0x4e, 0x1f, 0x7d, 0],
                [0xa0, 0x40, 0xff, 0],
                [0x4e, 0x1f, 0x7d, 0],
                [0, 0, 0, 0],
            ]
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one sequential trace verifies the coupled Refloat state-machine boundaries"
    )]
    fn led_dynamics_match_refloat_rate_hysteresis_and_mode_gates() {
        macro_rules! update {
            ($state:expr, $config:expr, $run:expr, $mode:expr, $darkride:expr, $footpad:expr, $pitch:expr, $distance:expr $(,)?) => {
                $state.update(
                    $config,
                    super::FloatOutBoyLedUpdate {
                        run_state: $run,
                        mode: $mode,
                        darkride: $darkride,
                        footpad: $footpad,
                        pitch_degrees: $pitch,
                        distance: $distance,
                    },
                )
            };
        }
        let bar = super::FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(1.0),
            FloatOutBoyLedColor::WhiteRgb,
            FloatOutBoyLedColor::Black,
            super::FloatOutBoyLedAnimationMode::Solid,
            super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
        );
        let status = super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.9),
            Ratio::from_ratio_const(0.1),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(0.5),
        );
        let config = super::FloatOutBoyLedsConfig::new(bar, bar, bar, bar, status, bar)
            .enabled()
            .with_headlights_on();
        let mut dynamics = super::FloatOutBoyLedDynamics::new(0.0);

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Startup,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::Left,
            70.0,
            0.0,
        );
        assert_f32_eq!(dynamics.on_off_fade().as_ratio(), 0.0);
        assert!(!dynamics.is_board_upright());

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Ready,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::Left,
            61.0,
            0.0,
        );
        assert_f32_eq!(dynamics.on_off_fade().as_ratio(), 0.1);
        assert_f32_eq!(dynamics.sensor_fades().0.as_ratio(), 1.0 / 3.0);
        assert!(dynamics.is_board_upright());

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Ready,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            50.0,
            0.0,
        );
        assert!(dynamics.is_board_upright());
        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Ready,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            49.0,
            0.0,
        );
        assert!(!dynamics.is_board_upright());

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::Both,
            -1.0,
            0.0,
        );
        assert_eq!(dynamics.direction(), (-1.0, false));
        assert_f32_eq!(dynamics.sensor_fades().0.as_ratio(), 0.0);
        assert_eq!(dynamics.headlights(), (-1.0, false, true));

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            -1.0,
            0.0,
        );
        assert!(dynamics.headlights().0 > -1.0);
        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Ready,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            -1.0,
            0.0,
        );
        assert_eq!(dynamics.headlights(), (-1.0, false, false));

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            -1.0,
            0.0,
        );
        for _ in 0..31 {
            update!(
                dynamics,
                config,
                crate::FloatOutBoyRunState::Running,
                crate::FloatOutBoyMode::Normal,
                false,
                crate::FloatOutBoyFootpadState::None,
                -1.0,
                0.0,
            );
        }
        assert_eq!(dynamics.headlights(), (1.0, true, false));

        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            true,
            crate::FloatOutBoyFootpadState::None,
            -1.0,
            1.0,
        );
        assert_eq!(dynamics.direction(), (-1.0, false));

        let showing_config = super::FloatOutBoyLedsConfig::new(
            bar,
            bar,
            bar,
            bar,
            status.showing_sensors_while_running(),
            bar,
        )
        .enabled();
        let mut showing_dynamics = super::FloatOutBoyLedDynamics::new(0.0);
        update!(
            showing_dynamics,
            showing_config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::Both,
            0.0,
            0.0,
        );
        assert_eq!(
            showing_dynamics.sensor_fades(),
            (Ratio::from_ratio_const(0.0), Ratio::from_ratio_const(0.0))
        );
    }
}
