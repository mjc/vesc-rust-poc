//! Float Out Boy LED support types.
//!
//! This module deliberately owns FOB-specific LED configuration and rendering. It is
//! not a generic VESC or `vescpkg-rs` LED API. Raw config field packing stays at
//! package/config boundaries.

#[cfg(test)]
macro_rules! assert_f32_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual: f32 = $actual;
        let expected: f32 = $expected;
        let tolerance = f32::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 4.0;
        let exactly_equal = !actual.is_nan() && actual.to_bits() == expected.to_bits();
        assert!(
            exactly_equal
                || (actual.is_finite()
                    && expected.is_finite()
                    && (actual - expected).abs() <= tolerance),
            "expected {expected:?}, got {actual:?} (tolerance {tolerance:?})"
        );
    }};
}

mod hardware;
mod mode;
#[cfg(any(test, feature = "test-support"))]
mod test_support;

pub use self::hardware::FloatOutBoyHardwareLedsConfig;
pub use self::mode::FloatOutBoyLedMode;
pub use crate::protocol::{FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyRunState};

/// Compatibility namespace for FOB hardware LED configuration.
pub mod lcm {
    pub use super::{FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode};
}

use vescpkg_rs::prelude::Ratio;

const LED_FADE_STEP: Ratio = Ratio::from_ratio_const(3.0 / 30.0);
const SENSOR_FADE_STEP: Ratio = Ratio::from_ratio_const(10.0 / 30.0);
const UTILIZATION_FADE_STEP: Ratio = Ratio::from_ratio_const(5.0 / 30.0);

vescpkg_rs::wire_enum! {
/// Refloat-compatible internal LED output pin.
pub enum FloatOutBoyLedPin {
    /// STM32 B6.
    B6 = 0,
    /// STM32 B7.
    B7 = 1,
    /// STM32 C9.
    C9 = 2,
}
}

vescpkg_rs::wire_enum! {
/// Refloat-compatible internal LED output-drive configuration.
pub enum FloatOutBoyLedPinConfig {
    /// Open drain for an external 5 V pull-up.
    PullupTo5v = 0,
    /// Push-pull alternate-function output.
    NoPullup = 1,
}
}

impl From<FloatOutBoyLedPin> for vescpkg_rs::stm32::ws2812::OutputPin {
    fn from(pin: FloatOutBoyLedPin) -> Self {
        match pin {
            FloatOutBoyLedPin::B6 => Self::B6,
            FloatOutBoyLedPin::B7 => Self::B7,
            FloatOutBoyLedPin::C9 => Self::C9,
        }
    }
}

impl From<FloatOutBoyLedPinConfig> for vescpkg_rs::stm32::ws2812::OutputDrive {
    fn from(config: FloatOutBoyLedPinConfig) -> Self {
        match config {
            FloatOutBoyLedPinConfig::PullupTo5v => Self::OpenDrain,
            FloatOutBoyLedPinConfig::NoPullup => Self::PushPull,
        }
    }
}

vescpkg_rs::wire_enum! {
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

macro_rules! named_led_colors {
    ($($name:ident = $id:literal => $channels:expr),+ $(,)?) => {
        vescpkg_rs::wire_enum! {
        /// Float Out Boy named LED color.
        ///
        /// C map: these IDs follow the `enumNames` order for LED color config fields at
        /// `third_party/float-out-boy/src/conf/settings.xml:3456-3487`.
        pub enum FloatOutBoyLedColor {
            $(#[doc = concat!(stringify!($name), " LED color.")]
            $name = $id,)+
        }
        }

        impl FloatOutBoyLedColor {
            const fn channels(self) -> [u8; 4] {
                match self {
                    $(Self::$name => $channels,)+
                }
            }
        }
    };
}

named_led_colors! {
    Black = 0 => [0x00, 0x00, 0x00, 0x00],
    WhiteFull = 1 => [0xff, 0xff, 0xff, 0xff],
    WhiteRgb = 2 => [0xff, 0xff, 0xff, 0x00],
    WhiteSingle = 3 => [0x00, 0x00, 0x00, 0xff],
    Red = 4 => [0xff, 0x00, 0x00, 0x00],
    Ferrari = 5 => [0xff, 0x38, 0x00, 0x00],
    Flame = 6 => [0xff, 0x50, 0x00, 0x00],
    Coral = 7 => [0xff, 0x60, 0x40, 0x00],
    Sunset = 8 => [0xff, 0x78, 0x30, 0x00],
    Sunrise = 9 => [0xff, 0x90, 0x40, 0x00],
    Gold = 10 => [0xff, 0x80, 0x20, 0x00],
    Orange = 11 => [0xff, 0x78, 0x00, 0x00],
    Yellow = 12 => [0xff, 0xa0, 0x00, 0x00],
    Banana = 13 => [0xff, 0xb0, 0x40, 0x00],
    Lime = 14 => [0xff, 0xff, 0x00, 0x00],
    Acid = 15 => [0xa0, 0xff, 0x00, 0x00],
    Sage = 16 => [0xa0, 0xff, 0x50, 0x00],
    Green = 17 => [0x00, 0xff, 0x00, 0x00],
    Mint = 18 => [0x00, 0xff, 0x50, 0x00],
    Tiffany = 19 => [0x00, 0xff, 0xc0, 0x00],
    Cyan = 20 => [0x00, 0xff, 0xff, 0x00],
    Steel = 21 => [0x90, 0xc0, 0xff, 0x00],
    Sky = 22 => [0x70, 0xd0, 0xff, 0x00],
    Azure = 23 => [0x00, 0xa0, 0xff, 0x00],
    Sapphire = 24 => [0x00, 0x70, 0xff, 0x00],
    Blue = 25 => [0x00, 0x00, 0xff, 0x00],
    Violet = 26 => [0x80, 0x00, 0xff, 0x00],
    Amethyst = 27 => [0xa0, 0x60, 0xff, 0x00],
    Magenta = 28 => [0xff, 0x00, 0xff, 0x00],
    Pink = 29 => [0xff, 0x00, 0xc0, 0x00],
    Fuchsia = 30 => [0xff, 0x00, 0x70, 0x00],
    Lavender = 31 => [0xff, 0x70, 0xa0, 0x00],
}

/// One renderer pixel in red, green, blue, white channel order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FloatOutBoyLedPixel {
    pub(crate) channels: [u8; 4],
}

impl FloatOutBoyLedPixel {
    const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            channels: [red, green, blue, 0],
        }
    }

    /// Return Refloat 1.2.1's exact channel values for a named color.
    #[must_use]
    pub fn from_named(color: FloatOutBoyLedColor) -> Self {
        Self {
            channels: color.channels(),
        }
    }

    /// Gamma-correct and reorder this pixel for one physical strip.
    pub fn physical_channels(self, order: FloatOutBoyLedColorOrder) -> ([u8; 4], usize) {
        let [red, green, blue, white] = self.channels.map(refloat_led_gamma);
        let (bytes, len) = match order {
            FloatOutBoyLedColorOrder::Grb => ([green, red, blue, 0], 3),
            FloatOutBoyLedColorOrder::Grbw => ([green, red, blue, white], 4),
            FloatOutBoyLedColorOrder::Rgb => ([red, green, blue, 0], 3),
            FloatOutBoyLedColorOrder::Wrgb => ([white, red, green, blue], 4),
        };
        (bytes, len)
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
        first.scaled_and_blended(second, Ratio::FULL, Ratio::clamped(blend))
    }
}

fn refloat_led_gamma(channel: u8) -> u8 {
    let channel = u16::from(channel);
    u8::try_from(channel.saturating_mul(channel.saturating_add(1)) / 256).unwrap_or_default()
}

vescpkg_rs::wire_enum! {
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

vescpkg_rs::wire_enum! {
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

/// Float Out Boy LED bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedBarConfig {
    /// Bar brightness ratio.
    pub brightness: Ratio,
    /// Primary named color.
    pub primary_color: FloatOutBoyLedColor,
    /// Secondary named color.
    pub secondary_color: FloatOutBoyLedColor,
    /// Animation mode.
    pub animation_mode: FloatOutBoyLedAnimationMode,
    /// Animation speed multiplier.
    pub animation_speed: f32,
}

/// Float Out Boy status-bar configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyStatusBarConfig {
    /// Idle timeout in source configuration units.
    pub idle_timeout: u16,
    /// Duty threshold for red progress.
    pub duty_threshold: Ratio,
    /// Red portion of the status bar.
    pub red_bar_percentage: Ratio,
    /// Whether running sensor state appears on the status bar.
    pub show_sensors_while_running: bool,
    /// Status brightness with headlights on.
    pub brightness_headlights_on: Ratio,
    /// Status brightness with headlights off.
    pub brightness_headlights_off: Ratio,
}

/// Lifted-board LED behavior.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyLiftedLedsConfig {
    /// Turn lights off while lifted.
    pub lights_off: bool,
    /// Render status on the front strip while lifted.
    pub status_on_front: bool,
}

/// Float Out Boy LEDs configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedsConfig {
    /// Master LED enable.
    pub on: bool,
    /// Headlight enable.
    pub headlights_on: bool,
    /// Headlight transition mode.
    pub headlights_transition: FloatOutBoyLedTransition,
    /// Direction-change transition mode.
    pub direction_transition: FloatOutBoyLedTransition,
    /// Lifted-board behavior.
    pub lifted: FloatOutBoyLiftedLedsConfig,
    /// Active headlight bar.
    pub headlights: FloatOutBoyLedBarConfig,
    /// Active taillight bar.
    pub taillights: FloatOutBoyLedBarConfig,
    /// Front direction bar.
    pub front: FloatOutBoyLedBarConfig,
    /// Rear direction bar.
    pub rear: FloatOutBoyLedBarConfig,
    /// Active status bar.
    pub status: FloatOutBoyStatusBarConfig,
    /// Idle status bar.
    pub status_idle: FloatOutBoyLedBarConfig,
}

/// Runtime LED enable overrides owned by package state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyLedRuntimeStatus {
    /// Whether LED rendering is enabled.
    pub enabled: bool,
    /// Whether headlights are enabled.
    pub headlights_enabled: bool,
}

vescpkg_rs::wire_enum! {
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
    /// Physical strip order.
    pub order: FloatOutBoyLedStripOrder,
    /// Pixel count.
    pub count: u8,
    /// Physical channel order.
    pub color_order: FloatOutBoyLedColorOrder,
    /// Whether logical pixels are reversed.
    pub reverse: bool,
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
    /// Remaining battery fraction.
    pub battery_level: f32,
    /// Raw VESC duty-cycle fraction.
    pub duty_cycle: f32,
    /// Motor-current saturation fraction.
    pub motor_current_saturation: f32,
    /// Battery-current saturation fraction.
    pub battery_current_saturation: f32,
    /// Whether motor distance changed during this update.
    pub moving: bool,
}

/// Pure 30 Hz state for Refloat's lifted-board, footpad, and direction decisions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedDynamics {
    run_state: crate::FloatOutBoyRunState,
    board_is_upright: bool,
    on_off_fade: Ratio,
    left_sensor: Ratio,
    right_sensor: Ratio,
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
            on_off_fade: Ratio::ZERO,
            left_sensor: Ratio::ZERO,
            right_sensor: Ratio::ZERO,
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
            ..
        } = input;
        if matches!(run_state, crate::FloatOutBoyRunState::Startup) {
            self.run_state = run_state;
            return false;
        }

        if !config.on && self.on_off_fade.is_zero() {
            self.run_state = run_state;
            return false;
        }
        self.on_off_fade = self.on_off_fade.slew_toward(config.on, LED_FADE_STEP);

        if !self.board_is_upright && pitch_degrees > 60.0 {
            self.board_is_upright = true;
        } else if self.board_is_upright && pitch_degrees < 50.0 {
            self.board_is_upright = false;
        }

        let running = run_state == crate::FloatOutBoyRunState::Running;
        if run_state != self.run_state {
            if matches!(self.run_state, crate::FloatOutBoyRunState::Disabled)
                || matches!(run_state, crate::FloatOutBoyRunState::Disabled)
            {
                self.on_off_fade = Ratio::ZERO;
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
        self.left_sensor = self.left_sensor.slew_toward(left, SENSOR_FADE_STEP);
        self.right_sensor = self.right_sensor.slew_toward(right, SENSOR_FADE_STEP);

        let headlights_should = run_state == crate::FloatOutBoyRunState::Running
            && mode != crate::FloatOutBoyMode::Flywheel
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

        if run_state == crate::FloatOutBoyRunState::Running
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
        self.on_off_fade
    }

    /// Return the left/right footpad indicator fades.
    #[must_use]
    pub fn sensor_fades(self) -> (Ratio, Ratio) {
        (self.left_sensor, self.right_sensor)
    }

    vescpkg_rs::const_field_getters! {
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

#[cfg(any(test, feature = "test-support"))]
/// Test-only split status input retained for compact renderer fixtures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedStatusUpdate {
    /// Remaining battery fraction.
    pub battery_level: f32,
    /// Raw VESC duty-cycle fraction.
    pub duty_cycle: f32,
    /// Motor-current saturation fraction.
    pub motor_current_saturation: f32,
    /// Battery-current saturation fraction.
    pub battery_current_saturation: f32,
    /// Whether motor distance changed during this update.
    pub moving: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatOutBoyStatusDynamics {
    brightness: Ratio,
    utilization_blend: Ratio,
    idle_blend: Ratio,
    idle_time: f32,
    animation_start: f32,
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
    utilization: f32,
    utilization_kind: FloatOutBoyStatusProgress,
    utilization_blend: Ratio,
    current_time: f32,
    sensors: (Ratio, Ratio),
    confirmation_progress: f32,
}

impl FloatOutBoyStatusDynamics {
    const fn new() -> Self {
        Self {
            brightness: Ratio::ZERO,
            utilization_blend: Ratio::ZERO,
            idle_blend: Ratio::ZERO,
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
        if !self.idle_blend.is_zero() {
            target_brightness = target_brightness.min(config.status_idle.brightness);
        }
        self.brightness = self
            .brightness
            .slew_toward(target_brightness, LED_FADE_STEP);
        self.brightness
    }

    fn update(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        sensors: (Ratio, Ratio),
        reset_idle: bool,
        current_time: f32,
    ) -> (f32, FloatOutBoyStatusProgress, Ratio, Ratio, f32) {
        let status_config = config.status;
        if reset_idle || input.footpad.is_pressed() {
            self.idle_time = current_time;
        }

        let (duty, motor_current, battery_current) = if input.run_state
            == crate::FloatOutBoyRunState::Running
            && input.mode != crate::FloatOutBoyMode::Flywheel
        {
            (
                (input.duty_cycle.abs() * 10.0 / 9.0).clamp(0.0, 1.0),
                input.motor_current_saturation.clamp(0.0, 1.0),
                input.battery_current_saturation.clamp(0.0, 1.0),
            )
        } else {
            (0.0, 0.0, 0.0)
        };
        let mut utilization = duty;
        let mut utilization_kind = FloatOutBoyStatusProgress::Duty;
        if motor_current > utilization {
            utilization = motor_current;
            utilization_kind = FloatOutBoyStatusProgress::MotorCurrent;
        }
        if battery_current > utilization {
            utilization = battery_current;
            utilization_kind = FloatOutBoyStatusProgress::BatteryCurrent;
        }
        // Upstream 73086101 reuses this serialized field as the motor-utilization threshold.
        let utilization_threshold = status_config.duty_threshold.as_ratio().max(0.15);
        let utilization_target = if utilization > utilization_threshold {
            Ratio::FULL
        } else if utilization < utilization_threshold - 0.1 {
            Ratio::ZERO
        } else {
            self.utilization_blend
        };
        self.utilization_blend = self
            .utilization_blend
            .slew_toward(utilization_target, UTILIZATION_FADE_STEP);

        if sensors.0.is_full() || sensors.1.is_full() {
            self.idle_blend = Ratio::ZERO;
        }
        let idle_timeout = f32::from(status_config.idle_timeout);
        let idle = idle_timeout > 0.0 && current_time - self.idle_time > idle_timeout;
        if idle && self.idle_blend.is_zero() {
            self.animation_start = current_time;
        }
        self.idle_blend = self.idle_blend.slew_toward(idle, LED_FADE_STEP);
        if input.moving {
            self.idle_time = current_time;
        }
        (
            utilization,
            utilization_kind,
            self.utilization_blend,
            self.idle_blend,
            current_time - self.animation_start,
        )
    }
}

/// Pure composed status/front/rear frames for one internal LED configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatOutBoyLedRenderer {
    dynamics: FloatOutBoyLedDynamics,
    status_dynamics: FloatOutBoyStatusDynamics,
    /// Current status-strip frame.
    pub status: FloatOutBoyLedStripFrame,
    /// Current front-strip frame.
    pub front: FloatOutBoyLedStripFrame,
    /// Current rear-strip frame.
    pub rear: FloatOutBoyLedStripFrame,
    front_bar: FloatOutBoyLedBarConfig,
    rear_bar: FloatOutBoyLedBarConfig,
    animation_start: f32,
    confirmation_start: f32,
    front_brightness: Ratio,
    rear_brightness: Ratio,
    status_on_front_blend: Ratio,
    status_on_front_idle_blend: Ratio,
    status_on_front_idle_time: f32,
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
            front_brightness: config.front.brightness,
            rear_brightness: config.rear.brightness,
            status_on_front_blend: Ratio::ZERO,
            status_on_front_idle_blend: Ratio::ZERO,
            status_on_front_idle_time: 0.0,
        }
    }

    /// Advance and compose the front/rear frame in Refloat's transition order.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered LED composition pass"
    )]
    pub fn update(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        current_time: f32,
    ) -> bool {
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
                self.status_on_front_blend = Ratio::FULL;
            }
        }
        if input.footpad.is_pressed() || (!was_upright && upright) {
            self.status_on_front_idle_time = current_time;
        }
        let status_on_front = config.lifted.status_on_front
            && input.run_state == crate::FloatOutBoyRunState::Ready
            && upright;
        let status_brightness = self.status_dynamics.update_brightness(config);
        let front_target = if status_on_front {
            status_brightness
        } else if upright && config.lifted.lights_off {
            Ratio::ZERO
        } else {
            self.front_bar.brightness
        };
        let rear_target = if upright && config.lifted.lights_off {
            Ratio::ZERO
        } else {
            self.rear_bar.brightness
        };
        self.front_brightness = self
            .front_brightness
            .slew_toward(front_target, LED_FADE_STEP);
        self.rear_brightness = self.rear_brightness.slew_toward(rear_target, LED_FADE_STEP);
        self.status_on_front_blend = self
            .status_on_front_blend
            .slew_toward(status_on_front, LED_FADE_STEP);
        if matches!(input.run_state, crate::FloatOutBoyRunState::Disabled) {
            self.status
                .render_disabled(status_brightness * fade, current_time);
            self.front
                .render_disabled(self.front_brightness * fade, current_time);
            self.rear
                .render_disabled(self.rear_brightness * fade, current_time);
            return true;
        }

        let status_config = config.status;
        let sensors = self.dynamics.sensor_fades();
        let (utilization, utilization_kind, utilization_blend, idle_blend, idle_animation_time) =
            self.status_dynamics
                .update(config, input, sensors, upright_changed, current_time);
        let status_layers = FloatOutBoyStatusLayers {
            brightness: self.status_dynamics.brightness,
            fade,
            blend: Ratio::FULL,
            idle_blend,
            idle: FloatOutBoyStatusIdleLayer::Animate(config.status_idle, idle_animation_time),
            reverse: false,
            red_percentage: status_config.red_bar_percentage,
            battery: input.battery_level.clamp(0.0, 1.0),
            utilization,
            utilization_kind,
            utilization_blend,
            current_time,
            sensors,
            confirmation_progress: (current_time - self.confirmation_start) / 0.8,
        };
        self.status.render_status_layers(status_layers);
        if config.lifted.status_on_front && !self.status_on_front_blend.is_zero() {
            let front_idle =
                config.lifted.lights_off && current_time - self.status_on_front_idle_time > 3.0;
            self.status_on_front_idle_blend = self
                .status_on_front_idle_blend
                .slew_toward(front_idle, LED_FADE_STEP);
        }

        let animation_time = current_time - self.animation_start;
        let mut front_bar = self.front_bar;
        front_bar.brightness = self.front_brightness;
        let mut rear_bar = self.rear_bar;
        rear_bar.brightness = self.rear_brightness;
        self.front.render_bar(front_bar, fade, animation_time);
        self.rear.render_bar(rear_bar, fade, animation_time);

        let headlights = self.dynamics.headlights();
        let direction = self.dynamics.direction();
        let seed = crate::wire::saturating_trunc_f32_to_u32(self.animation_start);
        self.compose_transitions(
            config,
            input,
            current_time,
            (seed, fade),
            (old_headlights, headlights),
            (old_direction, direction),
        );
        let blend = self.status_on_front_blend;
        if config.lifted.status_on_front && !blend.is_zero() {
            let mut layers = status_layers;
            layers.brightness = self.front_brightness;
            layers.blend = blend;
            layers.idle_blend = self.status_on_front_idle_blend;
            layers.idle = FloatOutBoyStatusIdleLayer::Black;
            layers.reverse = true;
            self.front.render_status_layers(layers);
        }
        true
    }

    fn compose_transitions(
        &mut self,
        config: FloatOutBoyLedsConfig,
        input: FloatOutBoyLedUpdate,
        current_time: f32,
        (seed, fade): (u32, Ratio),
        (old_headlights, headlights): ((f32, bool, bool), (f32, bool, bool)),
        (old_direction, direction): ((f32, bool), (f32, bool)),
    ) {
        if headlights.2 || old_headlights.2 {
            let targets = if headlights.2 {
                let should_be_on = input.run_state == crate::FloatOutBoyRunState::Running
                    && input.mode != crate::FloatOutBoyMode::Flywheel
                    && config.headlights_on;
                select_front_rear_bars(config, should_be_on, old_direction.1)
            } else {
                select_front_rear_bars(config, headlights.1, direction.1)
            };
            self.render_pair_transition(
                config.headlights_transition,
                headlights.0,
                seed,
                fade,
                targets,
            );
            if !headlights.2 {
                (self.front_bar, self.rear_bar) = targets;
                self.animation_start = current_time;
            }
        }
        if headlights.1 && !headlights.2 && direction.0.to_bits() != old_direction.0.to_bits() {
            let targets = select_front_rear_bars(config, true, !old_direction.1);
            let progress = if old_direction.1 {
                -direction.0
            } else {
                direction.0
            };
            self.render_pair_transition(config.direction_transition, progress, seed, fade, targets);
            if direction.1 != old_direction.1 {
                (self.front_bar, self.rear_bar) = targets;
                self.animation_start = current_time;
            }
        }
    }

    /// Begin Refloat's 0.8-second status-confirm animation.
    pub fn start_confirmation(&mut self, current_time: f32) {
        if current_time - self.confirmation_start > 0.8 {
            self.confirmation_start = current_time;
        }
    }

    fn render_pair_transition(
        &mut self,
        mode: FloatOutBoyLedTransition,
        progress: f32,
        seed: u32,
        fade: Ratio,
        targets: (FloatOutBoyLedBarConfig, FloatOutBoyLedBarConfig),
    ) {
        self.front
            .render_transition(mode, progress, seed, self.front_bar, targets.0, fade);
        self.rear
            .render_transition(mode, progress, seed, self.rear_bar, targets.1, fade);
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
    /// Battery progress.
    Battery,
    /// Duty progress, with a red high end.
    Duty,
    /// Motor-current saturation, with a red high end.
    MotorCurrent,
    /// Battery-current saturation, with a red high end.
    BatteryCurrent,
}

/// Shared brightness and blend inputs for one LED overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatOutBoyLedOverlay {
    brightness: Ratio,
    blend: Ratio,
}

impl FloatOutBoyLedOverlay {
    fn brightness(self, dim: f32) -> Ratio {
        Ratio::clamped(self.brightness.as_ratio() * dim)
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

    /// Return one pixel in physical strip order.
    #[must_use]
    pub fn physical_pixel(&self, index: usize) -> Option<FloatOutBoyLedPixel> {
        let len = usize::from(self.config.count);
        if index >= len {
            return None;
        }
        let logical_index = if self.config.reverse {
            len.saturating_sub(index).saturating_sub(1)
        } else {
            index
        };
        self.pixels.get(logical_index).copied()
    }

    fn pixels_mut(&mut self) -> &mut [FloatOutBoyLedPixel] {
        let len = usize::from(self.config.count);
        self.pixels.get_mut(..len).unwrap_or_default()
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
        let offset = len.div_ceil(2).saturating_sub(1);
        let right_offset = len.saturating_sub(offset).saturating_sub(1);
        let (left, right) = if reverse_roles {
            (right.as_ratio(), left.as_ratio())
        } else {
            (left.as_ratio(), right.as_ratio())
        };
        let blend = Ratio::clamped(overlay.blend.as_ratio().min(left.max(right)));
        let color = FloatOutBoyLedPixel::rgb(0, 0xc0, 0xff);

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
            self.render_pixel_blended(index, color, overlay.brightness(dim), blend);
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
            utilization,
            utilization_kind,
            utilization_blend,
            current_time,
            sensors: (left, right),
            confirmation_progress,
        } = layers;

        if !idle_blend.is_zero() {
            match idle {
                FloatOutBoyStatusIdleLayer::Animate(mut bar, time) => {
                    bar.brightness = brightness;
                    self.render_bar(bar, fade, time);
                }
                FloatOutBoyStatusIdleLayer::Black => {
                    self.render_target(FloatOutBoyLedPixel::default(), brightness * fade, blend);
                }
            }
        }

        let overlay = |amount| FloatOutBoyLedOverlay {
            brightness: brightness * fade,
            blend: amount,
        };
        if !idle_blend.is_full() && !utilization_blend.is_full() {
            self.render_battery_status(
                battery,
                reverse,
                current_time,
                overlay(blend.min(idle_blend.complement())),
            );
        }
        if !idle_blend.is_full() && !utilization_blend.is_zero() {
            self.render_status_progress(
                utilization,
                utilization_kind,
                red_percentage,
                reverse,
                overlay(utilization_blend),
            );
        }
        if !idle_blend.is_full() && (!left.is_zero() || !right.is_zero()) {
            self.render_footpads(left, right, reverse, overlay(blend));
        }
        if (0.0..=1.0).contains(&confirmation_progress) {
            self.render_confirmation(brightness * fade, confirmation_progress);
        }
    }

    /// Paint one Refloat status-utilization progress display over this strip.
    fn render_status_progress(
        &mut self,
        value: f32,
        kind: FloatOutBoyStatusProgress,
        red_percentage: Ratio,
        reverse: bool,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let len = usize::from(self.config.count);
        let len_float = f32::from(self.config.count);
        let progress = len_float * value;
        let offset = usize::from(crate::wire::saturating_trunc_f32_to_u8(progress));
        let remaining = (progress - vescpkg_rs::floor(progress)) * 0.7;
        let red_count = usize::from(crate::wire::saturating_trunc_f32_to_u8(vescpkg_rs::round(
            len_float * red_percentage.as_ratio(),
        )));
        let red_offset = len.saturating_sub(red_count);
        let red = FloatOutBoyLedPixel::rgb(0xff, 0x38, 0x28);
        let base = match kind {
            FloatOutBoyStatusProgress::Battery => FloatOutBoyLedPixel::rgb(0x90, 0x90, 0x90),
            FloatOutBoyStatusProgress::Duty => FloatOutBoyLedPixel::rgb(0xff, 0xb0, 0x30),
            FloatOutBoyStatusProgress::MotorCurrent => FloatOutBoyLedPixel::rgb(0xff, 0x50, 0x90),
            FloatOutBoyStatusProgress::BatteryCurrent => FloatOutBoyLedPixel::rgb(0, 0xff, 0x80),
        };

        for index in 0..len {
            let (target, dim) = if index <= offset {
                let target = if kind != FloatOutBoyStatusProgress::Battery && index >= red_offset {
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
            self.render_pixel_blended(
                physical_index,
                target,
                overlay.brightness(dim),
                overlay.blend,
            );
        }
    }

    /// Paint Refloat's battery bar and pulse one red LED at or below one pixel.
    fn render_battery_status(
        &mut self,
        value: f32,
        reverse: bool,
        current_time: f32,
        overlay: FloatOutBoyLedOverlay,
    ) {
        let battery = value.clamp(0.0, 1.0);
        self.render_status_progress(
            battery,
            FloatOutBoyStatusProgress::Battery,
            Ratio::from_ratio_const(0.0),
            reverse,
            overlay,
        );

        let len = usize::from(self.config.count);
        if len == 0 || battery > 1.0 / f32::from(u8::try_from(len).unwrap_or(u8::MAX)) {
            return;
        }
        let index = if reverse { len.saturating_sub(1) } else { 0 };
        let blink = 0.15 + 0.85 * refloat_cosine_progress(current_time * 2.0);
        self.render_pixel_blended(
            index,
            FloatOutBoyLedPixel::rgb(0xff, 0x50, 0x38),
            overlay.brightness(blink),
            overlay.blend,
        );
    }

    /// Paint Refloat's disabled red pulse.
    pub fn render_disabled(&mut self, brightness: Ratio, time: f32) {
        let red = FloatOutBoyLedPixel::rgb(0xff, 0, 0);
        self.render_pulse_shape(
            red,
            FloatOutBoyLedPixel::default(),
            brightness,
            time / 2.0,
            3.0,
        );
    }

    /// Paint Refloat's status-confirm pulse over this strip.
    pub fn render_confirmation(&mut self, brightness: Ratio, progress: f32) {
        let len = usize::from(self.config.count);
        let len_float = f32::from(self.config.count);
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
        let confirm = FloatOutBoyLedPixel::rgb(0xa0, 0x40, 0xff);

        for index in 0..len {
            let index_float = f32::from(u8::try_from(index).unwrap_or_default());
            let distance = (index_float - offset + 1.0).min(len_float - offset - index_float);
            let target = FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel::default(),
                confirm,
                (distance / feather).clamp(0.0, 1.0),
            );
            self.render_pixel_blended(index, target, brightness, Ratio::clamped(blend));
        }
    }

    /// Render one currently implemented Refloat bar animation.
    pub fn render_bar(&mut self, mut bar: FloatOutBoyLedBarConfig, on_off_fade: Ratio, time: f32) {
        bar.brightness = bar.brightness * on_off_fade;
        let time = time * bar.animation_speed;
        let target = match bar.animation_mode {
            FloatOutBoyLedAnimationMode::Felony => {
                self.render_felony(bar, time);
                return;
            }
            FloatOutBoyLedAnimationMode::RainbowCycle
            | FloatOutBoyLedAnimationMode::RainbowFade
            | FloatOutBoyLedAnimationMode::RainbowRoll => {
                self.render_rainbow(bar, time);
                return;
            }
            FloatOutBoyLedAnimationMode::Pulse => {
                self.render_pulse(bar, time);
                return;
            }
            FloatOutBoyLedAnimationMode::KnightRider => {
                self.render_knight_rider(bar, time);
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
        self.render_target(target, bar.brightness, Ratio::FULL);
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
                let brightness = from_bar
                    .brightness
                    .lerp(to_bar.brightness, blend.as_ratio())
                    * on_off_fade;
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
        let (target, blend, brightness_progress) = if progress <= 0.0 {
            let blend = Ratio::clamped(progress + 1.0);
            (FloatOutBoyLedPixel::default(), blend, blend.as_ratio())
        } else {
            (
                FloatOutBoyLedPixel::blend(
                    FloatOutBoyLedPixel::default(),
                    transition_target(to_bar),
                    progress,
                ),
                Ratio::FULL,
                progress,
            )
        };
        let brightness = from_brightness.lerp(to_bar.brightness, brightness_progress) * on_off_fade;
        self.render_target(target, brightness, blend);
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
        let (first, rest) = map.split_at_mut(len);
        for (value, (first, second)) in
            (0_u8..).zip(first.iter_mut().zip(rest.iter_mut().take(len)))
        {
            *first = value;
            *second = value;
        }
        refloat_sattolo_shuffle(seed, map.get_mut(..len).unwrap_or_default());
        refloat_sattolo_shuffle(
            seed,
            map.get_mut(len..len.saturating_mul(2)).unwrap_or_default(),
        );

        let len_i16 = i16::try_from(len).unwrap_or_default();
        let stop = crate::wire::saturating_trunc_f32_to_i16(progress * f32::from(len_i16));
        let target = transition_target(to_bar);
        let mid_brightness = from_bar.brightness.lerp(to_bar.brightness, 0.5) * on_off_fade;
        let target_brightness = to_bar.brightness * on_off_fade;

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
        self.render_pixel_blended(index, target, brightness, Ratio::FULL);
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

    fn render_pulse(&mut self, bar: FloatOutBoyLedBarConfig, time: f32) {
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        self.render_pulse_shape(primary, secondary, bar.brightness, time, 5.0);
    }

    fn render_pulse_shape(
        &mut self,
        primary: FloatOutBoyLedPixel,
        secondary: FloatOutBoyLedPixel,
        brightness: Ratio,
        time: f32,
        center_divisor: f32,
    ) {
        let len_float = f32::from(self.config.count);
        if len_float == 0.0 {
            return;
        }
        let progress = refloat_cosine_progress(time);
        let center = len_float / center_divisor;
        let length = len_float / 2.0 - center;
        let offset = length * (1.0 - progress);
        let feather = len_float / 4.0;
        let ratio = center / length;
        let fade = if time < ratio { time / ratio } else { 1.0 };
        for (index, pixel) in self.pixels_mut().iter_mut().enumerate() {
            let index = f32::from(u8::try_from(index).unwrap_or_default());
            let distance = (index - offset + 1.0).min(len_float - offset - index);
            let blend = (distance / feather).clamp(0.0, 1.0) * fade;
            let target = FloatOutBoyLedPixel::blend(secondary, primary, blend);
            *pixel = pixel.scaled_and_blended(target, brightness, Ratio::FULL);
        }
    }

    fn render_knight_rider(&mut self, bar: FloatOutBoyLedBarConfig, time: f32) {
        let count = self.config.count;
        if count == 0 {
            return;
        }
        let len_float = f32::from(count);
        let tail = f32::from((count / 3).saturating_add(1));
        let time = time * 0.7;
        let backlight = if time > 0.3 { 0.08 } else { 0.0 };
        let first = len_float * vescpkg_rs::remainder(time, 2.0) - 0.5 * len_float - 1.0;
        let second = 1.5 * len_float - len_float * vescpkg_rs::remainder(time - 1.0, 2.0);
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        for (index, pixel) in self.pixels_mut().iter_mut().enumerate() {
            let index = f32::from(u8::try_from(index).unwrap_or_default());
            let trail_blend = |position, direction| {
                let distance = direction * (position - index);
                if (0.0..=tail).contains(&distance) {
                    (tail - distance) / tail
                } else if distance > -1.0 {
                    distance + 1.0
                } else {
                    backlight
                }
            };
            let blend = trail_blend(first, 1.0_f32).max(trail_blend(second, -1.0));
            let target = FloatOutBoyLedPixel::blend(secondary, primary, blend);
            *pixel = pixel.scaled_and_blended(target, bar.brightness, Ratio::FULL);
        }
    }

    fn render_felony(&mut self, bar: FloatOutBoyLedBarConfig, time: f32) {
        let phase = vescpkg_rs::remainder(time, 0.15);
        let len = usize::from(self.config.count);
        let stop = len / 2;
        let start = stop.saturating_add(len & 1);
        let primary = FloatOutBoyLedPixel::from_named(bar.primary_color);
        let secondary = FloatOutBoyLedPixel::from_named(bar.secondary_color);
        let black = FloatOutBoyLedPixel::default();
        for (index, pixel) in self.pixels_mut().iter_mut().enumerate() {
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
            *pixel = pixel.scaled_and_blended(target, bar.brightness, Ratio::FULL);
        }
    }

    fn render_rainbow(&mut self, bar: FloatOutBoyLedBarConfig, time: f32) {
        let target = match bar.animation_mode {
            FloatOutBoyLedAnimationMode::RainbowCycle => {
                let step = crate::wire::saturating_trunc_f32_to_u8(time * 10.0)
                    .checked_rem(10)
                    .unwrap_or_default();
                let hue = crate::wire::saturating_trunc_f32_to_u8(f32::from(step) * 25.5);
                refloat_hue_to_pixel(hue)
            }
            FloatOutBoyLedAnimationMode::RainbowFade => {
                let hue = crate::wire::saturating_trunc_f32_to_u8(
                    vescpkg_rs::remainder(time, 1.0) * 255.0,
                );
                refloat_hue_to_pixel(hue)
            }
            FloatOutBoyLedAnimationMode::RainbowRoll => {
                let count = self.config.count;
                if count == 0 {
                    return;
                }
                let offset = vescpkg_rs::remainder(time, 1.0) * 255.0;
                for (index, pixel) in self.pixels_mut().iter_mut().enumerate() {
                    let index = u8::try_from(index).unwrap_or_default();
                    let hue = crate::wire::saturating_trunc_f32_to_u8(
                        255.0 / f32::from(count) * f32::from(index) + offset,
                    );
                    *pixel = pixel.scaled_and_blended(
                        refloat_hue_to_pixel(hue),
                        bar.brightness,
                        Ratio::FULL,
                    );
                }
                return;
            }
            FloatOutBoyLedAnimationMode::Solid
            | FloatOutBoyLedAnimationMode::Fade
            | FloatOutBoyLedAnimationMode::Pulse
            | FloatOutBoyLedAnimationMode::Strobe
            | FloatOutBoyLedAnimationMode::KnightRider
            | FloatOutBoyLedAnimationMode::Felony => return,
        };
        self.render_target(target, bar.brightness, Ratio::FULL);
    }

    fn render_target(&mut self, target: FloatOutBoyLedPixel, brightness: Ratio, blend: Ratio) {
        for pixel in self.pixels_mut() {
            *pixel = pixel.scaled_and_blended(target, brightness, blend);
        }
    }
}

fn transition_target(bar: FloatOutBoyLedBarConfig) -> FloatOutBoyLedPixel {
    FloatOutBoyLedPixel::from_named(match bar.animation_mode {
        FloatOutBoyLedAnimationMode::Solid => bar.primary_color,
        _ => bar.secondary_color,
    })
}

/// Select physical front/rear bars from settled headlight and travel direction state.
#[must_use]
pub const fn select_front_rear_bars(
    config: FloatOutBoyLedsConfig,
    headlights_on: bool,
    direction_forward: bool,
) -> (FloatOutBoyLedBarConfig, FloatOutBoyLedBarConfig) {
    match (headlights_on, direction_forward) {
        (false, _) => (config.front, config.rear),
        (true, true) => (config.headlights, config.taillights),
        (true, false) => (config.taillights, config.headlights),
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
    FloatOutBoyLedPixel::rgb(
        channel(tweak(red, 3.2)),
        channel(tweak(green, 2.4)),
        channel(tweak(blue, 2.2)),
    )
}

#[cfg(test)]
mod renderer_tests;
