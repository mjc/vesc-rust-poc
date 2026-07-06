use crate::domain::RefloatRealtimeBalancePitch;
use core::marker::PhantomData;
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct AxisScalar<Tag>(f32, PhantomData<fn() -> Tag>);

impl<Tag> AxisScalar<Tag> {
    #[inline(always)]
    const fn new(value: f32) -> Self {
        Self(value, PhantomData)
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatAccelMagnitude(f32);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MeasuredGravity([f32; 3]);

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
enum RollGravityErrorTag {}
#[cfg(any(test, target_arch = "arm"))]
enum PitchGravityErrorTag {}
#[cfg(any(test, target_arch = "arm"))]
enum YawGravityErrorTag {}

#[cfg(any(test, target_arch = "arm"))]
type RollGravityError = AxisScalar<RollGravityErrorTag>;
#[cfg(any(test, target_arch = "arm"))]
type PitchGravityError = AxisScalar<PitchGravityErrorTag>;
#[cfg(any(test, target_arch = "arm"))]
type YawGravityError = AxisScalar<YawGravityErrorTag>;

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatFeedbackGains {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

#[cfg(any(test, target_arch = "arm"))]
enum RollAngularRateTag {}
#[cfg(any(test, target_arch = "arm"))]
enum PitchAngularRateTag {}
#[cfg(any(test, target_arch = "arm"))]
enum YawAngularRateTag {}
#[cfg(any(test, target_arch = "arm"))]
enum RollAngularHalfStepTag {}
#[cfg(any(test, target_arch = "arm"))]
enum PitchAngularHalfStepTag {}
#[cfg(any(test, target_arch = "arm"))]
enum YawAngularHalfStepTag {}

#[cfg(any(test, target_arch = "arm"))]
type RollAngularRate = AxisScalar<RollAngularRateTag>;
#[cfg(any(test, target_arch = "arm"))]
type PitchAngularRate = AxisScalar<PitchAngularRateTag>;
#[cfg(any(test, target_arch = "arm"))]
type YawAngularRate = AxisScalar<YawAngularRateTag>;
#[cfg(any(test, target_arch = "arm"))]
type RollAngularHalfStep = AxisScalar<RollAngularHalfStepTag>;
#[cfg(any(test, target_arch = "arm"))]
type PitchAngularHalfStep = AxisScalar<PitchAngularHalfStepTag>;
#[cfg(any(test, target_arch = "arm"))]
type YawAngularHalfStep = AxisScalar<YawAngularHalfStepTag>;

enum QuaternionScalarTag {}
enum QuaternionXTag {}
enum QuaternionYTag {}
enum QuaternionZTag {}

type QuaternionScalar = AxisScalar<QuaternionScalarTag>;
type QuaternionX = AxisScalar<QuaternionXTag>;
type QuaternionY = AxisScalar<QuaternionYTag>;
type QuaternionZ = AxisScalar<QuaternionZTag>;

#[cfg(any(test, target_arch = "arm"))]
enum QuaternionDeltaScalarTag {}
#[cfg(any(test, target_arch = "arm"))]
enum QuaternionDeltaXTag {}
#[cfg(any(test, target_arch = "arm"))]
enum QuaternionDeltaYTag {}
#[cfg(any(test, target_arch = "arm"))]
enum QuaternionDeltaZTag {}

#[cfg(any(test, target_arch = "arm"))]
type QuaternionDeltaScalar = AxisScalar<QuaternionDeltaScalarTag>;
#[cfg(any(test, target_arch = "arm"))]
type QuaternionDeltaX = AxisScalar<QuaternionDeltaXTag>;
#[cfg(any(test, target_arch = "arm"))]
type QuaternionDeltaY = AxisScalar<QuaternionDeltaYTag>;
#[cfg(any(test, target_arch = "arm"))]
type QuaternionDeltaZ = AxisScalar<QuaternionDeltaZTag>;

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct MeasuredAngularRate([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct CorrectedAngularRate([f32; 3]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct AngularRateHalfStep([f32; 3]);

#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatQuaternion([f32; 4]);

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatQuaternionDelta([f32; 4]);

#[cfg(any(test, target_arch = "arm"))]
impl RefloatAccelMagnitude {
    #[inline(always)]
    const fn as_f32(self) -> f32 {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl MeasuredGravity {
    /// C map: `third_party/refloat/src/balance_filter.c:82-96`.
    #[inline(always)]
    fn from_acceleration(acceleration: ImuAcceleration) -> Option<(RefloatAccelMagnitude, Self)> {
        acceleration.map_axes(Self::from_axes)
    }

    fn from_axes(
        x: ImuAccelerationX,
        y: ImuAccelerationY,
        z: ImuAccelerationZ,
    ) -> Option<(RefloatAccelMagnitude, Self)> {
        Self::from_xyz([
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        ])
    }

    fn from_xyz(xyz: [f32; 3]) -> Option<(RefloatAccelMagnitude, Self)> {
        let length_squared = Self::length_squared(xyz);
        let accel_norm = libm::sqrtf(length_squared);
        match accel_norm {
            norm if norm > 0.01 => Some((
                RefloatAccelMagnitude(norm),
                Self::scaled(xyz, refloat_inv_sqrt(length_squared)),
            )),
            _ => None,
        }
    }

    #[inline(always)]
    fn scaled([x, y, z]: [f32; 3], scale: f32) -> Self {
        Self([x * scale, y * scale, z * scale])
    }

    #[inline(always)]
    fn length_squared([x, y, z]: [f32; 3]) -> f32 {
        x * x + y * y + z * z
    }

    /// C map: `third_party/refloat/src/balance_filter.c:103-106`.
    #[inline(always)]
    fn error_against(self, estimated_gravity: RefloatHalfGravity) -> RefloatGravityError {
        let [x, y, z] = self.0;
        let [rhs_x, rhs_y, rhs_z] = estimated_gravity.0;
        RefloatGravityError::new(
            RollGravityError::new(y * rhs_z - z * rhs_y),
            PitchGravityError::new(z * rhs_x - x * rhs_z),
            YawGravityError::new(x * rhs_y - y * rhs_x),
        )
    }

    #[cfg(test)]
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
impl RefloatGravityError {
    #[inline(always)]
    const fn new(roll: RollGravityError, pitch: PitchGravityError, yaw: YawGravityError) -> Self {
        Self([roll.0, pitch.0, yaw.0])
    }

    #[inline(always)]
    const fn xyz(self) -> [f32; 3] {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl MeasuredAngularRate {
    #[inline(always)]
    const fn new(roll: RollAngularRate, pitch: PitchAngularRate, yaw: YawAngularRate) -> Self {
        Self([roll.0, pitch.0, yaw.0])
    }

    fn from_axes(
        roll: ImuAngularRateRoll,
        pitch: ImuAngularRatePitch,
        yaw: ImuAngularRateYaw,
    ) -> Self {
        Self::new(
            RollAngularRate::new(roll.angular_velocity().as_degrees_per_second()),
            PitchAngularRate::new(pitch.angular_velocity().as_degrees_per_second()),
            YawAngularRate::new(yaw.angular_velocity().as_degrees_per_second()),
        )
    }

    #[inline(always)]
    const fn without_accel_feedback(self) -> CorrectedAngularRate {
        CorrectedAngularRate(self.0)
    }

    /// C map: `third_party/refloat/src/balance_filter.c:107-111`.
    #[inline(always)]
    fn with_gravity_feedback(
        self,
        error: RefloatGravityError,
        gains: RefloatFeedbackGains,
    ) -> CorrectedAngularRate {
        let [roll_rate, pitch_rate, yaw_rate] = self.0;
        let [roll_error, pitch_error, yaw_error] = error.xyz();
        CorrectedAngularRate([
            roll_rate + gains.roll * roll_error,
            pitch_rate + gains.pitch * pitch_error,
            yaw_rate + gains.yaw * yaw_error,
        ])
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl From<ImuAngularRate> for MeasuredAngularRate {
    #[inline(always)]
    fn from(angular_rate: ImuAngularRate) -> Self {
        angular_rate.map_axes(Self::from_axes)
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl CorrectedAngularRate {
    #[cfg(test)]
    const fn new(roll: RollAngularRate, pitch: PitchAngularRate, yaw: YawAngularRate) -> Self {
        Self([roll.0, pitch.0, yaw.0])
    }

    /// C map: `third_party/refloat/src/balance_filter.c:114-117`.
    #[inline(always)]
    fn half_step(self, dt: f32) -> AngularRateHalfStep {
        let [roll_rate, pitch_rate, yaw_rate] = self.0;
        AngularRateHalfStep::new(
            RollAngularHalfStep::new(roll_rate * 0.5 * dt),
            PitchAngularHalfStep::new(pitch_rate * 0.5 * dt),
            YawAngularHalfStep::new(yaw_rate * 0.5 * dt),
        )
    }

    #[cfg(test)]
    const fn xyz(self) -> [f32; 3] {
        self.0
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl AngularRateHalfStep {
    #[inline(always)]
    const fn new(
        roll: RollAngularHalfStep,
        pitch: PitchAngularHalfStep,
        yaw: YawAngularHalfStep,
    ) -> Self {
        Self([roll.0, pitch.0, yaw.0])
    }
}

impl RefloatQuaternion {
    #[inline(always)]
    const fn new(scalar: QuaternionScalar, x: QuaternionX, y: QuaternionY, z: QuaternionZ) -> Self {
        Self([scalar.0, x.0, y.0, z.0])
    }

    /// C map: `third_party/refloat/src/balance_filter.c:145-154`.
    #[inline(always)]
    fn pitch_projection(self) -> f32 {
        let [scalar, x, y, z] = self.0;
        -2.0 * (x * z - scalar * y)
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn length_squared(self) -> f32 {
        let [scalar, x, y, z] = self.0;
        scalar * scalar + x * x + y * y + z * z
    }

    /// C map: `third_party/refloat/src/balance_filter.c:98-101`.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn estimated_half_gravity(self) -> RefloatHalfGravity {
        let [scalar, x, y, z] = self.0;
        RefloatHalfGravity([
            x * z - scalar * y,
            scalar * x + y * z,
            scalar * scalar - 0.5 + z * z,
        ])
    }

    /// C map: `third_party/refloat/src/balance_filter.c:118-124`.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    fn delta_from_gyro(self, gyro_half_step: AngularRateHalfStep) -> RefloatQuaternionDelta {
        let [scalar, x, y, z] = self.0;
        let [gx, gy, gz] = gyro_half_step.0;
        let vector_dot_gyro = x * gx + y * gy + z * gz;
        let vector_cross_gyro = [y * gz - z * gy, z * gx - x * gz, x * gy - y * gx];
        RefloatQuaternionDelta::new(
            QuaternionDeltaScalar::new(-vector_dot_gyro),
            QuaternionDeltaX::new(scalar * gx + vector_cross_gyro[0]),
            QuaternionDeltaY::new(scalar * gy + vector_cross_gyro[1]),
            QuaternionDeltaZ::new(scalar * gz + vector_cross_gyro[2]),
        )
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatQuaternionDelta {
    #[inline(always)]
    const fn new(
        scalar: QuaternionDeltaScalar,
        x: QuaternionDeltaX,
        y: QuaternionDeltaY,
        z: QuaternionDeltaZ,
    ) -> Self {
        Self([scalar.0, x.0, y.0, z.0])
    }

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
        let gyro =
            self.gyro_with_accel_correction(sample.angular_rate().into(), sample.acceleration());
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
        RefloatQuaternion::new(
            QuaternionScalar::new(self.q0),
            QuaternionX::new(self.q1),
            QuaternionY::new(self.q2),
            QuaternionZ::new(self.q3),
        )
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

        // C map: `third_party/refloat/src/balance_filter.c:87-111` applies
        // Mahony proportional feedback from accelerometer confidence,
        // measured-vs-estimated gravity error, and per-axis KP.
        gyro.with_gravity_feedback(error, self.feedback_gains(confidence))
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn measured_gravity(
        acceleration: ImuAcceleration,
    ) -> Option<(RefloatAccelMagnitude, MeasuredGravity)> {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` enters
        // feedback only when accel norm is above 0.01, then normalizes it.
        MeasuredGravity::from_acceleration(acceleration)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, accel: MeasuredGravity) -> RefloatGravityError {
        // C map: `third_party/refloat/src/balance_filter.c:98-101` computes
        // the estimated gravity half-vector from the current quaternion.
        let estimated_gravity = self.quaternion().estimated_half_gravity();

        // C map: `third_party/refloat/src/balance_filter.c:103-106` crosses
        // measured gravity (accelerometer) against estimated gravity.
        accel.error_against(estimated_gravity)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, gyro: CorrectedAngularRate, dt: f32) {
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
        CorrectedAngularRate, MeasuredAngularRate, PitchAngularRate, RefloatBalanceFilter,
        RollAngularRate, YawAngularRate,
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
        let (_, unit) = RefloatBalanceFilter::measured_gravity(imu_acceleration(0.0, 0.0, 2.0))
            .expect("nonzero accel normalizes");

        assert_eq!(unit.xyz(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn balance_filter_skips_accel_correction_for_tiny_sample_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            MeasuredAngularRate::new(
                RollAngularRate::new(1.0),
                PitchAngularRate::new(2.0),
                YawAngularRate::new(3.0),
            ),
            imu_acceleration(0.0, 0.0, 0.005),
        );

        assert_eq!(gyro.xyz(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn balance_filter_applies_gravity_error_feedback_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction(
            MeasuredAngularRate::new(
                RollAngularRate::new(1.0),
                PitchAngularRate::new(2.0),
                YawAngularRate::new(3.0),
            ),
            imu_acceleration(0.0, 1.0, 0.0),
        );

        let [roll_rate, pitch_rate, yaw_rate] = gyro.xyz();
        assert!((roll_rate - 2.4).abs() < 0.000001);
        assert_eq!(pitch_rate, 2.0);
        assert_eq!(yaw_rate, 3.0);
    }

    #[test]
    fn balance_filter_integrates_gyro_components_like_refloat() {
        let mut filter = RefloatBalanceFilter::from_quaternions([1.0, 2.0, 3.0, 4.0]);

        filter.integrate_gyro(
            CorrectedAngularRate::new(
                RollAngularRate::new(0.2),
                PitchAngularRate::new(0.4),
                YawAngularRate::new(0.6),
            ),
            0.5,
        );

        assert!((filter.q0 - 0.0).abs() < 0.000001);
        assert!((filter.q1 - 2.1).abs() < 0.000001);
        assert!((filter.q2 - 3.0).abs() < 0.000001);
        assert!((filter.q3 - 4.2).abs() < 0.000001);
    }
}
