use core::ops::{Add, Sub};

use crate::{AngleDegrees, AngularVelocity, SampleRate};

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
    use super::{SmoothAngle, angle_step, slew_toward};
    use crate::{AngleDegrees, AngularVelocity, SampleRate};

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
}
