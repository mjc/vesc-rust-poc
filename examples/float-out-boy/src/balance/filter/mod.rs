use crate::config::FloatOutBoyFilterConfig;
use crate::domain::FloatOutBoyRealtimeBalancePitch;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::ImuOrientation;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{AccelerationG, ImuAcceleration, ImuReadSample};

mod feedback;
#[cfg(any(test, target_arch = "arm"))]
mod gravity;
mod orientation;
#[cfg(any(test, target_arch = "arm"))]
mod rate;
mod scalar;

#[cfg(any(test, target_arch = "arm"))]
use feedback::{AccelConfidence, MahonyFeedbackGains};
use feedback::{AccelConfidenceFilter, MahonyFeedbackConfig};
#[cfg(any(test, target_arch = "arm"))]
use gravity::{GravityError, MeasuredGravity};
use orientation::EstimatedOrientation;
#[cfg(any(test, target_arch = "arm"))]
use rate::{CorrectedAngularRate, MeasuredAngularRate};
use vescpkg_rs::{MahonyPitchGain, MahonyRollGain};

/// Float Out Boy-owned balance filter state.
///
/// C map: `BalanceFilterData` is initialized from firmware quaternions at
/// `third_party/float-out-boy/src/balance_filter.c:53-61`, configured at `third_party/float-out-boy/src/balance_filter.c:64-70`,
/// updated from `imu_ref_callback` at `third_party/float-out-boy/src/main.c:760-765`, and read by
/// `imu_update` at `third_party/float-out-boy/src/imu.c:35-41`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct BalanceFilter {
    orientation: EstimatedOrientation,
    accel_confidence: AccelConfidenceFilter,
    feedback: MahonyFeedbackConfig,
}

impl BalanceFilter {
    #[cfg(test)]
    pub(crate) fn source_startup() -> Self {
        Self::default()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_orientation(orientation: ImuOrientation) -> Self {
        Self {
            orientation: EstimatedOrientation::from_orientation(orientation),
            ..Self::default()
        }
    }

    pub(crate) fn configure(&mut self, mahony_kp: MahonyPitchGain, mahony_kp_roll: MahonyRollGain) {
        // Float Out Boy copies `mahony_kp`/`mahony_kp_roll` into the filter and
        // averages yaw KP at `third_party/float-out-boy/src/balance_filter.c:64-70`.
        self.feedback = MahonyFeedbackConfig::from_pitch_roll(mahony_kp, mahony_kp_roll);
    }

    pub(crate) fn configure_from(&mut self, config: FloatOutBoyFilterConfig<'_>) {
        self.configure(config.mahony_kp(), config.mahony_kp_roll());
    }

    #[cfg(test)]
    pub(crate) const fn configured_gains(&self) -> (MahonyPitchGain, MahonyRollGain) {
        self.feedback.configured_gains()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn update(&mut self, sample: ImuReadSample) {
        // Float Out Boy's callback feeds gyro first, accel second at
        // `third_party/float-out-boy/src/main.c:760-765`; the Mahony update itself is
        // `third_party/float-out-boy/src/balance_filter.c:73-134`.
        let gyro =
            self.gyro_with_accel_correction(sample.angular_rate().into(), sample.acceleration());
        self.integrate_gyro(gyro, sample.period().duration());
        self.normalize_quaternion();
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:145-154`.
    pub(crate) fn balance_pitch(&self) -> FloatOutBoyRealtimeBalancePitch {
        self.estimated_orientation().balance_pitch()
    }

    #[inline]
    const fn estimated_orientation(&self) -> EstimatedOrientation {
        self.orientation
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn gyro_with_accel_correction(
        &mut self,
        gyro: MeasuredAngularRate,
        acceleration: ImuAcceleration,
    ) -> CorrectedAngularRate {
        Self::measured_gravity(acceleration).map_or_else(
            || gyro.without_accel_feedback(),
            |(accel_norm, measured_gravity)| {
                let confidence = self.accel_confidence(accel_norm);
                let error = self.accel_error(measured_gravity);

                // C map: `third_party/float-out-boy/src/balance_filter.c:87-111` applies
                // Mahony proportional feedback from accelerometer confidence,
                // measured-vs-estimated gravity error, and per-axis KP.
                gyro.with_gravity_feedback(error, self.feedback_gains(confidence))
            },
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn measured_gravity(acceleration: ImuAcceleration) -> Option<(AccelerationG, MeasuredGravity)> {
        // C map: `third_party/float-out-boy/src/balance_filter.c:82-96` enters
        // feedback only when accel norm is above 0.01, then normalizes it.
        MeasuredGravity::from_acceleration(acceleration)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, accel: MeasuredGravity) -> GravityError {
        // C map: `third_party/float-out-boy/src/balance_filter.c:98-101` projects
        // the current estimated orientation into a gravity half-vector.
        let estimated_gravity = self.estimated_orientation().estimated_half_gravity();

        // C map: `third_party/float-out-boy/src/balance_filter.c:103-106` crosses
        // measured gravity (accelerometer) against estimated gravity.
        accel.error_against(estimated_gravity)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, gyro: CorrectedAngularRate, dt: vescpkg_rs::prelude::VescSeconds) {
        // C map: `third_party/float-out-boy/src/balance_filter.c:114-117`
        // pre-multiplies gyro by half the tick duration.
        let gyro_half_step = gyro.half_step(dt);

        // C map: `third_party/float-out-boy/src/balance_filter.c:118-124`
        // integrates q_dot = 0.5 * q * gyro in upstream component order.
        let orientation_change = self.orientation.change_from_angular_rate(gyro_half_step);
        self.orientation.apply_change(orientation_change);
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalize_quaternion(&mut self) {
        // C map: `third_party/float-out-boy/src/balance_filter.c:126-133` keeps the
        // integrated orientation on the unit-quaternion sphere.
        self.orientation.normalize();
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, new_acc_mag: AccelerationG) -> AccelConfidence {
        // C map: `third_party/float-out-boy/src/balance_filter.c:42-50` filters the
        // accelerometer magnitude and decays confidence toward zero.
        self.accel_confidence.confidence(new_acc_mag)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn feedback_gains(&self, confidence: AccelConfidence) -> MahonyFeedbackGains {
        // C map: `third_party/float-out-boy/src/balance_filter.c:87-90` scales the
        // Mahony feedback gains by the current accelerometer confidence.
        self.feedback.accel_correction_gains(confidence)
    }
}

#[cfg(test)]
mod tests;
