use super::*;

impl FloatOutBoyLedPixel {
    pub(crate) const fn channels(self) -> [u8; 4] {
        self.channels
    }
}

impl FloatOutBoyLedBarConfig {
    pub(crate) const fn new(
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

    pub(crate) const fn brightness(self) -> Ratio {
        self.brightness
    }

    pub(crate) const fn primary_color(self) -> FloatOutBoyLedColor {
        self.primary_color
    }

    pub(crate) const fn secondary_color(self) -> FloatOutBoyLedColor {
        self.secondary_color
    }

    pub(crate) const fn animation_mode(self) -> FloatOutBoyLedAnimationMode {
        self.animation_mode
    }

    pub(crate) const fn animation_speed(self) -> f32 {
        self.animation_speed
    }
}

impl FloatOutBoyStatusBarConfig {
    pub(crate) const fn new(
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

    pub(crate) const fn showing_sensors_while_running(mut self) -> Self {
        self.show_sensors_while_running = true;
        self
    }

    pub(crate) const fn idle_timeout(self) -> u16 {
        self.idle_timeout
    }

    pub(crate) const fn duty_threshold(self) -> Ratio {
        self.duty_threshold
    }

    pub(crate) const fn red_bar_percentage(self) -> Ratio {
        self.red_bar_percentage
    }

    pub(crate) const fn shows_sensors_while_running(self) -> bool {
        self.show_sensors_while_running
    }

    pub(crate) const fn brightness_headlights_on(self) -> Ratio {
        self.brightness_headlights_on
    }

    pub(crate) const fn brightness_headlights_off(self) -> Ratio {
        self.brightness_headlights_off
    }
}

impl FloatOutBoyLedsConfig {
    pub(crate) const fn new(
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

    pub(crate) const fn enabled(mut self) -> Self {
        self.on = true;
        self
    }

    pub(crate) const fn with_headlights_on(mut self) -> Self {
        self.headlights_on = true;
        self
    }

    pub(crate) const fn with_headlights_transition(
        mut self,
        transition: FloatOutBoyLedTransition,
    ) -> Self {
        self.headlights_transition = transition;
        self
    }

    pub(crate) const fn with_direction_transition(
        mut self,
        transition: FloatOutBoyLedTransition,
    ) -> Self {
        self.direction_transition = transition;
        self
    }

    pub(crate) const fn lights_off_when_lifted(mut self) -> Self {
        self.lifted.lights_off = true;
        self
    }

    pub(crate) const fn status_on_front_when_lifted(mut self) -> Self {
        self.lifted.status_on_front = true;
        self
    }

    pub(crate) const fn is_enabled(self) -> bool {
        self.on
    }

    pub(crate) const fn are_headlights_on(self) -> bool {
        self.headlights_on
    }

    pub(crate) const fn headlights_transition(self) -> FloatOutBoyLedTransition {
        self.headlights_transition
    }

    pub(crate) const fn direction_transition(self) -> FloatOutBoyLedTransition {
        self.direction_transition
    }

    pub(crate) const fn turns_lights_off_when_lifted(self) -> bool {
        self.lifted.lights_off
    }

    pub(crate) const fn shows_status_on_front_when_lifted(self) -> bool {
        self.lifted.status_on_front
    }

    pub(crate) const fn headlights(self) -> FloatOutBoyLedBarConfig {
        self.headlights
    }

    pub(crate) const fn taillights(self) -> FloatOutBoyLedBarConfig {
        self.taillights
    }

    pub(crate) const fn front(self) -> FloatOutBoyLedBarConfig {
        self.front
    }

    pub(crate) const fn rear(self) -> FloatOutBoyLedBarConfig {
        self.rear
    }

    pub(crate) const fn status(self) -> FloatOutBoyStatusBarConfig {
        self.status
    }

    pub(crate) const fn status_idle(self) -> FloatOutBoyLedBarConfig {
        self.status_idle
    }
}

impl FloatOutBoyLedStripConfig {
    pub(crate) const fn new(
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

    pub(crate) const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub(crate) const fn order(self) -> FloatOutBoyLedStripOrder {
        self.order
    }

    pub(crate) const fn count(self) -> u8 {
        self.count
    }

    pub(crate) const fn color_order(self) -> FloatOutBoyLedColorOrder {
        self.color_order
    }

    pub(crate) const fn is_reversed(self) -> bool {
        self.reverse
    }
}

impl FloatOutBoyLedRuntimeStatus {
    pub(crate) const fn headlights_enabled(self) -> bool {
        self.headlights_enabled
    }
}

impl FloatOutBoyLedUpdate {
    pub(crate) const fn with_status(
        ride: FloatOutBoyLedUpdate,
        status: FloatOutBoyLedStatusUpdate,
    ) -> Self {
        Self {
            battery_level: status.battery_level,
            duty_cycle: status.duty_cycle,
            moving: status.moving,
            ..ride
        }
    }
}

impl FloatOutBoyLedRenderer {
    pub(crate) const fn confirmation_start_for_test(self) -> f32 {
        self.confirmation_start
    }

    pub(crate) const fn status(&self) -> &FloatOutBoyLedStripFrame {
        &self.status
    }

    pub(crate) const fn front(&self) -> &FloatOutBoyLedStripFrame {
        &self.front
    }

    pub(crate) const fn rear(&self) -> &FloatOutBoyLedStripFrame {
        &self.rear
    }
}

impl FloatOutBoyLedStripFrame {
    pub(crate) fn set_logical_pixel(&mut self, index: usize, pixel: FloatOutBoyLedPixel) -> bool {
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
