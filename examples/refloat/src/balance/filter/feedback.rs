#[cfg(any(test, target_arch = "arm"))]
use super::gravity::{AccelMagnitude, PitchGravityError, RollGravityError, YawGravityError};
use super::scalar::AxisScalar;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::AngularVelocity;
pub(crate) use vescpkg_rs::{MahonyPitchGain, MahonyRollGain};
#[cfg(any(test, target_arch = "arm"))]
enum AccelConfidenceTag {}
#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AccelConfidence(AxisScalar<AccelConfidenceTag>);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MahonyFeedbackGains {
    roll: RollAccelCorrectionGain,
    pitch: PitchAccelCorrectionGain,
    yaw: YawAccelCorrectionGain,
}

enum FilteredAccelMagnitudeTag {}
#[cfg(any(test, target_arch = "arm"))]
enum PitchAccelCorrectionGainTag {}
enum PitchFeedbackGainTag {}
#[cfg(any(test, target_arch = "arm"))]
enum RollAccelCorrectionGainTag {}
enum RollFeedbackGainTag {}
#[cfg(any(test, target_arch = "arm"))]
enum YawAccelCorrectionGainTag {}
enum YawFeedbackGainTag {}

// C map: upstream balance_filter keeps Mahony pitch/roll KP as scalar config
// inputs and uses accel confidence as a scalar feedback weight.
type FilteredAccelMagnitude = AxisScalar<FilteredAccelMagnitudeTag>;
#[cfg(any(test, target_arch = "arm"))]
type PitchAccelCorrectionGain = AxisScalar<PitchAccelCorrectionGainTag>;
type PitchFeedbackGain = AxisScalar<PitchFeedbackGainTag>;
#[cfg(any(test, target_arch = "arm"))]
type RollAccelCorrectionGain = AxisScalar<RollAccelCorrectionGainTag>;
type RollFeedbackGain = AxisScalar<RollFeedbackGainTag>;
#[cfg(any(test, target_arch = "arm"))]
type YawAccelCorrectionGain = AxisScalar<YawAccelCorrectionGainTag>;
type YawFeedbackGain = AxisScalar<YawFeedbackGainTag>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AccelConfidenceFilter {
    magnitude: FilteredAccelMagnitude,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MahonyFeedbackConfig {
    pitch: PitchFeedbackGain,
    roll: RollFeedbackGain,
    yaw: YawFeedbackGain,
}

#[cfg(any(test, target_arch = "arm"))]
impl AccelConfidence {
    #[inline(always)]
    pub(super) const fn new(confidence: f32) -> Self {
        // C map: `calculate_acc_confidence` stores the clamped confidence
        // scalar at `third_party/refloat/src/balance_filter.c:42-50`.
        Self(AxisScalar::new(confidence))
    }

    #[inline(always)]
    fn roll_gain(self, gain: RollFeedbackGain) -> RollAccelCorrectionGain {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` turns the
        // filtered accel confidence into the roll correction gain.
        RollAccelCorrectionGain::new(2.0 * gain.0 * self.0.0)
    }

    #[inline(always)]
    fn pitch_gain(self, gain: PitchFeedbackGain) -> PitchAccelCorrectionGain {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` turns the
        // filtered accel confidence into the pitch correction gain.
        PitchAccelCorrectionGain::new(2.0 * gain.0 * self.0.0)
    }

    #[inline(always)]
    fn yaw_gain(self, gain: YawFeedbackGain) -> YawAccelCorrectionGain {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` turns the
        // filtered accel confidence into the yaw correction gain.
        YawAccelCorrectionGain::new(2.0 * gain.0 * self.0.0)
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl MahonyFeedbackGains {
    #[inline(always)]
    pub(super) fn roll_correction(self, error: RollGravityError) -> AngularVelocity {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` applies
        // the roll correction gain to the roll gravity error.
        AngularVelocity::from_radians_per_second(self.roll.0 * error.0)
    }

    #[inline(always)]
    pub(super) fn pitch_correction(self, error: PitchGravityError) -> AngularVelocity {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` applies
        // the pitch correction gain to the pitch gravity error.
        AngularVelocity::from_radians_per_second(self.pitch.0 * error.0)
    }

    #[inline(always)]
    pub(super) fn yaw_correction(self, error: YawGravityError) -> AngularVelocity {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` applies
        // the yaw correction gain to the yaw gravity error.
        AngularVelocity::from_radians_per_second(self.yaw.0 * error.0)
    }
}

impl AccelConfidenceFilter {
    pub(super) const fn source_startup() -> Self {
        // C map: `third_party/refloat/src/balance_filter.c:53-62` initializes
        // accelerometer magnitude confidence state to gravity.
        Self {
            magnitude: FilteredAccelMagnitude::new(1.0),
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn confidence(&mut self, new_acc_mag: AccelMagnitude) -> AccelConfidence {
        // C map: `third_party/refloat/src/balance_filter.c:42-50` filters
        // accelerometer magnitude and clamps the confidence at zero.
        self.magnitude.0 = new_acc_mag.blend_with_filtered(self.magnitude.0);
        AccelConfidence::new((1.0 - 0.02 * sqrt((self.magnitude.0 - 1.0).abs())).max(0.0))
    }
}

impl MahonyFeedbackConfig {
    pub(super) const fn source_startup() -> Self {
        // Source startup mirrors generated Refloat defaults consumed by
        // `third_party/refloat/src/balance_filter.c:64-70`.
        Self {
            pitch: PitchFeedbackGain::new(2.0),
            roll: RollFeedbackGain::new(1.4),
            yaw: YawFeedbackGain::new(1.7),
        }
    }

    pub(super) const fn from_pitch_roll(
        mahony_pitch: MahonyPitchGain,
        mahony_roll: MahonyRollGain,
    ) -> Self {
        // C map: `third_party/refloat/src/balance_filter.c:64-70` copies pitch
        // and roll KP from config, then derives yaw KP from their midpoint.
        let pitch = PitchFeedbackGain::new(mahony_pitch.value());
        let roll = RollFeedbackGain::new(mahony_roll.value());
        Self {
            pitch,
            roll,
            yaw: Self::yaw_from_pitch_roll(pitch, roll),
        }
    }

    const fn yaw_from_pitch_roll(
        pitch_feedback_gain: PitchFeedbackGain,
        roll_feedback_gain: RollFeedbackGain,
    ) -> YawFeedbackGain {
        // C map: `third_party/refloat/src/balance_filter.c:67-70`.
        YawFeedbackGain::new((pitch_feedback_gain.0 + roll_feedback_gain.0) / 2.0)
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn accel_correction_gains(self, confidence: AccelConfidence) -> MahonyFeedbackGains {
        // C map: `third_party/refloat/src/balance_filter.c:87-90` scales the
        // per-axis correction gains by accelerometer confidence.
        MahonyFeedbackGains {
            roll: confidence.roll_gain(self.roll),
            pitch: confidence.pitch_gain(self.pitch),
            yaw: confidence.yaw_gain(self.yaw),
        }
    }
}
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::sqrt;
