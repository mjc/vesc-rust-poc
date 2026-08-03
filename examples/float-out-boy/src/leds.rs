//! Float Out Boy LED support types.
//!
//! These types model Float Out Boy's internal LED configuration surface. Raw config
//! field packing stays at package/config boundaries.

use vescpkg_rs::prelude::Ratio;

pub use vescpkg_rs::stm32::float_out_boy_ws2812::{
    Pin as FloatOutBoyLedPin, PinConfig as FloatOutBoyLedPinConfig,
};

wire_enum! {
/// Float Out Boy LED color channel order.
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
}

wire_enum! {
/// Float Out Boy named LED color.
///
/// C map: these IDs follow the `enumNames` order for LED color config fields at
/// `third_party/float-out-boy/src/conf/settings.xml:3456-3487`.
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
}

/// One renderer pixel in red, green, blue, white channel order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FloatOutBoyLedPixel {
    pub(crate) channels: [u8; 4],
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
        if blend.is_nan() {
            return Self::default();
        }
        first.scaled_and_blended(second, Ratio::from_ratio_const(1.0), Ratio::clamped(blend))
    }
}

fn refloat_led_gamma(channel: u8) -> u8 {
    let channel = u16::from(channel);
    u8::try_from(channel.saturating_mul(channel.saturating_add(1)) / 256).unwrap_or_default()
}

wire_enum! {
/// Float Out Boy LED animation mode.
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
}

wire_enum! {
/// Float Out Boy LED transition mode.
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
}

/// Float Out Boy LED animation speed scalar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FloatOutBoyLedAnimationSpeed(pub(crate) f32);

/// Float Out Boy LED bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedBarConfig {
    pub(crate) brightness: Ratio,
    pub(crate) primary_color: FloatOutBoyLedColor,
    pub(crate) secondary_color: FloatOutBoyLedColor,
    pub(crate) animation_mode: FloatOutBoyLedAnimationMode,
    pub(crate) animation_speed: FloatOutBoyLedAnimationSpeed,
}

/// Float Out Boy status-bar idle timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FloatOutBoyStatusBarIdleTimeout(pub(crate) u16);

/// Float Out Boy status-bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyStatusBarConfig {
    pub(crate) idle_timeout: FloatOutBoyStatusBarIdleTimeout,
    pub(crate) duty_threshold: Ratio,
    pub(crate) red_bar_percentage: Ratio,
    pub(crate) show_sensors_while_running: bool,
    pub(crate) brightness_headlights_on: Ratio,
    pub(crate) brightness_headlights_off: Ratio,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyLiftedLedsConfig {
    pub(crate) lights_off: bool,
    pub(crate) status_on_front: bool,
}

/// Float Out Boy LEDs configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedsConfig {
    pub(crate) on: bool,
    pub(crate) headlights_on: bool,
    pub(crate) headlights_transition: FloatOutBoyLedTransition,
    pub(crate) direction_transition: FloatOutBoyLedTransition,
    pub(crate) lifted: FloatOutBoyLiftedLedsConfig,
    pub(crate) headlights: FloatOutBoyLedBarConfig,
    pub(crate) taillights: FloatOutBoyLedBarConfig,
    pub(crate) front: FloatOutBoyLedBarConfig,
    pub(crate) rear: FloatOutBoyLedBarConfig,
    pub(crate) status: FloatOutBoyStatusBarConfig,
    pub(crate) status_idle: FloatOutBoyLedBarConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyLedRuntimeStatus {
    pub(crate) enabled: bool,
    pub(crate) headlights_enabled: bool,
}

wire_enum! {
/// Float Out Boy physical LED strip order.
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
}

/// Float Out Boy LED strip configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyLedStripConfig {
    pub(crate) order: FloatOutBoyLedStripOrder,
    pub(crate) count: u8,
    pub(crate) color_order: FloatOutBoyLedColorOrder,
    pub(crate) reverse: bool,
}

// Refloat stores each strip length in one byte.
// Refloat's internal LED setup accepts at most 30 front, rear, or status
// pixels; keeping only that physical bound avoids putting three impossible
// 255-pixel scratch frames on the firmware startup stack.
const MAX_LED_STRIP_PIXELS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyHeadlightsState {
    Off,
    TransitioningOn,
    On,
    TransitioningOff,
}

impl FloatOutBoyHeadlightsState {
    const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }

    const fn is_transitioning(self) -> bool {
        matches!(self, Self::TransitioningOn | Self::TransitioningOff)
    }
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
    headlights_time: f32,
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
            headlights_time: 0.0,
            direction_forward: true,
            direction_split: 1.0,
            split_distance: distance,
        }
    }

    /// Advance the pure renderer decisions by one source-rate 30 Hz tick.
    pub fn update(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        current_time: f32,
    ) -> bool {
        let FloatOutBoyLedUpdate {
            run_state,
            mode,
            darkride,
            footpad,
            pitch_degrees,
            distance,
        } = input;
        if matches!(run_state, crate::FloatOutBoyRunState::Startup) {
            self.run_state = run_state;
            return false;
        }

        if !config.on && self.on_off_fade == 0.0 {
            self.run_state = run_state;
            return false;
        }
        self.on_off_fade = rate_limit(self.on_off_fade, f32::from(u8::from(config.on)), 3.0 / 30.0);

        if !self.board_is_upright && pitch_degrees > 60.0 {
            self.board_is_upright = true;
        } else if self.board_is_upright && pitch_degrees < 50.0 {
            self.board_is_upright = false;
        }

        let running = matches!(run_state, crate::FloatOutBoyRunState::Running);
        if run_state != self.run_state {
            if matches!(self.run_state, crate::FloatOutBoyRunState::Disabled)
                || matches!(run_state, crate::FloatOutBoyRunState::Disabled)
            {
                self.on_off_fade = 0.0;
            }
            if running && !self.headlights_state.is_transitioning() {
                self.direction_forward = pitch_degrees >= 0.0;
                self.direction_split = if self.direction_forward { 1.0 } else { -1.0 };
            }
        }
        self.run_state = run_state;

        let show_sensors = config.status.show_sensors_while_running || !running;
        let both = !running && matches!(footpad, crate::FloatOutBoyFootpadState::Both);
        let left =
            show_sensors && (both || matches!(footpad, crate::FloatOutBoyFootpadState::Left));
        let right =
            show_sensors && (both || matches!(footpad, crate::FloatOutBoyFootpadState::Right));
        self.left_sensor = rate_limit(self.left_sensor, f32::from(u8::from(left)), 10.0 / 30.0);
        self.right_sensor = rate_limit(self.right_sensor, f32::from(u8::from(right)), 10.0 / 30.0);

        let headlights_should = matches!(run_state, crate::FloatOutBoyRunState::Running)
            && !matches!(mode, crate::FloatOutBoyMode::Flywheel)
            && config.headlights_on;
        let headlights_on = self.headlights_state.is_on();
        let transitioning = self.headlights_state.is_transitioning();
        let was_headlights_transitioning = transitioning;
        if headlights_should != headlights_on && !transitioning {
            self.headlights_split = -1.0;
            self.headlights_time = current_time;
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
            let elapsed = current_time - self.headlights_time;
            self.headlights_split =
                (self.headlights_split + direction * elapsed * 2.0).clamp(-1.0, 1.0);
            self.headlights_time = current_time;
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
            && !was_headlights_transitioning
            && !self.headlights_state.is_transitioning()
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
        true
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

    const_field_getters! {
        /// Return whether the lifted-board hysteresis is on its upright side.
        pub fn is_board_upright -> bool = board_is_upright;
    }

    /// Return the current headlight transition split and settled state.
    #[must_use]
    pub const fn headlights(self) -> (f32, bool, bool) {
        (
            self.headlights_split,
            self.headlights_state.is_on(),
            self.headlights_state.is_transitioning(),
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

/// Values used only by Refloat's status-strip composition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedStatusUpdate {
    /// Remaining battery fraction.
    pub battery_level: f32,
    /// Raw VESC duty-cycle fraction.
    pub duty_cycle: f32,
    /// Whether motor distance changed during this update.
    pub moving: bool,
}

/// One coherent 30 Hz renderer snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedFrameUpdate {
    /// Ride and transition inputs.
    pub ride: FloatOutBoyLedUpdate,
    /// Status-strip inputs sampled for the same frame.
    pub status: FloatOutBoyLedStatusUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatOutBoyStatusDynamics {
    brightness: f32,
    duty_blend: f32,
    idle_blend: f32,
    idle_time: f32,
    animation_start: f32,
}

#[derive(Debug, Clone, Copy)]
struct FloatOutBoyStatusRenderState {
    brightness: Ratio,
    duty: f32,
    duty_blend: Ratio,
    idle_blend: Ratio,
    idle_animation_time: f32,
}

#[derive(Debug, Clone, Copy)]
enum FloatOutBoyStatusIdleLayer {
    Animate(FloatOutBoyLedBarConfig, f32),
    Black,
}

#[derive(Debug, Clone, Copy)]
struct FloatOutBoyStatusLayers {
    brightness: Ratio,
    fade: Ratio,
    blend: Ratio,
    idle_blend: Ratio,
    idle: FloatOutBoyStatusIdleLayer,
    reverse: bool,
    red_percentage: Ratio,
    battery: f32,
    duty: f32,
    duty_blend: Ratio,
    sensors: (Ratio, Ratio),
    confirmation_progress: f32,
}

impl FloatOutBoyStatusDynamics {
    const fn new() -> Self {
        Self {
            brightness: 0.0,
            duty_blend: 0.0,
            idle_blend: 0.0,
            idle_time: 0.0,
            animation_start: 0.0,
        }
    }

    fn update_brightness(&mut self, config: FloatOutBoyLedsConfig) -> Ratio {
        let status_config = config.status;
        let mut target_brightness = if config.headlights_on {
            status_config.brightness_headlights_on
        } else {
            status_config.brightness_headlights_off
        };
        if self.idle_blend > 0.0 {
            target_brightness = Ratio::clamped(
                target_brightness
                    .as_ratio()
                    .min(config.status_idle.brightness.as_ratio()),
            );
        }
        self.brightness = rate_limit(self.brightness, target_brightness.as_ratio(), 3.0 / 30.0);
        Ratio::clamped(self.brightness)
    }

    fn update(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        status: FloatOutBoyLedStatusUpdate,
        sensors: (Ratio, Ratio),
        reset_idle: bool,
        current_time: f32,
    ) -> FloatOutBoyStatusRenderState {
        let status_config = config.status;
        if reset_idle || input.footpad.is_pressed() {
            self.idle_time = current_time;
        }

        let duty = if matches!(input.run_state, crate::FloatOutBoyRunState::Running)
            && !matches!(input.mode, crate::FloatOutBoyMode::Flywheel)
        {
            (status.duty_cycle.abs() * 10.0 / 9.0).min(1.0)
        } else {
            0.0
        };
        let duty_threshold = status_config.duty_threshold.as_ratio().max(0.15);
        let duty_target = if duty > duty_threshold {
            1.0
        } else if duty < duty_threshold - 0.1 {
            0.0
        } else {
            self.duty_blend
        };
        self.duty_blend = rate_limit(self.duty_blend, duty_target, 5.0 / 30.0).clamp(0.0, 1.0);

        if sensors.0.as_ratio() >= 1.0 || sensors.1.as_ratio() >= 1.0 {
            self.idle_blend = 0.0;
        }
        let idle_timeout = f32::from(status_config.idle_timeout.0);
        let idle_target = if idle_timeout > 0.0 && current_time - self.idle_time > idle_timeout {
            if self.idle_blend == 0.0 {
                self.animation_start = current_time;
            }
            1.0
        } else {
            0.0
        };
        self.idle_blend = rate_limit(self.idle_blend, idle_target, 3.0 / 30.0).clamp(0.0, 1.0);
        if status.moving {
            self.idle_time = current_time;
        }

        FloatOutBoyStatusRenderState {
            brightness: Ratio::clamped(self.brightness),
            duty,
            duty_blend: Ratio::clamped(self.duty_blend),
            idle_blend: Ratio::clamped(self.idle_blend),
            idle_animation_time: current_time - self.animation_start,
        }
    }
}

/// Pure composed status/front/rear frames for one internal LED configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedRenderer {
    dynamics: FloatOutBoyLedDynamics,
    status_dynamics: FloatOutBoyStatusDynamics,
    pub(crate) status: FloatOutBoyLedStripFrame,
    pub(crate) front: FloatOutBoyLedStripFrame,
    pub(crate) rear: FloatOutBoyLedStripFrame,
    front_bar: FloatOutBoyLedBarConfig,
    rear_bar: FloatOutBoyLedBarConfig,
    animation_start: f32,
    confirmation_start: f32,
    front_brightness: f32,
    rear_brightness: f32,
    status_on_front_blend: f32,
    status_on_front_idle_blend: f32,
    status_on_front_idle_time: f32,
}

#[derive(Debug, Clone, Copy)]
struct FloatOutBoyFrameTransition {
    config: FloatOutBoyLedsConfig,
    input: FloatOutBoyLedUpdate,
    current_time: f32,
    fade: Ratio,
    seed: u32,
    old_headlights: (f32, bool, bool),
    old_direction: (f32, bool),
    headlights: (f32, bool, bool),
    direction: (f32, bool),
}

impl FloatOutBoyLedRenderer {
    /// Build cleared frames and source-default front/rear bar roles.
    #[must_use]
    pub fn new(
        hardware: crate::lcm::FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
        distance: f32,
    ) -> Self {
        Self {
            dynamics: FloatOutBoyLedDynamics::new(distance),
            status_dynamics: FloatOutBoyStatusDynamics::new(),
            status: FloatOutBoyLedStripFrame::new(hardware.status),
            front: FloatOutBoyLedStripFrame::new(hardware.front),
            rear: FloatOutBoyLedStripFrame::new(hardware.rear),
            front_bar: config.front,
            rear_bar: config.rear,
            animation_start: 0.0,
            confirmation_start: -1.0,
            front_brightness: config.front.brightness.as_ratio(),
            rear_brightness: config.rear.brightness.as_ratio(),
            status_on_front_blend: 0.0,
            status_on_front_idle_blend: 0.0,
            status_on_front_idle_time: 0.0,
        }
    }

    /// Advance and compose the front/rear frame in Refloat's transition order.
    pub fn update(
        &mut self,
        config: FloatOutBoyLedsConfig,
        frame: FloatOutBoyLedFrameUpdate,
        current_time: f32,
    ) -> bool {
        let FloatOutBoyLedFrameUpdate {
            ride: input,
            status,
        } = frame;
        let was_startup = matches!(self.dynamics.run_state, crate::FloatOutBoyRunState::Startup);
        let old_headlights = self.dynamics.headlights();
        let old_direction = self.dynamics.direction();
        let was_upright = self.dynamics.is_board_upright();
        if !self.dynamics.update(config, input, current_time) {
            return false;
        }
        let upright = self.dynamics.is_board_upright();
        let upright_changed = upright != was_upright;

        let fade = self.dynamics.on_off_fade();
        if was_startup {
            self.animation_start = current_time;
            self.status_dynamics.idle_time = current_time;
            self.status_on_front_idle_time = current_time;
            if upright {
                self.status_on_front_blend = 1.0;
            }
        }
        if input.footpad.is_pressed() || (!was_upright && upright) {
            self.status_on_front_idle_time = current_time;
        }
        let status_on_front = config.lifted.status_on_front
            && matches!(input.run_state, crate::FloatOutBoyRunState::Ready)
            && upright;
        let status_brightness = self.status_dynamics.update_brightness(config);
        let front_target = if status_on_front {
            status_brightness.as_ratio()
        } else if upright && config.lifted.lights_off {
            0.0
        } else {
            self.front_bar.brightness.as_ratio()
        };
        let rear_target = if upright && config.lifted.lights_off {
            0.0
        } else {
            self.rear_bar.brightness.as_ratio()
        };
        self.front_brightness = rate_limit(self.front_brightness, front_target, 3.0 / 30.0);
        self.rear_brightness = rate_limit(self.rear_brightness, rear_target, 3.0 / 30.0);
        self.status_on_front_blend = rate_limit(
            self.status_on_front_blend,
            f32::from(u8::from(status_on_front)),
            3.0 / 30.0,
        );
        if matches!(input.run_state, crate::FloatOutBoyRunState::Disabled) {
            self.status
                .render_disabled(status_brightness, fade, current_time);
            self.front
                .render_disabled(Ratio::clamped(self.front_brightness), fade, current_time);
            self.rear
                .render_disabled(Ratio::clamped(self.rear_brightness), fade, current_time);
            return true;
        }

        let status_state =
            self.compose_status(config, input, current_time, status, fade, upright_changed);
        if config.lifted.status_on_front && self.status_on_front_blend > 0.0 {
            let front_idle =
                config.lifted.lights_off && current_time - self.status_on_front_idle_time > 3.0;
            self.status_on_front_idle_blend = rate_limit(
                self.status_on_front_idle_blend,
                f32::from(u8::from(front_idle)),
                3.0 / 30.0,
            );
        }

        let animation_time = current_time - self.animation_start;
        let mut front_bar = self.front_bar;
        front_bar.brightness = Ratio::clamped(self.front_brightness);
        let mut rear_bar = self.rear_bar;
        rear_bar.brightness = Ratio::clamped(self.rear_brightness);
        self.front.render_bar(front_bar, fade, animation_time);
        self.rear.render_bar(rear_bar, fade, animation_time);

        let transition = FloatOutBoyFrameTransition {
            config,
            input,
            current_time,
            fade,
            seed: crate::wire::saturating_trunc_f32_to_u32(self.animation_start),
            old_headlights,
            old_direction,
            headlights: self.dynamics.headlights(),
            direction: self.dynamics.direction(),
        };
        self.compose_headlights(transition);
        self.compose_direction(transition);
        self.compose_front_status(config, status, status_state, fade, current_time);
        true
    }

    /// Begin Refloat's 0.8-second status-confirm animation.
    pub fn start_confirmation(&mut self, current_time: f32) {
        if current_time - self.confirmation_start > 0.8 {
            self.confirmation_start = current_time;
        }
    }

    fn compose_status(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        current_time: f32,
        status: FloatOutBoyLedStatusUpdate,
        fade: Ratio,
        reset_idle: bool,
    ) -> FloatOutBoyStatusRenderState {
        let status_config = config.status;
        let sensors = self.dynamics.sensor_fades();
        let state =
            self.status_dynamics
                .update(config, input, status, sensors, reset_idle, current_time);

        self.status.render_status_layers(FloatOutBoyStatusLayers {
            brightness: state.brightness,
            fade,
            blend: Ratio::from_ratio_const(1.0),
            idle_blend: state.idle_blend,
            idle: FloatOutBoyStatusIdleLayer::Animate(
                config.status_idle,
                state.idle_animation_time,
            ),
            reverse: false,
            red_percentage: status_config.red_bar_percentage,
            battery: status.battery_level.clamp(0.0, 1.0),
            duty: state.duty,
            duty_blend: state.duty_blend,
            sensors,
            confirmation_progress: (current_time - self.confirmation_start) / 0.8,
        });
        state
    }

    fn compose_front_status(
        &mut self,
        config: FloatOutBoyLedsConfig,
        status: FloatOutBoyLedStatusUpdate,
        state: FloatOutBoyStatusRenderState,
        fade: Ratio,
        current_time: f32,
    ) {
        let blend = Ratio::clamped(self.status_on_front_blend);
        if !config.lifted.status_on_front || blend.as_ratio() <= 0.0 {
            return;
        }
        self.front.render_status_layers(FloatOutBoyStatusLayers {
            brightness: Ratio::clamped(self.front_brightness),
            fade,
            blend,
            idle_blend: Ratio::clamped(self.status_on_front_idle_blend),
            idle: FloatOutBoyStatusIdleLayer::Black,
            reverse: true,
            red_percentage: config.status.red_bar_percentage,
            battery: status.battery_level.clamp(0.0, 1.0),
            duty: state.duty,
            duty_blend: state.duty_blend,
            sensors: self.dynamics.sensor_fades(),
            confirmation_progress: (current_time - self.confirmation_start) / 0.8,
        });
    }

    fn compose_headlights(&mut self, transition: FloatOutBoyFrameTransition) {
        let FloatOutBoyFrameTransition {
            config,
            input,
            current_time,
            old_headlights,
            old_direction,
            headlights,
            direction,
            ..
        } = transition;
        let targets = if headlights.2 {
            let headlights_should = matches!(input.run_state, crate::FloatOutBoyRunState::Running)
                && !matches!(input.mode, crate::FloatOutBoyMode::Flywheel)
                && config.headlights_on;
            select_front_rear_bars(config, headlights_should, old_direction.1)
        } else if old_headlights.2 {
            select_front_rear_bars(config, headlights.1, direction.1)
        } else {
            return;
        };
        self.render_pair_transition(
            transition,
            config.headlights_transition,
            headlights.0,
            targets,
        );
        if !headlights.2 {
            (self.front_bar, self.rear_bar) = targets;
            self.animation_start = current_time;
        }
    }

    fn compose_direction(&mut self, transition: FloatOutBoyFrameTransition) {
        let FloatOutBoyFrameTransition {
            config,
            current_time,
            old_direction,
            headlights,
            direction,
            ..
        } = transition;
        if !headlights.1 || headlights.2 || direction.0.to_bits() == old_direction.0.to_bits() {
            return;
        }
        let targets = select_front_rear_bars(config, true, !old_direction.1);
        let progress = if old_direction.1 {
            -direction.0
        } else {
            direction.0
        };
        self.render_pair_transition(transition, config.direction_transition, progress, targets);
        if direction.1 != old_direction.1 {
            (self.front_bar, self.rear_bar) = targets;
            self.animation_start = current_time;
        }
    }

    fn render_pair_transition(
        &mut self,
        tick: FloatOutBoyFrameTransition,
        mode: FloatOutBoyLedTransition,
        progress: f32,
        targets: (FloatOutBoyLedBarConfig, FloatOutBoyLedBarConfig),
    ) {
        self.front.render_transition(
            mode,
            progress,
            tick.seed,
            self.front_bar,
            targets.0,
            tick.fade,
        );
        self.rear.render_transition(
            mode,
            progress,
            tick.seed,
            self.rear_bar,
            targets.1,
            tick.fade,
        );
    }
}

/// Allocation-free pixels for one configured internal LED strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
struct FloatOutBoyLedOverlay {
    strip_brightness: Ratio,
    on_off_fade: Ratio,
    blend: Ratio,
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

    /// Return one pixel in physical strip order.
    #[must_use]
    pub fn physical_pixel(&self, index: usize) -> Option<FloatOutBoyLedPixel> {
        let len = usize::from(self.config.count);
        if index >= len {
            return None;
        }
        let logical_index = if self.config.reverse {
            len.checked_sub(index)?.checked_sub(1)?
        } else {
            index
        };
        self.pixels.get(logical_index).copied()
    }

    /// Paint Refloat's left/right footpad indicator over this strip.
    fn render_footpads(
        &mut self,
        left: Ratio,
        right: Ratio,
        reverse_roles: bool,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let len = usize::from(self.config.count);
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

    fn render_status_layers(&mut self, layers: FloatOutBoyStatusLayers) {
        let FloatOutBoyStatusLayers {
            brightness,
            fade,
            blend,
            idle_blend,
            idle,
            reverse,
            red_percentage,
            battery,
            duty,
            duty_blend,
            sensors: (left, right),
            confirmation_progress,
        } = layers;

        if idle_blend.as_ratio() > 0.0 {
            match idle {
                FloatOutBoyStatusIdleLayer::Animate(mut bar, time) => {
                    bar.brightness = brightness;
                    self.render_bar(bar, fade, time);
                }
                FloatOutBoyStatusIdleLayer::Black => self.render_target(
                    FloatOutBoyLedPixel::default(),
                    Ratio::clamped(brightness.as_ratio() * fade.as_ratio()),
                    blend,
                ),
            }
        }

        let overlay = |amount| FloatOutBoyLedOverlay {
            strip_brightness: brightness,
            on_off_fade: fade,
            blend: amount,
        };
        if idle_blend.as_ratio() < 1.0 && duty_blend.as_ratio() < 1.0 {
            self.render_status_progress(
                battery,
                FloatOutBoyStatusProgress::Battery,
                red_percentage,
                reverse,
                overlay(Ratio::clamped(
                    blend.as_ratio().min(1.0 - idle_blend.as_ratio()),
                )),
            );
        }
        if idle_blend.as_ratio() < 1.0 && duty_blend.as_ratio() > 0.0 {
            self.render_status_progress(
                duty,
                FloatOutBoyStatusProgress::Duty,
                red_percentage,
                reverse,
                overlay(duty_blend),
            );
        }
        if idle_blend.as_ratio() < 1.0 && (left.as_ratio() > 0.0 || right.as_ratio() > 0.0) {
            self.render_footpads(left, right, reverse, overlay(blend));
        }
        if (0.0..=1.0).contains(&confirmation_progress) {
            self.render_confirmation(brightness, fade, confirmation_progress);
        }
    }

    /// Paint Refloat's battery or duty progress display over this strip.
    fn render_status_progress(
        &mut self,
        value: f32,
        kind: FloatOutBoyStatusProgress,
        red_percentage: Ratio,
        reverse: bool,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let len = usize::from(self.config.count);
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
        let red = FloatOutBoyLedPixel {
            channels: [0xff, 0, 0, 0],
        };
        let brightness = Ratio::clamped(strip_brightness.as_ratio() * on_off_fade.as_ratio());
        self.render_pulse_shape(
            red,
            FloatOutBoyLedPixel::default(),
            brightness,
            time / 2.0,
            3.0,
        );
    }

    /// Paint Refloat's status-confirm pulse over this strip.
    pub fn render_confirmation(
        &mut self,
        strip_brightness: Ratio,
        on_off_fade: Ratio,
        progress: f32,
    ) {
        let len = usize::from(self.config.count);
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
        let time = time * bar.animation_speed.0;
        let target = match bar.animation_mode {
            FloatOutBoyLedAnimationMode::Felony => {
                self.render_felony(bar, on_off_fade, time);
                return;
            }
            FloatOutBoyLedAnimationMode::RainbowCycle
            | FloatOutBoyLedAnimationMode::RainbowFade
            | FloatOutBoyLedAnimationMode::RainbowRoll => {
                self.render_rainbow(bar, on_off_fade, time);
                return;
            }
            FloatOutBoyLedAnimationMode::Pulse => {
                self.render_pulse(bar, on_off_fade, time);
                return;
            }
            FloatOutBoyLedAnimationMode::KnightRider => {
                self.render_knight_rider(bar, on_off_fade, time);
                return;
            }
            FloatOutBoyLedAnimationMode::Solid => {
                FloatOutBoyLedPixel::from_named(bar.primary_color)
            }
            FloatOutBoyLedAnimationMode::Fade => FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel::from_named(bar.secondary_color),
                FloatOutBoyLedPixel::from_named(bar.primary_color),
                refloat_cosine_progress(time),
            ),
            FloatOutBoyLedAnimationMode::Strobe => {
                let color = if vescpkg_rs::remainder(time, 2.0) >= 1.0 {
                    bar.secondary_color
                } else {
                    bar.primary_color
                };
                FloatOutBoyLedPixel::from_named(color)
            }
        };
        let brightness = Ratio::clamped(bar.brightness.as_ratio() * on_off_fade.as_ratio());
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
                    (from_bar.brightness.as_ratio()
                        + (to_bar.brightness.as_ratio() - from_bar.brightness.as_ratio())
                            * blend.as_ratio())
                        * on_off_fade.as_ratio(),
                );
                self.render_target(transition_target(to_bar), brightness, blend);
            }
            FloatOutBoyLedTransition::FadeOutIn => {
                self.render_fade_out_in(progress, to_bar, from_bar.brightness, on_off_fade);
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
                    + (to_bar.brightness.as_ratio() - from_brightness.as_ratio())
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
                + (to_bar.brightness.as_ratio() - from_brightness.as_ratio()) * progress)
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
        let len = usize::from(self.config.count);
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
                .brightness
                .as_ratio()
                .midpoint(to_bar.brightness.as_ratio())
                * on_off_fade.as_ratio(),
        );
        let target_brightness =
            Ratio::clamped(to_bar.brightness.as_ratio() * on_off_fade.as_ratio());

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
                        FloatOutBoyLedPixel::from_named(from_bar.primary_color),
                        target,
                        f32::from(random) / 256.0,
                    )
                } else {
                    let white = u8::try_from(
                        (refloat_random(random_seed.wrapping_add(23)) % 128).wrapping_add(80),
                    )
                    .unwrap_or_default();
                    let mut channels = refloat_hue_to_pixel(random).channels;
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
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        let brightness = Ratio::clamped(bar.brightness.as_ratio() * on_off_fade.as_ratio());
        self.render_pulse_shape(primary, secondary, brightness, time, 5.0);
    }

    fn render_pulse_shape(
        &mut self,
        primary: FloatOutBoyLedPixel,
        secondary: FloatOutBoyLedPixel,
        brightness: Ratio,
        time: f32,
        center_divisor: f32,
    ) {
        let len = usize::from(self.config.count);
        let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
            return;
        };
        let len_float = f32::from(len_u16);
        let progress = refloat_cosine_progress(time);
        let center = len_float / center_divisor;
        let length = len_float / 2.0 - center;
        let offset = length * (1.0 - progress);
        let feather = len_float / 4.0;
        let ratio = center / length;
        let fade = if time < ratio { time / ratio } else { 1.0 };
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
        let len = usize::from(self.config.count);
        let Some(len_u16) = u16::try_from(len).ok().filter(|len| *len > 0) else {
            return;
        };
        let len_float = f32::from(len_u16);
        let tail = f32::from((len_u16 / 3).saturating_add(1));
        let time = time * 0.7;
        let backlight = if time > 0.3 { 0.08 } else { 0.0 };
        let first = len_float * vescpkg_rs::remainder(time, 2.0) - 0.5 * len_float - 1.0;
        let second = 1.5 * len_float - len_float * vescpkg_rs::remainder(time - 1.0, 2.0);
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        let brightness = Ratio::clamped(bar.brightness.as_ratio() * on_off_fade.as_ratio());

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
        let len = usize::from(self.config.count);
        let stop = len / 2;
        let start = stop.saturating_add(len.checked_rem(2).unwrap_or_default());
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        let black = FloatOutBoyLedPixel::default();
        let brightness = Ratio::clamped(bar.brightness.as_ratio() * on_off_fade.as_ratio());

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
        let brightness = Ratio::clamped(bar.brightness.as_ratio() * on_off_fade.as_ratio());
        match bar.animation_mode {
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
                let len = usize::from(self.config.count);
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
        let len = usize::from(self.config.count);
        for pixel in self.pixels.get_mut(..len).unwrap_or_default() {
            *pixel = pixel.scaled_and_blended(target, brightness, blend);
        }
    }
}

fn transition_target(bar: FloatOutBoyLedBarConfig) -> FloatOutBoyLedPixel {
    let color = if matches!(bar.animation_mode, FloatOutBoyLedAnimationMode::Solid) {
        bar.primary_color
    } else {
        bar.secondary_color
    };
    FloatOutBoyLedPixel::from_named(color)
}

/// Select physical front/rear bars from settled headlight and travel direction state.
#[must_use]
pub const fn select_front_rear_bars(
    config: FloatOutBoyLedsConfig,
    headlights_on: bool,
    direction_forward: bool,
) -> (FloatOutBoyLedBarConfig, FloatOutBoyLedBarConfig) {
    if !headlights_on {
        return (config.front, config.rear);
    }
    if direction_forward {
        (config.headlights, config.taillights)
    } else {
        (config.taillights, config.headlights)
    }
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
#[path = "leds/tests/api.rs"]
mod test_api;

#[cfg(test)]
mod renderer_tests;
