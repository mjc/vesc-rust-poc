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

/// Allocation-free pixels for one configured internal LED strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatOutBoyLedStripFrame {
    config: FloatOutBoyLedStripConfig,
    pixels: [FloatOutBoyLedPixel; MAX_LED_STRIP_PIXELS],
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
}
