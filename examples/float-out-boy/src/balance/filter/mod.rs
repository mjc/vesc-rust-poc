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
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BalanceFilter {
    orientation: EstimatedOrientation,
    accel_confidence: AccelConfidenceFilter,
    feedback: MahonyFeedbackConfig,
}

impl BalanceFilter {
    pub(crate) const fn source_startup() -> Self {
        Self {
            orientation: EstimatedOrientation::source_startup(),
            accel_confidence: AccelConfidenceFilter::source_startup(),
            feedback: MahonyFeedbackConfig::source_startup(),
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_orientation(orientation: ImuOrientation) -> Self {
        Self {
            orientation: EstimatedOrientation::from_orientation(orientation),
            ..Self::source_startup()
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
        let Some((accel_norm, measured_gravity)) = Self::measured_gravity(acceleration) else {
            return gyro.without_accel_feedback();
        };
        let confidence = self.accel_confidence(accel_norm);
        let error = self.accel_error(measured_gravity);

        // C map: `third_party/float-out-boy/src/balance_filter.c:87-111` applies
        // Mahony proportional feedback from accelerometer confidence,
        // measured-vs-estimated gravity error, and per-axis KP.
        gyro.with_gravity_feedback(error, self.feedback_gains(confidence))
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
mod tests {
    use super::feedback::AccelConfidence;
    use super::rate::{
        CorrectedAngularRate, MeasuredAngularRate, PitchAngularRate, RollAngularRate,
        YawAngularRate,
    };
    use super::{BalanceFilter, MahonyPitchGain, MahonyRollGain};
    use vescpkg_rs::prelude::{
        AccelerationG, AngularVelocity, ImuAcceleration, ImuAccelerationX, ImuAccelerationY,
        ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll,
        ImuAngularRateYaw, ImuMagneticField, ImuMagneticFieldX, ImuMagneticFieldY,
        ImuMagneticFieldZ, ImuOrientation, ImuQuaternion, ImuQuaternionW, ImuQuaternionX,
        ImuQuaternionY, ImuQuaternionZ, ImuReadSample, ImuSamplePeriod, MagneticFluxDensity,
        VescSeconds,
    };

    fn imu_accel_x(acceleration: AccelerationG) -> ImuAccelerationX {
        ImuAccelerationX::new(acceleration)
    }

    fn imu_accel_y(acceleration: AccelerationG) -> ImuAccelerationY {
        ImuAccelerationY::new(acceleration)
    }

    fn imu_accel_z(acceleration: AccelerationG) -> ImuAccelerationZ {
        ImuAccelerationZ::new(acceleration)
    }

    fn imu_acceleration(
        x: ImuAccelerationX,
        y: ImuAccelerationY,
        z: ImuAccelerationZ,
    ) -> ImuAcceleration {
        ImuAcceleration::from_axes(x, y, z)
    }

    fn imu_roll_rate(rate: AngularVelocity) -> ImuAngularRateRoll {
        ImuAngularRateRoll::new(rate)
    }

    fn imu_pitch_rate(rate: AngularVelocity) -> ImuAngularRatePitch {
        ImuAngularRatePitch::new(rate)
    }

    fn imu_yaw_rate(rate: AngularVelocity) -> ImuAngularRateYaw {
        ImuAngularRateYaw::new(rate)
    }

    fn imu_angular_rate(
        roll: ImuAngularRateRoll,
        pitch: ImuAngularRatePitch,
        yaw: ImuAngularRateYaw,
    ) -> ImuAngularRate {
        ImuAngularRate::from_axes(roll, pitch, yaw)
    }

    fn imu_period(period: VescSeconds) -> ImuSamplePeriod {
        ImuSamplePeriod::new(period)
    }

    fn imu_magnetic_field() -> ImuMagneticField {
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
        )
    }

    fn imu_sample(
        acceleration: ImuAcceleration,
        angular_rate: ImuAngularRate,
        period: ImuSamplePeriod,
    ) -> ImuReadSample {
        ImuReadSample::from_parts(acceleration, angular_rate, imu_magnetic_field(), period)
    }

    #[test]
    fn balance_filter_update_integrates_positive_pitch_like_float_out_boy_callback() {
        let mut filter = BalanceFilter::source_startup();

        filter.update(imu_sample(
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(0.0)),
                imu_accel_y(AccelerationG::from_g(0.0)),
                imu_accel_z(AccelerationG::from_g(1.0)),
            ),
            imu_angular_rate(
                imu_roll_rate(AngularVelocity::from_radians_per_second(0.0)),
                imu_pitch_rate(AngularVelocity::from_radians_per_second(1.0)),
                imu_yaw_rate(AngularVelocity::from_radians_per_second(0.0)),
            ),
            imu_period(VescSeconds::from_seconds(0.1)),
        ));

        // Float Out Boy's `imu_ref_callback` forwards gyro/accel/dt at
        // `third_party/float-out-boy/src/main.c:760-765`; `balance_filter_update` integrates the
        // quaternion at `third_party/float-out-boy/src/balance_filter.c:73-134`, and
        // `balance_filter_get_pitch` reads it at `third_party/float-out-boy/src/balance_filter.c:145-154`.
        assert!(filter.balance_pitch().angle().as_radians() > 0.0);
    }

    #[test]
    fn balance_filter_pitch_clamps_quaternion_projection_like_float_out_boy() {
        let positive = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
            ImuQuaternion::from_components(
                ImuQuaternionW::new(1.0),
                ImuQuaternionX::new(0.0),
                ImuQuaternionY::new(1.0),
                ImuQuaternionZ::new(0.0),
            ),
        ));
        let negative = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
            ImuQuaternion::from_components(
                ImuQuaternionW::new(-1.0),
                ImuQuaternionX::new(0.0),
                ImuQuaternionY::new(1.0),
                ImuQuaternionZ::new(0.0),
            ),
        ));

        // Float Out Boy clamps the asin input before converting to pitch at
        // `third_party/float-out-boy/src/balance_filter.c:145-154`.
        assert_f32_eq!(
            positive.balance_pitch().angle().as_radians(),
            core::f32::consts::FRAC_PI_2
        );
        assert_f32_eq!(
            negative.balance_pitch().angle().as_radians(),
            -core::f32::consts::FRAC_PI_2
        );
    }

    #[test]
    fn balance_filter_configures_yaw_kp_from_pitch_and_roll_like_float_out_boy() {
        let mut filter = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
            ImuQuaternion::from_components(
                ImuQuaternionW::new(1.0),
                ImuQuaternionX::new(2.0),
                ImuQuaternionY::new(0.0),
                ImuQuaternionZ::new(0.0),
            ),
        ));
        let (_, measured_gravity) = BalanceFilter::measured_gravity(imu_acceleration(
            imu_accel_x(AccelerationG::from_g(1.0)),
            imu_accel_y(AccelerationG::from_g(0.0)),
            imu_accel_z(AccelerationG::from_g(0.0)),
        ))
        .expect("nonzero accel normalizes");
        let gravity_error = filter.accel_error(measured_gravity);

        filter.configure(MahonyPitchGain::new(4.0), MahonyRollGain::new(2.0));

        // Float Out Boy averages pitch and roll KP for yaw at
        // `third_party/float-out-boy/src/balance_filter.c:64-70`.
        let corrected = MeasuredAngularRate::new(
            RollAngularRate::new(AngularVelocity::from_radians_per_second(10.0)),
            PitchAngularRate::new(AngularVelocity::from_radians_per_second(10.0)),
            YawAngularRate::new(AngularVelocity::from_radians_per_second(10.0)),
        )
        .with_gravity_feedback(
            gravity_error,
            filter.feedback_gains(AccelConfidence::new(0.5)),
        );

        assert!((corrected.yaw().as_radians_per_second() - 16.0).abs() < 0.000_001);
    }

    #[test]
    fn balance_filter_normalizes_accel_before_correction_like_float_out_boy() {
        let (_, unit) = BalanceFilter::measured_gravity(imu_acceleration(
            imu_accel_x(AccelerationG::from_g(0.0)),
            imu_accel_y(AccelerationG::from_g(0.0)),
            imu_accel_z(AccelerationG::from_g(2.0)),
        ))
        .expect("nonzero accel normalizes");

        assert_f32_eq!(unit.x(), 0.0);
        assert_f32_eq!(unit.y(), 0.0);
        assert_f32_eq!(unit.z(), 1.0);
    }

    #[test]
    fn balance_filter_skips_accel_correction_for_tiny_sample_like_float_out_boy() {
        let mut filter = BalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            MeasuredAngularRate::new(
                RollAngularRate::new(AngularVelocity::from_radians_per_second(1.0)),
                PitchAngularRate::new(AngularVelocity::from_radians_per_second(2.0)),
                YawAngularRate::new(AngularVelocity::from_radians_per_second(3.0)),
            ),
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(0.0)),
                imu_accel_y(AccelerationG::from_g(0.0)),
                imu_accel_z(AccelerationG::from_g(0.005)),
            ),
        );

        assert!((gyro.roll().as_radians_per_second() - 1.0).abs() < 0.000_001);
        assert!((gyro.pitch().as_radians_per_second() - 2.0).abs() < 0.000_001);
        assert!((gyro.yaw().as_radians_per_second() - 3.0).abs() < 0.000_001);
    }

    #[test]
    fn balance_filter_applies_gravity_error_feedback_like_float_out_boy() {
        let mut filter = BalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            MeasuredAngularRate::new(
                RollAngularRate::new(AngularVelocity::from_radians_per_second(1.0)),
                PitchAngularRate::new(AngularVelocity::from_radians_per_second(2.0)),
                YawAngularRate::new(AngularVelocity::from_radians_per_second(3.0)),
            ),
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(0.0)),
                imu_accel_y(AccelerationG::from_g(1.0)),
                imu_accel_z(AccelerationG::from_g(0.0)),
            ),
        );

        assert!((gyro.roll().as_radians_per_second() - 2.4).abs() < 0.000_001);
        assert!((gyro.pitch().as_radians_per_second() - 2.0).abs() < 0.000_001);
        assert!((gyro.yaw().as_radians_per_second() - 3.0).abs() < 0.000_001);
    }

    #[test]
    fn balance_filter_integrates_gyro_components_like_float_out_boy() {
        let mut filter = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
            ImuQuaternion::from_components(
                ImuQuaternionW::new(1.0),
                ImuQuaternionX::new(2.0),
                ImuQuaternionY::new(3.0),
                ImuQuaternionZ::new(4.0),
            ),
        ));

        filter.integrate_gyro(
            CorrectedAngularRate::new(
                RollAngularRate::new(AngularVelocity::from_radians_per_second(0.2)),
                PitchAngularRate::new(AngularVelocity::from_radians_per_second(0.4)),
                YawAngularRate::new(AngularVelocity::from_radians_per_second(0.6)),
            ),
            vescpkg_rs::prelude::VescSeconds::from_seconds(0.5),
        );

        let [scalar, body_x, body_y, body_z] = filter.estimated_orientation().wxyz_for_test();
        assert!((scalar - 0.0).abs() < 0.000_001);
        assert!((body_x - 2.1).abs() < 0.000_001);
        assert!((body_y - 3.0).abs() < 0.000_001);
        assert!((body_z - 4.2).abs() < 0.000_001);
    }
}
