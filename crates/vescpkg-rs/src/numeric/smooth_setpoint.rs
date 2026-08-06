use crate::{AngleDegrees, AngularVelocity, Rpm, SampleRate, VescSeconds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Travel direction used to select directional setpoint slew limits.
pub enum SmoothSetpointDirection {
    /// Forward travel.
    Forward,
    /// Reverse travel.
    Reverse,
}

impl SmoothSetpointDirection {
    /// Select a direction from a forward flag.
    #[must_use]
    pub const fn from_forward(forward: bool) -> Self {
        if forward {
            Self::Forward
        } else {
            Self::Reverse
        }
    }

    /// Select forward for non-negative electrical RPM and reverse otherwise.
    #[must_use]
    pub fn from_erpm(erpm: Rpm) -> Self {
        Self::from_forward(!erpm.is_negative())
    }

    /// Return whether this is forward travel.
    #[must_use]
    pub const fn is_forward(self) -> bool {
        matches!(self, Self::Forward)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
/// Per-update multiplier for filter response and speed limits.
pub struct SmoothSetpointMultiplier(f32);

impl Default for SmoothSetpointMultiplier {
    fn default() -> Self {
        Self::ONE
    }
}

impl SmoothSetpointMultiplier {
    /// An unscaled update.
    pub const ONE: Self = Self::from_factor(1.0);

    /// Build a multiplier from its raw factor.
    #[must_use]
    pub const fn from_factor(factor: f32) -> Self {
        Self(factor)
    }

    /// Return the raw multiplier factor.
    #[must_use]
    pub const fn factor(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct FilterAlpha(f32);

impl FilterAlpha {
    fn from_time_constant(time_constant: VescSeconds, frequency: SampleRate) -> Self {
        let omega = (1.0 / (time_constant.as_seconds() * frequency.as_hertz())).min(0.5);
        Self(omega - 0.5 * omega * omega)
    }

    const fn scaled(self, factor: f32) -> Self {
        Self(self.0 * factor)
    }

    const fn factor(self, multiplier: SmoothSetpointMultiplier) -> f32 {
        self.0 * multiplier.factor()
    }

    const fn retained(self) -> f32 {
        1.0 - self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Time constants and directional slew limits for [`SmoothSetpoint`].
pub struct SmoothSetpointConfig {
    /// Target low-pass time constant.
    pub time_constant: VescSeconds,
    /// Engaging-step filter time constant.
    pub on_speed_time_constant: VescSeconds,
    /// Disengaging-step filter time constant.
    pub off_speed_time_constant: VescSeconds,
    /// Exponential wind-down time constant.
    pub winddown_time_constant: VescSeconds,
    /// Engaging slew limit while moving forward.
    pub on_speed_up: AngularVelocity,
    /// Disengaging slew limit while moving forward.
    pub off_speed_up: AngularVelocity,
    /// Engaging slew limit while moving in reverse.
    pub on_speed_down: AngularVelocity,
    /// Disengaging slew limit while moving in reverse.
    pub off_speed_down: AngularVelocity,
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
/// Fixed-state, second-order filtered and directionally rate-limited angle setpoint.
pub struct SmoothSetpoint {
    on_speed_up: AngularVelocity,
    off_speed_up: AngularVelocity,
    on_speed_down: AngularVelocity,
    off_speed_down: AngularVelocity,
    target_alpha: FilterAlpha,
    on_speed_alpha: FilterAlpha,
    off_speed_alpha: FilterAlpha,
    winddown_alpha: FilterAlpha,
    is_winddown: bool,
    filtered_target: AngleDegrees,
    step: AngleDegrees,
    value: AngleDegrees,
}

fn same_source_sign(lhs: AngleDegrees, rhs: AngleDegrees) -> bool {
    lhs.is_negative() == rhs.is_negative()
}

impl SmoothSetpoint {
    /// Update filter coefficients and directional speed limits without resetting live motion.
    pub fn configure(&mut self, config: SmoothSetpointConfig, frequency: SampleRate) {
        self.on_speed_up = config.on_speed_up;
        self.off_speed_up = config.off_speed_up;
        self.on_speed_down = config.on_speed_down;
        self.off_speed_down = config.off_speed_down;
        self.target_alpha =
            FilterAlpha::from_time_constant(config.time_constant, frequency).scaled(2.146);
        self.on_speed_alpha =
            FilterAlpha::from_time_constant(config.on_speed_time_constant, frequency);
        self.off_speed_alpha =
            FilterAlpha::from_time_constant(config.off_speed_time_constant, frequency);
        self.winddown_alpha =
            FilterAlpha::from_time_constant(config.winddown_time_constant, frequency);
    }

    /// Clear live filter motion while retaining the current configuration.
    pub fn reset(&mut self) {
        self.is_winddown = false;
        self.filtered_target = AngleDegrees::ZERO;
        self.step = AngleDegrees::ZERO;
        self.value = AngleDegrees::ZERO;
    }

    /// Advance the setpoint by one measured control-loop interval.
    pub fn update(
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
            + (target - self.filtered_target) * self.target_alpha.factor(multiplier);
        let delta = (self.filtered_target - self.value) * self.target_alpha.factor(multiplier);
        if delta.abs() > self.step.abs() || !same_source_sign(delta, self.step) {
            let alpha = if same_source_sign(self.value, delta) {
                self.on_speed_alpha
            } else {
                self.off_speed_alpha
            };
            self.step = self.step + (delta - self.step) * alpha.factor(multiplier);
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

    /// Exponentially decay the current value toward zero.
    pub fn wind_down(&mut self) {
        self.is_winddown = true;
        self.value = self.value * self.winddown_alpha.retained();
    }

    /// Return the current rate-limited value.
    #[must_use]
    pub const fn value(self) -> AngleDegrees {
        self.value
    }

    /// Replace the live value without changing filter configuration.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn set_value_for_test(&mut self, value: AngleDegrees) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_001;

    fn cutoff_config() -> SmoothSetpointConfig {
        SmoothSetpointConfig {
            time_constant: VescSeconds::from_seconds(0.2),
            on_speed_time_constant: VescSeconds::from_seconds(0.08),
            off_speed_time_constant: VescSeconds::from_seconds(0.16),
            winddown_time_constant: VescSeconds::from_seconds(0.2),
            on_speed_up: AngularVelocity::from_degrees_per_second(24.0),
            off_speed_up: AngularVelocity::from_degrees_per_second(12.0),
            on_speed_down: AngularVelocity::from_degrees_per_second(20.0),
            off_speed_down: AngularVelocity::from_degrees_per_second(10.0),
        }
    }

    fn configured() -> SmoothSetpoint {
        let mut setpoint = SmoothSetpoint::default();
        setpoint.configure(cutoff_config(), SampleRate::from_hertz(500.0));
        setpoint
    }

    fn assert_angle_close(actual: AngleDegrees, expected: f32) {
        assert!((actual.as_degrees() - expected).abs() < EPSILON);
    }

    #[test]
    fn first_update_matches_refloat_second_order_filter() {
        let mut setpoint = configured();

        setpoint.update(
            AngleDegrees::from_degrees(10.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            VescSeconds::from_seconds(0.002),
        );

        assert_angle_close(setpoint.value(), 0.000_112_55);
        assert_angle_close(setpoint.filtered_target, 0.213_527);
    }

    #[test]
    fn time_constant_alpha_uses_refloat_half_omega_cap() {
        let alpha = FilterAlpha::from_time_constant(
            VescSeconds::from_seconds(0.000_1),
            SampleRate::from_hertz(500.0),
        );

        assert!((alpha.0 - 0.375).abs() < f32::EPSILON);
        assert!((alpha.scaled(2.146).0 - 0.804_75).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_limit_selects_directional_on_and_off_speeds() {
        let mut setpoint = configured();
        setpoint.value = AngleDegrees::from_degrees(2.0);

        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Forward,
                AngleDegrees::from_degrees(3.0)
            ),
            AngularVelocity::from_degrees_per_second(24.0)
        );
        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Forward,
                AngleDegrees::from_degrees(1.0)
            ),
            AngularVelocity::from_degrees_per_second(12.0)
        );
        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Reverse,
                AngleDegrees::from_degrees(3.0)
            ),
            AngularVelocity::from_degrees_per_second(20.0)
        );
        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Reverse,
                AngleDegrees::from_degrees(1.0)
            ),
            AngularVelocity::from_degrees_per_second(10.0)
        );
    }

    #[test]
    fn sign_crossing_uses_the_faster_directional_limit() {
        let mut setpoint = configured();
        setpoint.value = AngleDegrees::from_degrees(2.0);

        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Forward,
                AngleDegrees::from_degrees(-3.0)
            ),
            AngularVelocity::from_degrees_per_second(24.0)
        );
        assert_eq!(
            setpoint.speed_limit(
                SmoothSetpointDirection::Reverse,
                AngleDegrees::from_degrees(-3.0)
            ),
            AngularVelocity::from_degrees_per_second(20.0)
        );
    }

    #[test]
    fn speed_limit_caps_a_large_internal_step() {
        let mut setpoint = configured();
        setpoint.filtered_target = AngleDegrees::from_degrees(100.0);
        setpoint.step = AngleDegrees::from_degrees(100.0);

        setpoint.update(
            AngleDegrees::from_degrees(100.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            VescSeconds::from_seconds(0.002),
        );

        assert_angle_close(setpoint.value(), 24.0 / 500.0);
    }

    #[test]
    fn multiplier_scales_filter_and_speed_limit() {
        let elapsed = VescSeconds::from_seconds(0.002);
        let mut normal_filter = configured();
        let mut boosted_filter = normal_filter;

        normal_filter.update(
            AngleDegrees::from_degrees(10.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
        boosted_filter.update(
            AngleDegrees::from_degrees(10.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::from_factor(2.0),
            elapsed,
        );

        assert!(boosted_filter.filtered_target > normal_filter.filtered_target);

        let mut normal_limit = configured();
        normal_limit.filtered_target = AngleDegrees::from_degrees(100.0);
        normal_limit.step = AngleDegrees::from_degrees(100.0);
        let mut boosted_limit = normal_limit;
        normal_limit.update(
            AngleDegrees::from_degrees(100.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
        boosted_limit.update(
            AngleDegrees::from_degrees(100.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::from_factor(2.0),
            elapsed,
        );

        assert_angle_close(
            boosted_limit.value(),
            normal_limit.value().as_degrees() * 2.0,
        );
    }

    #[test]
    fn repeated_winddown_matches_refloat_exponential_decay() {
        let mut setpoint = configured();
        setpoint.value = AngleDegrees::from_degrees(10.0);

        setpoint.wind_down();
        setpoint.wind_down();

        assert_angle_close(setpoint.value(), 10.0 * 0.990_05 * 0.990_05);
        assert!(setpoint.is_winddown);
    }

    #[test]
    fn first_update_after_winddown_restarts_from_the_decayed_value() {
        let mut setpoint = configured();
        setpoint.value = AngleDegrees::from_degrees(10.0);
        setpoint.filtered_target = AngleDegrees::from_degrees(-20.0);
        setpoint.step = AngleDegrees::from_degrees(-5.0);
        setpoint.wind_down();
        let decayed = setpoint.value;

        setpoint.update(
            decayed,
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            VescSeconds::from_seconds(0.002),
        );

        assert_eq!(setpoint.value, decayed);
        assert_eq!(setpoint.filtered_target, decayed);
        assert_eq!(setpoint.step, AngleDegrees::ZERO);
        assert!(!setpoint.is_winddown);
    }

    #[test]
    fn reset_clears_motion_but_retains_configuration() {
        let mut setpoint = configured();
        let configured_speeds = (
            setpoint.on_speed_up,
            setpoint.off_speed_up,
            setpoint.on_speed_down,
            setpoint.off_speed_down,
        );
        setpoint.value = AngleDegrees::from_degrees(3.0);
        setpoint.filtered_target = AngleDegrees::from_degrees(4.0);
        setpoint.step = AngleDegrees::from_degrees(0.5);
        setpoint.is_winddown = true;

        setpoint.reset();

        assert_eq!(setpoint.value, AngleDegrees::ZERO);
        assert_eq!(setpoint.filtered_target, AngleDegrees::ZERO);
        assert_eq!(setpoint.step, AngleDegrees::ZERO);
        assert!(!setpoint.is_winddown);
        assert_eq!(
            configured_speeds,
            (
                setpoint.on_speed_up,
                setpoint.off_speed_up,
                setpoint.on_speed_down,
                setpoint.off_speed_down,
            )
        );
    }

    #[test]
    fn configure_does_not_reset_live_motion() {
        let mut setpoint = configured();
        setpoint.value = AngleDegrees::from_degrees(3.0);
        setpoint.filtered_target = AngleDegrees::from_degrees(4.0);
        setpoint.step = AngleDegrees::from_degrees(0.5);

        setpoint.configure(cutoff_config(), SampleRate::from_hertz(250.0));

        assert_eq!(setpoint.value, AngleDegrees::from_degrees(3.0));
        assert_eq!(setpoint.filtered_target, AngleDegrees::from_degrees(4.0));
        assert_eq!(setpoint.step, AngleDegrees::from_degrees(0.5));
    }

    #[test]
    fn direction_from_erpm_treats_zero_as_forward_like_refloat() {
        assert_eq!(
            SmoothSetpointDirection::from_erpm(Rpm::ZERO),
            SmoothSetpointDirection::Forward
        );
        assert_eq!(
            SmoothSetpointDirection::from_erpm(Rpm::from_revolutions_per_minute(-1.0)),
            SmoothSetpointDirection::Reverse
        );
    }

    #[test]
    fn positive_and_negative_zero_share_refloats_nonnegative_sign() {
        assert!(same_source_sign(
            AngleDegrees::from_degrees(0.0),
            AngleDegrees::from_degrees(-0.0)
        ));
    }
}
