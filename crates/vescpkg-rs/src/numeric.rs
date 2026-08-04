use core::ops::{Add, Sub};

use crate::{AngleDegrees, AngularVelocity, Ratio, Rpm, SampleRate};

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

/// Fixed-storage motor speed and acceleration tracker.
///
/// The acceleration value is the rolling average of successive ERPM deltas;
/// absolute ERPM is independently smoothed with the caller's chosen ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotorKinematics<const N: usize> {
    last_erpm: Rpm,
    smoothed_abs_erpm: Rpm,
    average: Rpm,
    history: [Rpm; N],
    next: FixedRingIndex<N>,
}

impl<const N: usize> Default for MotorKinematics<N> {
    fn default() -> Self {
        Self {
            last_erpm: Rpm::ZERO,
            smoothed_abs_erpm: Rpm::ZERO,
            average: Rpm::ZERO,
            history: [Rpm::ZERO; N],
            next: FixedRingIndex::default(),
        }
    }
}

impl<const N: usize> MotorKinematics<N> {
    /// Record one electrical motor-speed sample.
    pub fn record(&mut self, motor_erpm: Rpm, absolute_speed_smoothing: Ratio) {
        let previous_abs_erpm = self.smoothed_abs_erpm.as_revolutions_per_minute();
        let current_abs_erpm = motor_erpm.abs().as_revolutions_per_minute();
        self.smoothed_abs_erpm = Rpm::from_revolutions_per_minute(
            previous_abs_erpm
                + absolute_speed_smoothing.as_ratio() * (current_abs_erpm - previous_abs_erpm),
        );

        let current = motor_erpm - self.last_erpm;
        let previous = self.next.replace_and_advance(&mut self.history, current);
        let divisor = f32::from(u16::try_from(N).unwrap_or(u16::MAX));
        self.average = self.average + (current - previous) / divisor;
        self.last_erpm = motor_erpm;
    }

    /// Clear acceleration history while retaining speed smoothing state.
    pub fn reset_acceleration(&mut self) {
        self.average = Rpm::ZERO;
        self.history = [Rpm::ZERO; N];
        self.next = FixedRingIndex::default();
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

/// Convert an angular velocity into one typed control-loop step.
#[must_use]
pub fn angle_step(speed: AngularVelocity, sample_rate: SampleRate) -> AngleDegrees {
    sample_rate
        .sample_period()
        .map_or(AngleDegrees::ZERO, |period| {
            AngleDegrees::from(speed * period)
        })
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
    use super::{FixedRingIndex, MotorKinematics, SmoothAngle, angle_step, slew_toward};
    use crate::{AngleDegrees, AngularVelocity, Ratio, Rpm, SampleRate};

    #[test]
    fn typed_angle_ramp_preserves_centering_and_per_sample_step() {
        let step = angle_step(
            AngularVelocity::from_degrees_per_second(30.0),
            SampleRate::from_hertz(100.0),
        );
        let mut state = SmoothAngle::default();

        state.advance(AngleDegrees::from_degrees(3.0), step, 0.04);
        assert!((step.as_degrees() - 0.3).abs() <= f32::EPSILON);
        assert_eq!(state.target, AngleDegrees::from_degrees(3.0));
        assert!((state.ramped_step.as_degrees() - 0.012).abs() <= f32::EPSILON);
        assert_eq!(state.setpoint, state.ramped_step);
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

        tracker.record(Rpm::from_revolutions_per_minute(-10.0), smoothing);
        tracker.record(Rpm::from_revolutions_per_minute(20.0), smoothing);
        tracker.record(Rpm::from_revolutions_per_minute(50.0), smoothing);

        assert_f32_eq!(
            tracker.smoothed_abs_erpm().as_revolutions_per_minute(),
            7.61
        );
        let average = tracker.average().as_revolutions_per_minute();
        assert!((average - 50.0 / 3.0).abs() <= f32::EPSILON * average.abs());

        tracker.record(Rpm::from_revolutions_per_minute(110.0), smoothing);
        assert_f32_eq!(tracker.average().as_revolutions_per_minute(), 40.0);
    }

    #[test]
    fn motor_kinematics_resets_only_acceleration_history() {
        let mut tracker = MotorKinematics::<2>::default();
        tracker.record(
            Rpm::from_revolutions_per_minute(100.0),
            Ratio::from_ratio_const(0.5),
        );

        tracker.reset_acceleration();

        assert_eq!(tracker.average(), Rpm::ZERO);
        assert_eq!(
            tracker.smoothed_abs_erpm(),
            Rpm::from_revolutions_per_minute(50.0),
        );
        tracker.record(
            Rpm::from_revolutions_per_minute(110.0),
            Ratio::from_ratio_const(0.5),
        );
        assert_eq!(tracker.average(), Rpm::from_revolutions_per_minute(5.0));
    }
}
