#![expect(missing_docs, reason = "compact FOB LED data construction API")]

#[cfg(any(test, feature = "test-support"))]
use super::FloatOutBoyLedStatusUpdate;
use super::{
    FloatOutBoyLedAnimationMode, FloatOutBoyLedBarConfig, FloatOutBoyLedColor,
    FloatOutBoyLedColorOrder, FloatOutBoyLedPixel, FloatOutBoyLedRenderer,
    FloatOutBoyLedRuntimeStatus, FloatOutBoyLedStripConfig, FloatOutBoyLedStripFrame,
    FloatOutBoyLedStripOrder, FloatOutBoyLedTransition, FloatOutBoyLedUpdate,
    FloatOutBoyLedsConfig, FloatOutBoyLiftedLedsConfig, FloatOutBoyStatusBarConfig, Ratio,
};

impl FloatOutBoyLedPixel {
    #[must_use]
    pub const fn channels(self) -> [u8; 4] {
        self.channels
    }
}

impl FloatOutBoyLedBarConfig {
    #[must_use]
    pub const fn new(
        brightness: Ratio,
        primary_color: FloatOutBoyLedColor,
        secondary_color: FloatOutBoyLedColor,
        animation_mode: FloatOutBoyLedAnimationMode,
        animation_speed: f32,
    ) -> Self {
        Self {
            brightness,
            primary_color,
            secondary_color,
            animation_mode,
            animation_speed,
        }
    }

    #[must_use]
    pub const fn brightness(self) -> Ratio {
        self.brightness
    }

    #[must_use]
    pub const fn primary_color(self) -> FloatOutBoyLedColor {
        self.primary_color
    }

    #[must_use]
    pub const fn secondary_color(self) -> FloatOutBoyLedColor {
        self.secondary_color
    }

    #[must_use]
    pub const fn animation_mode(self) -> FloatOutBoyLedAnimationMode {
        self.animation_mode
    }

    #[must_use]
    pub const fn animation_speed(self) -> f32 {
        self.animation_speed
    }
}

impl FloatOutBoyStatusBarConfig {
    #[must_use]
    pub const fn new(
        idle_timeout: u16,
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

    #[must_use]
    pub const fn showing_sensors_while_running(mut self) -> Self {
        self.show_sensors_while_running = true;
        self
    }

    #[must_use]
    pub const fn idle_timeout(self) -> u16 {
        self.idle_timeout
    }

    #[must_use]
    pub const fn duty_threshold(self) -> Ratio {
        self.duty_threshold
    }

    #[must_use]
    pub const fn red_bar_percentage(self) -> Ratio {
        self.red_bar_percentage
    }

    #[must_use]
    pub const fn shows_sensors_while_running(self) -> bool {
        self.show_sensors_while_running
    }

    #[must_use]
    pub const fn brightness_headlights_on(self) -> Ratio {
        self.brightness_headlights_on
    }

    #[must_use]
    pub const fn brightness_headlights_off(self) -> Ratio {
        self.brightness_headlights_off
    }
}

impl FloatOutBoyLedsConfig {
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

    #[must_use]
    pub const fn enabled(mut self) -> Self {
        self.on = true;
        self
    }

    #[must_use]
    pub const fn with_headlights_on(mut self) -> Self {
        self.headlights_on = true;
        self
    }

    #[must_use]
    pub const fn with_headlights_transition(
        mut self,
        transition: FloatOutBoyLedTransition,
    ) -> Self {
        self.headlights_transition = transition;
        self
    }

    #[must_use]
    pub const fn with_direction_transition(mut self, transition: FloatOutBoyLedTransition) -> Self {
        self.direction_transition = transition;
        self
    }

    #[must_use]
    pub const fn lights_off_when_lifted(mut self) -> Self {
        self.lifted.lights_off = true;
        self
    }

    #[must_use]
    pub const fn status_on_front_when_lifted(mut self) -> Self {
        self.lifted.status_on_front = true;
        self
    }

    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.on
    }

    #[must_use]
    pub const fn are_headlights_on(self) -> bool {
        self.headlights_on
    }

    #[must_use]
    pub const fn headlights_transition(self) -> FloatOutBoyLedTransition {
        self.headlights_transition
    }

    #[must_use]
    pub const fn direction_transition(self) -> FloatOutBoyLedTransition {
        self.direction_transition
    }

    #[must_use]
    pub const fn turns_lights_off_when_lifted(self) -> bool {
        self.lifted.lights_off
    }

    #[must_use]
    pub const fn shows_status_on_front_when_lifted(self) -> bool {
        self.lifted.status_on_front
    }

    #[must_use]
    pub const fn headlights(self) -> FloatOutBoyLedBarConfig {
        self.headlights
    }

    #[must_use]
    pub const fn taillights(self) -> FloatOutBoyLedBarConfig {
        self.taillights
    }

    #[must_use]
    pub const fn front(self) -> FloatOutBoyLedBarConfig {
        self.front
    }

    #[must_use]
    pub const fn rear(self) -> FloatOutBoyLedBarConfig {
        self.rear
    }

    #[must_use]
    pub const fn status(self) -> FloatOutBoyStatusBarConfig {
        self.status
    }

    #[must_use]
    pub const fn status_idle(self) -> FloatOutBoyLedBarConfig {
        self.status_idle
    }
}

impl FloatOutBoyLedStripConfig {
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

    #[must_use]
    pub const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    #[must_use]
    pub const fn order(self) -> FloatOutBoyLedStripOrder {
        self.order
    }

    #[must_use]
    pub const fn count(self) -> u8 {
        self.count
    }

    #[must_use]
    pub const fn color_order(self) -> FloatOutBoyLedColorOrder {
        self.color_order
    }

    #[must_use]
    pub const fn is_reversed(self) -> bool {
        self.reverse
    }
}

impl FloatOutBoyLedRuntimeStatus {
    #[must_use]
    pub const fn headlights_enabled(self) -> bool {
        self.headlights_enabled
    }
}

impl FloatOutBoyLedUpdate {
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub const fn with_status(
        ride: FloatOutBoyLedUpdate,
        status: FloatOutBoyLedStatusUpdate,
    ) -> Self {
        Self {
            battery_level: status.battery_level,
            duty_cycle: status.duty_cycle,
            motor_current_saturation: status.motor_current_saturation,
            battery_current_saturation: status.battery_current_saturation,
            moving: status.moving,
            ..ride
        }
    }
}

impl FloatOutBoyLedRenderer {
    #[must_use]
    pub const fn confirmation_start_for_test(self) -> f32 {
        self.confirmation_start
    }

    #[must_use]
    pub const fn status(&self) -> &FloatOutBoyLedStripFrame {
        &self.status
    }

    #[must_use]
    pub const fn front(&self) -> &FloatOutBoyLedStripFrame {
        &self.front
    }

    #[must_use]
    pub const fn rear(&self) -> &FloatOutBoyLedStripFrame {
        &self.rear
    }
}

impl FloatOutBoyLedStripFrame {
    #[must_use]
    pub fn set_logical_pixel(&mut self, index: usize, pixel: FloatOutBoyLedPixel) -> bool {
        if index >= usize::from(self.config.count) {
            return false;
        }
        let Some(target) = self.pixels.get_mut(index) else {
            return false;
        };
        *target = pixel;
        true
    }
}
