use crate::ema::EmaAlpha;
use vescpkg_rs::prelude::{AngleDegrees, AngularVelocity, Rpm, SampleRate, VescSeconds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SmoothSetpointDirection {
    Forward,
    Reverse,
}

impl SmoothSetpointDirection {
    pub(super) const fn from_forward(forward: bool) -> Self {
        if forward {
            Self::Forward
        } else {
            Self::Reverse
        }
    }

    pub(super) fn from_erpm(erpm: Rpm) -> Self {
        Self::from_forward(!erpm.is_negative())
    }

    pub(super) const fn is_forward(self) -> bool {
        matches!(self, Self::Forward)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(super) struct SmoothSetpointMultiplier(f32);

impl Default for SmoothSetpointMultiplier {
    fn default() -> Self {
        Self::ONE
    }
}

impl SmoothSetpointMultiplier {
    pub(super) const ONE: Self = Self::from_factor(1.0);

    pub(super) const fn from_factor(factor: f32) -> Self {
        Self(factor)
    }

    pub(super) const fn factor(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SmoothSetpointConfig {
    pub(super) time_constant: VescSeconds,
    pub(super) on_speed_time_constant: VescSeconds,
    pub(super) off_speed_time_constant: VescSeconds,
    pub(super) winddown_time_constant: VescSeconds,
    pub(super) on_speed_up: AngularVelocity,
    pub(super) off_speed_up: AngularVelocity,
    pub(super) on_speed_down: AngularVelocity,
    pub(super) off_speed_down: AngularVelocity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetpointPhase {
    Engaging,
    Disengaging,
    Crossing,
}

impl SetpointPhase {
    fn from_angles(value: AngleDegrees, target: AngleDegrees) -> Self {
        if value.as_degrees() * target.as_degrees() < 0.0 {
            Self::Crossing
        } else if value.abs() > target.abs() {
            Self::Disengaging
        } else {
            Self::Engaging
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct SmoothSetpoint {
    on_speed_up: AngularVelocity,
    off_speed_up: AngularVelocity,
    on_speed_down: AngularVelocity,
    off_speed_down: AngularVelocity,
    target_alpha: EmaAlpha,
    on_speed_alpha: EmaAlpha,
    off_speed_alpha: EmaAlpha,
    winddown_alpha: EmaAlpha,
    is_winddown: bool,
    filtered_target: AngleDegrees,
    step: AngleDegrees,
    value: AngleDegrees,
}

fn same_source_sign(lhs: AngleDegrees, rhs: AngleDegrees) -> bool {
    lhs.is_negative() == rhs.is_negative()
}

impl SmoothSetpoint {
    pub(super) fn configure(&mut self, config: SmoothSetpointConfig, frequency: SampleRate) {
        self.on_speed_up = config.on_speed_up;
        self.off_speed_up = config.off_speed_up;
        self.on_speed_down = config.on_speed_down;
        self.off_speed_down = config.off_speed_down;
        self.target_alpha =
            EmaAlpha::from_time_constant(config.time_constant, frequency).scaled(2.146);
        self.on_speed_alpha =
            EmaAlpha::from_time_constant(config.on_speed_time_constant, frequency);
        self.off_speed_alpha =
            EmaAlpha::from_time_constant(config.off_speed_time_constant, frequency);
        self.winddown_alpha =
            EmaAlpha::from_time_constant(config.winddown_time_constant, frequency);
    }

    pub(super) fn reset(&mut self) {
        self.is_winddown = false;
        self.filtered_target = AngleDegrees::ZERO;
        self.step = AngleDegrees::ZERO;
        self.value = AngleDegrees::ZERO;
    }

    pub(super) fn update(
        &mut self,
        target: AngleDegrees,
        direction: SmoothSetpointDirection,
        multiplier: SmoothSetpointMultiplier,
        elapsed: VescSeconds,
    ) {
        if self.is_winddown {
            self.is_winddown = false;
            self.filtered_target = self.value;
            self.step = AngleDegrees::ZERO;
        }

        self.filtered_target = self.filtered_target
            + (target - self.filtered_target) * (self.target_alpha.factor() * multiplier.factor());
        let delta = (self.filtered_target - self.value)
            * (self.target_alpha.factor() * multiplier.factor());
        if delta.abs() > self.step.abs() || !same_source_sign(delta, self.step) {
            let alpha = if same_source_sign(self.value, delta) {
                self.on_speed_alpha
            } else {
                self.off_speed_alpha
            };
            self.step = self.step + (delta - self.step) * (alpha.factor() * multiplier.factor());
        } else {
            self.step = delta;
        }

        let speed_limit = self.speed_limit(direction, target);
        let limited_step = self.step.abs().min(AngleDegrees::from(
            speed_limit * elapsed * multiplier.factor(),
        ));
        self.value = self.value + limited_step * self.step.signum();
    }

    fn speed_limit(
        self,
        direction: SmoothSetpointDirection,
        target: AngleDegrees,
    ) -> AngularVelocity {
        let is_up = self.value.is_negative() != direction.is_forward();
        let (on_speed, off_speed) = if is_up {
            (self.on_speed_up, self.off_speed_up)
        } else {
            (self.on_speed_down, self.off_speed_down)
        };
        match SetpointPhase::from_angles(self.value, target) {
            SetpointPhase::Engaging => on_speed,
            SetpointPhase::Disengaging => off_speed,
            SetpointPhase::Crossing => on_speed.max(off_speed),
        }
    }

    pub(super) fn wind_down(&mut self) {
        self.is_winddown = true;
        self.value = self.value * self.winddown_alpha.retained();
    }

    pub(super) const fn value(self) -> AngleDegrees {
        self.value
    }

    #[cfg(test)]
    pub(super) fn set_value_for_test(&mut self, value: AngleDegrees) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests;
