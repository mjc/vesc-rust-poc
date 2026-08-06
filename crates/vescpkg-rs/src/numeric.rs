use core::ops::{Add, Sub};

use crate::{AngleDegrees, AngularVelocity, Frequency, Ratio, Rpm, SampleRate, VescSeconds};

mod smooth_setpoint;
pub use smooth_setpoint::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};

/// Cursor for replacing the oldest value in a small fixed-size ring.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FixedRingIndex<const N: usize>(u8);

impl<const N: usize> FixedRingIndex<N> {
    const VALID_LENGTH: () = assert!(N > 0 && N <= 256);

    /// Replace the current slot, advance with wraparound, and return its old value.
    pub fn replace_and_advance<T: Copy>(&mut self, values: &mut [T; N], value: T) -> T {
        let () = Self::VALID_LENGTH;
        let index = usize::from(self.0);
        let Some(slot) = values.get_mut(index) else {
            return value;
        };
        let previous = core::mem::replace(slot, value);
        self.0 = if index.saturating_add(1) == N {
            0
        } else {
            self.0.saturating_add(1)
        };
        previous
    }
}

/// Allocation-free direct-form biquad low-pass filter.
#[cfg(feature = "math")]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BiquadLowPass {
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    z1: f32,
    z2: f32,
    enabled: bool,
}

#[cfg(feature = "math")]
impl BiquadLowPass {
    /// Configure the cutoff, sample rate, and quality factor.
    pub fn configure(&mut self, frequency: Frequency, sample_rate: SampleRate, quality: f32) {
        self.enabled = frequency.is_positive();
        if !self.enabled {
            return;
        }
        let k = crate::tan(core::f32::consts::PI * frequency.as_hertz() / sample_rate.as_hertz());
        let norm = 1.0 / (1.0 + k / quality + k * k);
        self.a0 = k * k * norm;
        self.a1 = 2.0 * self.a0;
        self.a2 = self.a0;
        self.b1 = 2.0 * (k * k - 1.0) * norm;
        self.b2 = (1.0 - k / quality + k * k) * norm;
    }

    /// Filter one sample, or return it unchanged while disabled.
    pub fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }
        let output = input * self.a0 + self.z1;
        self.z1 = input * self.a1 + self.z2 - self.b1 * output;
        self.z2 = input * self.a2 - self.b2 * output;
        output
    }

    /// Clear filter history without changing its configuration.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Fixed-storage motor speed and acceleration tracker.
///
/// The acceleration value is the rolling average of successive ERPM deltas;
/// absolute ERPM is independently smoothed with the caller's chosen ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotorKinematics<const N: usize> {
    last_erpm: Rpm,
    smoothed_abs_erpm: Rpm,
    absolute_speed_smoothing: Ratio,
    average: Rpm,
    history: [Rpm; N],
    next: usize,
    window: usize,
}

impl<const N: usize> Default for MotorKinematics<N> {
    fn default() -> Self {
        Self {
            last_erpm: Rpm::ZERO,
            smoothed_abs_erpm: Rpm::ZERO,
            absolute_speed_smoothing: Ratio::from_ratio_const(0.0),
            average: Rpm::ZERO,
            history: [Rpm::ZERO; N],
            next: 0,
            window: 1,
        }
    }
}

impl<const N: usize> MotorKinematics<N> {
    /// Configure speed smoothing and the active acceleration window.
    pub fn configure(&mut self, absolute_speed_smoothing: Ratio, window: usize) {
        self.absolute_speed_smoothing = absolute_speed_smoothing;
        self.window = window.clamp(1, N);
        self.reset_acceleration();
    }

    /// Record one electrical motor-speed sample using measured elapsed time.
    pub fn record(&mut self, motor_erpm: Rpm, elapsed: VescSeconds) {
        let previous_abs_erpm = self.smoothed_abs_erpm.as_revolutions_per_minute();
        let current_abs_erpm = motor_erpm.abs().as_revolutions_per_minute();
        self.smoothed_abs_erpm = Rpm::from_revolutions_per_minute(
            previous_abs_erpm
                + self.absolute_speed_smoothing.as_ratio() * (current_abs_erpm - previous_abs_erpm),
        );

        let current = (motor_erpm - self.last_erpm) / elapsed.as_seconds();
        let previous = core::mem::replace(&mut self.history[self.next], current);
        let divisor = f32::from(u16::try_from(self.window).unwrap_or(u16::MAX));
        self.average = self.average + (current - previous) / divisor;
        self.last_erpm = motor_erpm;
        self.next = (self.next + 1) % self.window;
    }

    /// Clear acceleration history while retaining speed smoothing state.
    pub fn reset_acceleration(&mut self) {
        self.average = Rpm::ZERO;
        self.history = [Rpm::ZERO; N];
        self.next = 0;
    }

    /// Return the rolling average ERPM delta.
    #[must_use]
    pub const fn average(&self) -> Rpm {
        self.average
    }

    /// Return the smoothed absolute ERPM.
    #[must_use]
    pub const fn smoothed_abs_erpm(&self) -> Rpm {
        self.smoothed_abs_erpm
    }
}

/// Fixed-state angular motion tracker with degree-wrap correction.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WrappedAngleMotion {
    last: AngleDegrees,
    rate: AngularVelocity,
    aggregate: AngleDegrees,
}

impl WrappedAngleMotion {
    /// Build an explicit motion state for host-side fixtures.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub const fn from_parts(
        last: AngleDegrees,
        rate: AngularVelocity,
        aggregate: AngleDegrees,
    ) -> Self {
        Self {
            last,
            rate,
            aggregate,
        }
    }

    /// Observe an angle, low-pass its wrapped rate, and aggregate sustained motion.
    pub fn observe(
        &mut self,
        angle: AngleDegrees,
        elapsed: VescSeconds,
        rate_limit: AngularVelocity,
        smoothing: Ratio,
        aggregate_threshold: AngularVelocity,
    ) {
        let mut delta = angle - self.last;
        self.last = angle;
        if delta < AngleDegrees::from_degrees(-180.0) {
            delta = delta + AngleDegrees::from_degrees(360.0);
        } else if delta > AngleDegrees::from_degrees(180.0) {
            delta = delta - AngleDegrees::from_degrees(360.0);
        }
        let seconds = elapsed.as_seconds();
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        let limit = rate_limit.abs().as_degrees_per_second();
        let limited = (delta.as_degrees() / seconds).clamp(-limit, limit);
        self.rate = AngularVelocity::from_degrees_per_second(
            self.rate.as_degrees_per_second() * smoothing.complement().as_ratio()
                + limited * smoothing.as_ratio(),
        );
        if self.rate.is_negative() != self.aggregate.is_negative() {
            self.aggregate = AngleDegrees::ZERO;
        }
        if self.rate.abs() > aggregate_threshold {
            self.aggregate = self.aggregate + delta;
        }
    }

    /// Return the filtered wrapped angular rate.
    #[must_use]
    pub const fn rate(&self) -> AngularVelocity {
        self.rate
    }

    /// Return the same-direction accumulated delta.
    #[must_use]
    pub const fn aggregate(&self) -> AngleDegrees {
        self.aggregate
    }

    /// Clear filtered and aggregated motion while retaining the last angle.
    #[cfg(any(test, feature = "test-support"))]
    pub fn clear_motion(&mut self) {
        self.rate = AngularVelocity::ZERO;
        self.aggregate = AngleDegrees::ZERO;
    }
}

/// Move `value` toward `target` by at most `step`.
///
/// `step` is expected to be non-negative. The generic bounds keep this usable
/// for both primitive floats and VESC unit types without discarding units.
pub fn slew_toward<T>(value: T, target: T, step: T) -> T
where
    T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T>,
{
    let difference = if target > value {
        target - value
    } else {
        value - target
    };
    if difference < step {
        target
    } else if target > value {
        value + step
    } else {
        value - step
    }
}

/// Convert an angular velocity and measured elapsed time into one typed control-loop step.
#[must_use]
pub fn angle_step(speed: AngularVelocity, elapsed: VescSeconds) -> AngleDegrees {
    if elapsed.as_seconds().is_finite() && elapsed.is_positive() {
        AngleDegrees::from(speed * elapsed)
    } else {
        AngleDegrees::ZERO
    }
}

/// Return Refloat's frequency-normalized exponential smoothing coefficient.
#[must_use]
pub fn ema_alpha(cutoff: Frequency, update_rate: SampleRate) -> f32 {
    let omega = (2.0 * core::f32::consts::PI * cutoff.as_hertz() / update_rate.as_hertz()).min(0.5);
    omega - 0.5 * omega * omega
}

/// Fixed-state smoothed, rate-limited angle output.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SmoothedAngleSlew {
    ramped_step: AngleDegrees,
    setpoint: AngleDegrees,
}

impl SmoothedAngleSlew {
    /// Advance toward `target` with caller-owned smoothing and center-window policy.
    pub fn advance(
        &mut self,
        target: AngleDegrees,
        step: AngleDegrees,
        smoothing: Ratio,
        center_window: AngleDegrees,
    ) -> AngleDegrees {
        let diff = target - self.setpoint;
        let smoothing = smoothing.as_ratio();
        if diff.abs() < center_window {
            self.ramped_step =
                step * (smoothing * diff.as_degrees() / 2.0) + self.ramped_step * (1.0 - smoothing);
            let centering = self
                .ramped_step
                .abs()
                .min(step * (diff.as_degrees().abs() / 2.0))
                * diff.signum();
            self.setpoint = if diff.abs() < centering.abs() {
                target
            } else {
                self.setpoint + centering
            };
        } else {
            self.ramped_step =
                step * (smoothing * diff.signum()) + self.ramped_step * (1.0 - smoothing);
            self.setpoint = self.setpoint + self.ramped_step;
        }
        self.setpoint
    }

    /// Return the current output without advancing it.
    #[must_use]
    pub const fn setpoint(self) -> AngleDegrees {
        self.setpoint
    }
}

/// State for a smoothed, rate-limited angle target.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SmoothAngle {
    /// Most recently requested target.
    pub target: AngleDegrees,
    /// Smoothed step applied on the last update.
    pub ramped_step: AngleDegrees,
    /// Current rate-limited output.
    pub setpoint: AngleDegrees,
}

impl SmoothAngle {
    /// Advance toward `target` with the source-compatible 1.5 degree center window.
    pub fn advance(&mut self, target: AngleDegrees, step: AngleDegrees, smoothing: f32) {
        self.target = target;
        let diff = target - self.setpoint;
        if diff.abs() < AngleDegrees::from_degrees(1.5) {
            self.ramped_step =
                step * (smoothing * diff.as_degrees() / 2.0) + self.ramped_step * (1.0 - smoothing);
            let centering = self
                .ramped_step
                .abs()
                .min(step * (diff.as_degrees().abs() / 2.0))
                * diff.signum();
            self.setpoint = if diff.abs() < centering.abs() {
                target
            } else {
                self.setpoint + centering
            };
        } else {
            self.ramped_step =
                step * (smoothing * diff.signum()) + self.ramped_step * (1.0 - smoothing);
            self.setpoint = self.setpoint + self.ramped_step;
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "math")]
    use super::BiquadLowPass;
    use super::{
        FixedRingIndex, MotorKinematics, SmoothAngle, SmoothSetpoint, SmoothSetpointConfig,
        SmoothSetpointDirection, SmoothSetpointMultiplier, SmoothedAngleSlew, WrappedAngleMotion,
        angle_step, slew_toward,
    };
    #[cfg(feature = "math")]
    use crate::Frequency;
    use crate::{AngleDegrees, AngularVelocity, Ratio, Rpm, SampleRate, VescSeconds};

    #[test]
    fn smooth_setpoint_preserves_the_refloat_second_order_step() {
        let mut state = SmoothSetpoint::default();
        state.configure(
            SmoothSetpointConfig {
                time_constant: VescSeconds::from_seconds(0.2),
                on_speed_time_constant: VescSeconds::from_seconds(0.08),
                off_speed_time_constant: VescSeconds::from_seconds(0.16),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: AngularVelocity::from_degrees_per_second(24.0),
                off_speed_up: AngularVelocity::from_degrees_per_second(12.0),
                on_speed_down: AngularVelocity::from_degrees_per_second(20.0),
                off_speed_down: AngularVelocity::from_degrees_per_second(10.0),
            },
            SampleRate::from_hertz(500.0),
        );

        state.update(
            AngleDegrees::from_degrees(10.0),
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            VescSeconds::from_seconds(0.002),
        );

        assert!((state.value().as_degrees() - 0.000_112_55).abs() < 0.000_000_1);
    }

    #[test]
    fn typed_angle_ramp_preserves_centering_and_per_sample_step() {
        let step = angle_step(
            AngularVelocity::from_degrees_per_second(30.0),
            VescSeconds::from_seconds(0.01),
        );
        let mut state = SmoothAngle::default();

        state.advance(AngleDegrees::from_degrees(3.0), step, 0.04);
        assert!((step.as_degrees() - 0.3).abs() <= f32::EPSILON);
        assert_eq!(state.target, AngleDegrees::from_degrees(3.0));
        assert!((state.ramped_step.as_degrees() - 0.012).abs() <= f32::EPSILON);
        assert_eq!(state.setpoint, state.ramped_step);
    }

    #[test]
    fn smoothed_angle_slew_keeps_fixed_state_and_custom_center_window() {
        let mut state = SmoothedAngleSlew::default();
        let output = state.advance(
            AngleDegrees::from_degrees(1.0),
            AngleDegrees::from_degrees(0.25),
            Ratio::from_ratio_const(0.02),
            AngleDegrees::from_degrees(2.0),
        );

        assert_eq!(core::mem::size_of::<SmoothedAngleSlew>(), 8);
        assert_eq!(output.as_degrees().to_bits(), 0.0025_f32.to_bits());
    }

    #[test]
    fn clamps_without_overshooting_in_either_direction() {
        assert_eq!(slew_toward(1.0_f32, 1.25, 0.5), 1.25);
        assert_eq!(slew_toward(1.0_f32, 0.75, 0.5), 0.75);
    }

    #[test]
    fn advances_by_one_step_in_either_direction() {
        assert_eq!(slew_toward(1.0_f32, 3.0, 0.5), 1.5);
        assert_eq!(slew_toward(1.0_f32, -1.0, 0.5), 0.5);
    }

    #[test]
    fn preserves_typed_units() {
        assert_eq!(
            slew_toward(
                AngleDegrees::from_degrees(1.0),
                AngleDegrees::from_degrees(3.0),
                AngleDegrees::from_degrees(0.5),
            ),
            AngleDegrees::from_degrees(1.5),
        );
    }

    #[test]
    fn preserves_source_nan_behavior() {
        assert_eq!(slew_toward(1.0_f32, f32::NAN, 0.5), 0.5);
        assert!(slew_toward(f32::NAN, 1.0, 0.5).is_nan());
    }

    #[test]
    fn fixed_ring_index_replaces_the_oldest_slot_and_wraps() {
        let mut values = [0; 3];
        let mut index = FixedRingIndex::default();

        assert_eq!(index.replace_and_advance(&mut values, 1), 0);
        assert_eq!(index.replace_and_advance(&mut values, 2), 0);
        assert_eq!(index.replace_and_advance(&mut values, 3), 0);
        assert_eq!(index.replace_and_advance(&mut values, 4), 1);
        assert_eq!(values, [4, 2, 3]);
    }

    #[test]
    fn motor_kinematics_tracks_smoothed_speed_and_rolling_delta_average() {
        let mut tracker = MotorKinematics::<3>::default();
        let smoothing = Ratio::from_ratio_const(0.1);
        tracker.configure(smoothing, 3);

        tracker.record(
            Rpm::from_revolutions_per_minute(-10.0),
            VescSeconds::from_seconds(1.0),
        );
        tracker.record(
            Rpm::from_revolutions_per_minute(20.0),
            VescSeconds::from_seconds(1.0),
        );
        tracker.record(
            Rpm::from_revolutions_per_minute(50.0),
            VescSeconds::from_seconds(1.0),
        );

        assert_f32_eq!(
            tracker.smoothed_abs_erpm().as_revolutions_per_minute(),
            7.61
        );
        let average = tracker.average().as_revolutions_per_minute();
        assert!((average - 50.0 / 3.0).abs() <= f32::EPSILON * average.abs());

        tracker.record(
            Rpm::from_revolutions_per_minute(110.0),
            VescSeconds::from_seconds(1.0),
        );
        assert_f32_eq!(tracker.average().as_revolutions_per_minute(), 40.0);
    }

    #[test]
    fn motor_kinematics_resets_only_acceleration_history() {
        let mut tracker = MotorKinematics::<2>::default();
        tracker.configure(Ratio::from_ratio_const(0.5), 2);
        tracker.record(
            Rpm::from_revolutions_per_minute(100.0),
            VescSeconds::from_seconds(1.0),
        );

        tracker.reset_acceleration();

        assert_eq!(tracker.average(), Rpm::ZERO);
        assert_eq!(
            tracker.smoothed_abs_erpm(),
            Rpm::from_revolutions_per_minute(50.0),
        );
        tracker.record(
            Rpm::from_revolutions_per_minute(110.0),
            VescSeconds::from_seconds(1.0),
        );
        assert_eq!(tracker.average(), Rpm::from_revolutions_per_minute(5.0));
    }

    #[test]
    #[cfg(feature = "math")]
    fn biquad_low_pass_preserves_disabled_samples_and_layout() {
        let mut filter = BiquadLowPass::default();

        filter.configure(Frequency::ZERO, SampleRate::from_hertz(500.0), 0.707);

        assert_eq!(filter.process(-6.75), -6.75);
        assert_eq!(core::mem::size_of::<BiquadLowPass>(), 32);
    }

    #[test]
    #[cfg(feature = "math")]
    fn biquad_low_pass_filters_and_resets_direct_form_history() {
        let mut filter = BiquadLowPass::default();
        filter.configure(
            Frequency::from_hertz(10.0),
            SampleRate::from_hertz(500.0),
            0.707,
        );

        let first = filter.process(20.0);
        assert!((first - 0.072_432_75).abs() < 0.000_001);
        assert!(filter.process(20.0) > first);

        filter.reset();
        assert_eq!(filter.process(0.0), 0.0);
    }

    #[test]
    fn wrapped_angle_motion_filters_boundary_crossings_and_aggregates_motion() {
        let mut motion = WrappedAngleMotion::default();
        let elapsed = VescSeconds::from_seconds(0.01);
        let limit = AngularVelocity::from_degrees_per_second(10.0);
        let smoothing = Ratio::from_ratio_const(0.2);
        let threshold = AngularVelocity::from_degrees_per_second(1.0);

        motion.observe(
            AngleDegrees::from_degrees(179.95),
            elapsed,
            limit,
            smoothing,
            threshold,
        );
        motion.observe(
            AngleDegrees::from_degrees(-179.95),
            elapsed,
            limit,
            smoothing,
            threshold,
        );

        assert!(motion.rate().is_positive());
        assert!(motion.aggregate().is_positive());
        assert!(motion.rate() <= limit);
    }

    #[test]
    fn wrapped_angle_motion_clears_aggregate_when_filtered_direction_reverses() {
        let mut motion = WrappedAngleMotion::default();
        let elapsed = VescSeconds::from_seconds(1.0);
        let limit = AngularVelocity::from_degrees_per_second(1.0);
        let smoothing = Ratio::from_ratio_const(1.0);
        let threshold = AngularVelocity::from_degrees_per_second(0.01);

        motion.observe(
            AngleDegrees::from_degrees(1.0),
            elapsed,
            limit,
            smoothing,
            threshold,
        );
        motion.observe(AngleDegrees::ZERO, elapsed, limit, smoothing, threshold);

        assert!(motion.rate().is_negative());
        assert_eq!(motion.aggregate(), AngleDegrees::from_degrees(-1.0));
    }

    #[test]
    fn wrapped_angle_motion_preserves_layout_wrap_and_zero_delta_filtering() {
        assert_eq!(core::mem::size_of::<WrappedAngleMotion>(), 12);
        let elapsed = VescSeconds::from_seconds(0.01);
        let limit = AngularVelocity::from_degrees_per_second(10.0);
        let smoothing = Ratio::from_ratio_const(0.2);
        let threshold = AngularVelocity::from_degrees_per_second(4.0);
        let mut wrapped = WrappedAngleMotion::from_parts(
            AngleDegrees::from_degrees(179.95),
            AngularVelocity::ZERO,
            AngleDegrees::ZERO,
        );
        wrapped.observe(
            AngleDegrees::from_degrees(-179.95),
            elapsed,
            limit,
            smoothing,
            threshold,
        );
        assert!((wrapped.rate().as_degrees_per_second() - 2.0).abs() < 0.000_1);

        let mut stationary = WrappedAngleMotion::from_parts(
            AngleDegrees::from_degrees(10.0),
            AngularVelocity::from_degrees_per_second(10.0),
            AngleDegrees::ZERO,
        );
        stationary.observe(
            AngleDegrees::from_degrees(10.0),
            elapsed,
            limit,
            smoothing,
            threshold,
        );
        assert!((stationary.rate().as_degrees_per_second() - 8.0).abs() < 0.000_01);
    }
}
