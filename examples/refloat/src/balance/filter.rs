use crate::domain::RefloatRealtimeBalancePitch;
use vescpkg_rs::prelude::AngleRadians;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{
    ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRate,
    ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw, ImuReadSample,
};

#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_inv_sqrt(value: f32) -> f32 {
    // Refloat uses `1.0 / sqrtf(x)` at `third_party/refloat/src/balance_filter.c:38-40`.
    1.0 / libm::sqrtf(value)
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatAccelSample([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatAccelMagnitude(f32);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatMeasuredGravity([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatHalfGravity([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatGravityError([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatAccelConfidence(f32);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatFeedbackGains {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatGyroRate([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatCorrectedGyroRate([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatGyroHalfStep([f32; 3]);

#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatQuaternion([f32; 4]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatQuaternionDelta([f32; 4]);

#[cfg(any(test, target_arch = "arm"))]
impl RefloatAccelSample {
    #[inline(always)]
    const fn new(xyz: [f32; 3]) -> Self {
        Self(xyz)
    }

    fn from_axes(x: ImuAccelerationX, y: ImuAccelerationY, z: ImuAccelerationZ) -> Self {
        Self::new([
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        ])
    }

    /// C map: `third_party/refloat/src/balance_filter.c:82-96`.
    #[inline(always)]
    fn normalized_for_feedback(self) -> Option<(RefloatAccelMagnitude, RefloatMeasuredGravity)> {
        let length_squared = self.length_squared();
        let accel_norm = libm::sqrtf(length_squared);
        match accel_norm {
            norm if norm > 0.01 => Some((
                RefloatAccelMagnitude(norm),
                RefloatMeasuredGravity(self.scaled(refloat_inv_sqrt(length_squared))),
            )),
            _ => None,
        }
    }

    #[inline(always)]
    fn scaled(self, scale: f32) -> [f32; 3] {
        let [x, y, z] = self.0;
        [x * scale, y * scale, z * scale]
    }

    #[inline(always)]
    fn length_squared(self) -> f32 {
        let [x, y, z] = self.0;
        x * x + y * y + z * z
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl From<ImuAcceleration> for RefloatAccelSample {
    #[inline(always)]
    fn from(acceleration: ImuAcceleration) -> Self {
        acceleration.map_axes(Self::from_axes)
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatAccelMagnitude {
    #[inline(always)]
    const fn as_f32(self) -> f32 {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatMeasuredGravity {
    /// C map: `third_party/refloat/src/balance_filter.c:103-106`.
    #[inline(always)]
    fn error_against(self, estimated_gravity: RefloatHalfGravity) -> RefloatGravityError {
        let [x, y, z] = self.0;
        let [rhs_x, rhs_y, rhs_z] = estimated_gravity.0;
        RefloatGravityError([
            y * rhs_z - z * rhs_y,
            z * rhs_x - x * rhs_z,
            x * rhs_y - y * rhs_x,
        ])
    }

    #[cfg(test)]
    const fn xyz(self) -> [f32; 3] {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatGravityError {
    #[inline(always)]
    const fn xyz(self) -> [f32; 3] {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatAccelConfidence {
    #[inline(always)]
    const fn as_f32(self) -> f32 {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatGyroRate {
    #[inline(always)]
    const fn new(xyz: [f32; 3]) -> Self {
        Self(xyz)
    }

    fn from_axes(
        roll: ImuAngularRateRoll,
        pitch: ImuAngularRatePitch,
        yaw: ImuAngularRateYaw,
    ) -> Self {
        Self::new([
            roll.angular_velocity().as_degrees_per_second(),
            pitch.angular_velocity().as_degrees_per_second(),
            yaw.angular_velocity().as_degrees_per_second(),
        ])
    }

    #[inline(always)]
    const fn without_accel_feedback(self) -> RefloatCorrectedGyroRate {
        RefloatCorrectedGyroRate(self.0)
    }

    /// C map: `third_party/refloat/src/balance_filter.c:107-111`.
    #[inline(always)]
    fn with_gravity_feedback(
        self,
        error: RefloatGravityError,
        gains: RefloatFeedbackGains,
    ) -> RefloatCorrectedGyroRate {
        let [gx, gy, gz] = self.0;
        let [halfex, halfey, halfez] = error.xyz();
        RefloatCorrectedGyroRate([
            gx + gains.roll * halfex,
            gy + gains.pitch * halfey,
            gz + gains.yaw * halfez,
        ])
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl From<ImuAngularRate> for RefloatGyroRate {
    #[inline(always)]
    fn from(angular_rate: ImuAngularRate) -> Self {
        angular_rate.map_axes(Self::from_axes)
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatCorrectedGyroRate {
    #[cfg(test)]
    const fn new(xyz: [f32; 3]) -> Self {
        Self(xyz)
    }

    /// C map: `third_party/refloat/src/balance_filter.c:114-117`.
    #[inline(always)]
    fn half_step(self, dt: f32) -> RefloatGyroHalfStep {
        let [gx, gy, gz] = self.0;
        RefloatGyroHalfStep([gx * 0.5 * dt, gy * 0.5 * dt, gz * 0.5 * dt])
    }

    #[cfg(test)]
    const fn xyz(self) -> [f32; 3] {
        self.0
    }
}

impl RefloatQuaternion {
    #[inline(always)]
    const fn new(components: [f32; 4]) -> Self {
        Self(components)
    }

    /// C map: `third_party/refloat/src/balance_filter.c:145-154`.
    #[inline(always)]
    fn pitch_projection(self) -> f32 {
        let [q0, q1, q2, q3] = self.0;
        -2.0 * (q1 * q3 - q0 * q2)
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn length_squared(self) -> f32 {
        let [q0, q1, q2, q3] = self.0;
        q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3
    }

    /// C map: `third_party/refloat/src/balance_filter.c:98-101`.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn estimated_half_gravity(self) -> RefloatHalfGravity {
        let [q0, q1, q2, q3] = self.0;
        RefloatHalfGravity([
            q1 * q3 - q0 * q2,
            q0 * q1 + q2 * q3,
            q0 * q0 - 0.5 + q3 * q3,
        ])
    }

    /// C map: `third_party/refloat/src/balance_filter.c:118-124`.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn delta_from_gyro(self, gyro_half_step: RefloatGyroHalfStep) -> RefloatQuaternionDelta {
        let [q0, q1, q2, q3] = self.0;
        let [gx, gy, gz] = gyro_half_step.0;
        let vector_dot_gyro = q1 * gx + q2 * gy + q3 * gz;
        let vector_cross_gyro = [q2 * gz - q3 * gy, q3 * gx - q1 * gz, q1 * gy - q2 * gx];
        RefloatQuaternionDelta([
            -vector_dot_gyro,
            q0 * gx + vector_cross_gyro[0],
            q0 * gy + vector_cross_gyro[1],
            q0 * gz + vector_cross_gyro[2],
        ])
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatQuaternionDelta {
    #[inline(always)]
    const fn wxyz(self) -> [f32; 4] {
        self.0
    }
}

/// Refloat-owned balance filter state.
///
/// C map: `BalanceFilterData` is initialized from firmware quaternions at
/// `third_party/refloat/src/balance_filter.c:53-61`, configured at `third_party/refloat/src/balance_filter.c:64-70`,
/// updated from `imu_ref_callback` at `third_party/refloat/src/main.c:760-765`, and read by
/// `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceFilter {
    q0: f32,
    q1: f32,
    q2: f32,
    q3: f32,
    acc_mag: f32,
    kp_pitch: f32,
    kp_roll: f32,
    kp_yaw: f32,
}

impl RefloatBalanceFilter {
    pub(crate) const fn source_startup() -> Self {
        Self {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
            acc_mag: 1.0,
            kp_pitch: 2.0,
            kp_roll: 1.4,
            kp_yaw: 1.7,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_quaternions([q0, q1, q2, q3]: [f32; 4]) -> Self {
        Self {
            q0,
            q1,
            q2,
            q3,
            ..Self::source_startup()
        }
    }

    pub(crate) fn configure(&mut self, mahony_kp: f32, mahony_kp_roll: f32) {
        // Refloat copies `mahony_kp`/`mahony_kp_roll` into the filter and
        // averages yaw KP at `third_party/refloat/src/balance_filter.c:64-70`.
        self.kp_pitch = mahony_kp;
        self.kp_roll = mahony_kp_roll;
        self.kp_yaw = Self::yaw_kp(mahony_kp, mahony_kp_roll);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn update(&mut self, sample: ImuReadSample) {
        // Refloat's callback feeds gyro first, accel second at
        // `third_party/refloat/src/main.c:760-765`; the Mahony update itself is
        // `third_party/refloat/src/balance_filter.c:73-134`.
        let gyro = self
            .gyro_with_accel_correction(sample.angular_rate().into(), sample.acceleration().into());
        self.integrate_gyro(gyro, sample.period().duration().as_seconds());
        self.normalize_quaternion();
    }

    pub(crate) fn balance_pitch(&self) -> RefloatRealtimeBalancePitch {
        RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(self.pitch_radians()))
    }

    fn pitch_radians(&self) -> f32 {
        // Refloat computes pitch as `asin(-2 * (q1*q3 - q0*q2))`, clamped to
        // +/- pi/2, at `third_party/refloat/src/balance_filter.c:145-154`.
        libm::asinf(self.pitch_sin().clamp(-1.0, 1.0))
    }

    fn pitch_sin(&self) -> f32 {
        // C map: `third_party/refloat/src/balance_filter.c:145-154` uses
        // quaternion (q0, q1, q2, q3) as (w, x, y, z) and feeds this
        // orientation projection to `asinf` for pitch.
        self.quaternion().pitch_projection()
    }

    const fn yaw_kp(mahony_kp: f32, mahony_kp_roll: f32) -> f32 {
        (mahony_kp + mahony_kp_roll) / 2.0
    }

    #[inline(always)]
    const fn quaternion(&self) -> RefloatQuaternion {
        RefloatQuaternion::new([self.q0, self.q1, self.q2, self.q3])
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn gyro_with_accel_correction(
        &mut self,
        gyro: RefloatGyroRate,
        accel: RefloatAccelSample,
    ) -> RefloatCorrectedGyroRate {
        let Some((accel_norm, accel)) = Self::normalized_accel(accel) else {
            return gyro.without_accel_feedback();
        };
        let confidence = self.accel_confidence(accel_norm);
        let error = self.accel_error(accel);

        // C map: `third_party/refloat/src/balance_filter.c:87-111` applies
        // Mahony proportional feedback from accelerometer confidence,
        // measured-vs-estimated gravity error, and per-axis KP.
        gyro.with_gravity_feedback(error, self.feedback_gains(confidence))
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalized_accel(
        accel: RefloatAccelSample,
    ) -> Option<(RefloatAccelMagnitude, RefloatMeasuredGravity)> {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` enters
        // feedback only when accel norm is above 0.01, then normalizes it.
        accel.normalized_for_feedback()
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, accel: RefloatMeasuredGravity) -> RefloatGravityError {
        // C map: `third_party/refloat/src/balance_filter.c:98-101` computes
        // the estimated gravity half-vector from the current quaternion.
        let estimated_gravity = self.quaternion().estimated_half_gravity();

        // C map: `third_party/refloat/src/balance_filter.c:103-106` crosses
        // measured gravity (accelerometer) against estimated gravity.
        accel.error_against(estimated_gravity)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, gyro: RefloatCorrectedGyroRate, dt: f32) {
        // C map: `third_party/refloat/src/balance_filter.c:114-117`
        // pre-multiplies gyro by half the tick duration.
        let gyro_half_step = gyro.half_step(dt);

        // C map: `third_party/refloat/src/balance_filter.c:118-124`
        // integrates q_dot = 0.5 * q * gyro in upstream component order.
        let [dq0, dq1, dq2, dq3] = self.quaternion().delta_from_gyro(gyro_half_step).wxyz();
        self.q0 += dq0;
        self.q1 += dq1;
        self.q2 += dq2;
        self.q3 += dq3;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalize_quaternion(&mut self) {
        // C map: `third_party/refloat/src/balance_filter.c:126-133` keeps the
        // integrated orientation on the unit-quaternion sphere.
        let recip_norm = refloat_inv_sqrt(self.quaternion().length_squared());
        self.q0 *= recip_norm;
        self.q1 *= recip_norm;
        self.q2 *= recip_norm;
        self.q3 *= recip_norm;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, new_acc_mag: RefloatAccelMagnitude) -> RefloatAccelConfidence {
        // Refloat filters accelerometer magnitude and clamps confidence at
        // zero in `third_party/refloat/src/balance_filter.c:42-50`.
        self.acc_mag = self.acc_mag * 0.9 + new_acc_mag.as_f32() * 0.1;
        RefloatAccelConfidence((1.0 - 0.02 * libm::sqrtf((self.acc_mag - 1.0).abs())).max(0.0))
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn feedback_gains(&self, confidence: RefloatAccelConfidence) -> RefloatFeedbackGains {
        let confidence = confidence.as_f32();
        RefloatFeedbackGains {
            roll: 2.0 * self.kp_roll * confidence,
            pitch: 2.0 * self.kp_pitch * confidence,
            yaw: 2.0 * self.kp_yaw * confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RefloatAccelSample, RefloatBalanceFilter, RefloatCorrectedGyroRate, RefloatGyroRate,
    };
    use vescpkg_rs::prelude::{
        AccelerationG, AngularVelocity, ImuAcceleration, ImuAccelerationX, ImuAccelerationY,
        ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll,
        ImuAngularRateYaw, ImuReadSample, ImuSamplePeriod, VescSeconds,
    };

    fn imu_acceleration(x_g: f32, y_g: f32, z_g: f32) -> ImuAcceleration {
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(x_g)),
            ImuAccelerationY::new(AccelerationG::from_g(y_g)),
            ImuAccelerationZ::new(AccelerationG::from_g(z_g)),
        )
    }

    fn imu_angular_rate(roll_dps: f32, pitch_dps: f32, yaw_dps: f32) -> ImuAngularRate {
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(roll_dps)),
            ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(pitch_dps)),
            ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(yaw_dps)),
        )
    }

    fn imu_period(seconds: f32) -> ImuSamplePeriod {
        ImuSamplePeriod::new(VescSeconds::from_seconds(seconds))
    }

    fn imu_sample(
        acceleration: ImuAcceleration,
        angular_rate: ImuAngularRate,
        period: ImuSamplePeriod,
    ) -> ImuReadSample {
        ImuReadSample::from_parts(acceleration, angular_rate, period)
    }

    #[test]
    fn balance_filter_update_integrates_positive_pitch_like_refloat_callback() {
        let mut filter = RefloatBalanceFilter::source_startup();

        filter.update(imu_sample(
            imu_acceleration(0.0, 0.0, 1.0),
            imu_angular_rate(0.0, 1.0, 0.0),
            imu_period(0.1),
        ));

        // Refloat's `imu_ref_callback` forwards gyro/accel/dt at
        // `third_party/refloat/src/main.c:760-765`; `balance_filter_update` integrates the
        // quaternion at `third_party/refloat/src/balance_filter.c:73-134`, and
        // `balance_filter_get_pitch` reads it at `third_party/refloat/src/balance_filter.c:145-154`.
        assert!(filter.pitch_radians() > 0.0);
    }

    #[test]
    fn balance_filter_pitch_clamps_quaternion_projection_like_refloat() {
        let positive = RefloatBalanceFilter::from_quaternions([1.0, 0.0, 1.0, 0.0]);
        let negative = RefloatBalanceFilter::from_quaternions([-1.0, 0.0, 1.0, 0.0]);

        // Refloat clamps the asin input before converting to pitch at
        // `third_party/refloat/src/balance_filter.c:145-154`.
        assert_eq!(positive.pitch_radians(), core::f32::consts::FRAC_PI_2);
        assert_eq!(negative.pitch_radians(), -core::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn balance_filter_configures_yaw_kp_from_pitch_and_roll_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        filter.configure(4.0, 2.0);

        // Refloat averages pitch and roll KP for yaw at
        // `third_party/refloat/src/balance_filter.c:64-70`.
        assert_eq!(filter.kp_pitch, 4.0);
        assert_eq!(filter.kp_roll, 2.0);
        assert_eq!(filter.kp_yaw, 3.0);
    }

    #[test]
    fn balance_filter_normalizes_accel_before_correction_like_refloat() {
        let (_, unit) =
            RefloatBalanceFilter::normalized_accel(RefloatAccelSample::new([0.0, 0.0, 2.0]))
                .expect("nonzero accel normalizes");

        assert_eq!(unit.xyz(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn balance_filter_skips_accel_correction_for_tiny_sample_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            RefloatGyroRate::new([1.0, 2.0, 3.0]),
            RefloatAccelSample::new([0.0, 0.0, 0.005]),
        );

        assert_eq!(gyro.xyz(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn balance_filter_applies_gravity_error_feedback_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            RefloatGyroRate::new([1.0, 2.0, 3.0]),
            RefloatAccelSample::new([0.0, 1.0, 0.0]),
        );

        let [roll_rate, pitch_rate, yaw_rate] = gyro.xyz();
        assert!((roll_rate - 2.4).abs() < 0.000001);
        assert_eq!(pitch_rate, 2.0);
        assert_eq!(yaw_rate, 3.0);
    }

    #[test]
    fn balance_filter_integrates_gyro_components_like_refloat() {
        let mut filter = RefloatBalanceFilter::from_quaternions([1.0, 2.0, 3.0, 4.0]);

        filter.integrate_gyro(RefloatCorrectedGyroRate::new([0.2, 0.4, 0.6]), 0.5);

        assert!((filter.q0 - 0.0).abs() < 0.000001);
        assert!((filter.q1 - 2.1).abs() < 0.000001);
        assert!((filter.q2 - 3.0).abs() < 0.000001);
        assert!((filter.q3 - 4.2).abs() < 0.000001);
    }
}
