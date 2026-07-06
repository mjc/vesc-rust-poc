use crate::domain::RefloatRealtimeBalancePitch;
use vescpkg_rs::prelude::AngleRadians;

/// C map: `third_party/refloat/src/balance_filter.c:145-154`.
#[inline(always)]
fn refloat_pitch_projection(q0: f32, q1: f32, q2: f32, q3: f32) -> f32 {
    -2.0 * (q1 * q3 - q0 * q2)
}

/// C map: `third_party/refloat/src/balance_filter.c:82-93`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_vector_length_squared(x: f32, y: f32, z: f32) -> f32 {
    x * x + y * y + z * z
}

/// C map: `third_party/refloat/src/balance_filter.c:82-96`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_accel_norm(ax: f32, ay: f32, az: f32) -> f32 {
    libm::sqrtf(refloat_vector_length_squared(ax, ay, az))
}

/// C map: `third_party/refloat/src/balance_filter.c:98-101`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_estimated_half_gravity(q0: f32, q1: f32, q2: f32, q3: f32) -> [f32; 3] {
    [
        q1 * q3 - q0 * q2,
        q0 * q1 + q2 * q3,
        q0 * q0 - 0.5 + q3 * q3,
    ]
}

/// C map: `third_party/refloat/src/balance_filter.c:103-106`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_accel_gravity_error(
    ax: f32,
    ay: f32,
    az: f32,
    [halfvx, halfvy, halfvz]: [f32; 3],
) -> [f32; 3] {
    [
        ay * halfvz - az * halfvy,
        az * halfvx - ax * halfvz,
        ax * halfvy - ay * halfvx,
    ]
}

/// C map: `third_party/refloat/src/balance_filter.c:114-117`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_gyro_half_step(gx: f32, gy: f32, gz: f32, dt: f32) -> [f32; 3] {
    [gx * 0.5 * dt, gy * 0.5 * dt, gz * 0.5 * dt]
}

/// C map: `third_party/refloat/src/balance_filter.c:118-124`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_quaternion_delta(
    q0: f32,
    q1: f32,
    q2: f32,
    q3: f32,
    gx: f32,
    gy: f32,
    gz: f32,
) -> [f32; 4] {
    [
        -q1 * gx - q2 * gy - q3 * gz,
        q0 * gx + q2 * gz - q3 * gy,
        q0 * gy - q1 * gz + q3 * gx,
        q0 * gz + q1 * gy - q2 * gx,
    ]
}

/// C map: `third_party/refloat/src/balance_filter.c:126-133`.
#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
fn refloat_quaternion_length_squared(q0: f32, q1: f32, q2: f32, q3: f32) -> f32 {
    q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3
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
    pub(crate) fn update(&mut self, gyro: [f32; 3], accel: [f32; 3], dt: f32) {
        // Refloat's callback feeds gyro first, accel second at
        // `third_party/refloat/src/main.c:760-765`; the Mahony update itself is
        // `third_party/refloat/src/balance_filter.c:73-134`.
        let gyro = self.gyro_with_accel_correction(gyro, accel);
        self.integrate_gyro(gyro, dt);
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
        refloat_pitch_projection(self.q0, self.q1, self.q2, self.q3)
    }

    const fn yaw_kp(mahony_kp: f32, mahony_kp_roll: f32) -> f32 {
        (mahony_kp + mahony_kp_roll) / 2.0
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn gyro_with_accel_correction(&mut self, [gx, gy, gz]: [f32; 3], accel: [f32; 3]) -> [f32; 3] {
        let Some((accel_norm, accel)) = Self::normalized_accel(accel) else {
            return [gx, gy, gz];
        };
        let confidence = self.accel_confidence(accel_norm);
        let [halfex, halfey, halfez] = self.accel_error(accel);

        // C map: `third_party/refloat/src/balance_filter.c:87-111` applies
        // Mahony proportional feedback from accelerometer confidence,
        // measured-vs-estimated gravity error, and per-axis KP.
        [
            gx + 2.0 * self.kp_roll * confidence * halfex,
            gy + 2.0 * self.kp_pitch * confidence * halfey,
            gz + 2.0 * self.kp_yaw * confidence * halfez,
        ]
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalized_accel([ax, ay, az]: [f32; 3]) -> Option<(f32, [f32; 3])> {
        let accel_norm = refloat_accel_norm(ax, ay, az);
        match accel_norm {
            // C map: `third_party/refloat/src/balance_filter.c:82-96` enters
            // feedback only when accel norm is above 0.01, then normalizes it.
            norm if norm > 0.01 => {
                let recip_norm = Self::inv_sqrt(refloat_vector_length_squared(ax, ay, az));
                Some((norm, [ax * recip_norm, ay * recip_norm, az * recip_norm]))
            }
            _ => None,
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, [ax, ay, az]: [f32; 3]) -> [f32; 3] {
        // C map: `third_party/refloat/src/balance_filter.c:98-101` computes
        // the estimated gravity half-vector from the current quaternion.
        let estimated_gravity = refloat_estimated_half_gravity(self.q0, self.q1, self.q2, self.q3);

        // C map: `third_party/refloat/src/balance_filter.c:103-106` crosses
        // measured gravity (accelerometer) against estimated gravity.
        refloat_accel_gravity_error(ax, ay, az, estimated_gravity)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, [gx, gy, gz]: [f32; 3], dt: f32) {
        // C map: `third_party/refloat/src/balance_filter.c:114-117`
        // pre-multiplies gyro by half the tick duration.
        let [gx, gy, gz] = refloat_gyro_half_step(gx, gy, gz, dt);
        let [q0, q1, q2, q3] = [self.q0, self.q1, self.q2, self.q3];

        // C map: `third_party/refloat/src/balance_filter.c:118-124`
        // integrates q_dot = 0.5 * q * gyro in upstream component order.
        let [dq0, dq1, dq2, dq3] = refloat_quaternion_delta(q0, q1, q2, q3, gx, gy, gz);
        self.q0 += dq0;
        self.q1 += dq1;
        self.q2 += dq2;
        self.q3 += dq3;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalize_quaternion(&mut self) {
        // C map: `third_party/refloat/src/balance_filter.c:126-133` keeps the
        // integrated orientation on the unit-quaternion sphere.
        let recip_norm = Self::inv_sqrt(refloat_quaternion_length_squared(
            self.q0, self.q1, self.q2, self.q3,
        ));
        self.q0 *= recip_norm;
        self.q1 *= recip_norm;
        self.q2 *= recip_norm;
        self.q3 *= recip_norm;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, new_acc_mag: f32) -> f32 {
        // Refloat filters accelerometer magnitude and clamps confidence at
        // zero in `third_party/refloat/src/balance_filter.c:42-50`.
        self.acc_mag = self.acc_mag * 0.9 + new_acc_mag * 0.1;
        (1.0 - 0.02 * libm::sqrtf((self.acc_mag - 1.0).abs())).max(0.0)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn inv_sqrt(value: f32) -> f32 {
        // Refloat uses `1.0 / sqrtf(x)` at `third_party/refloat/src/balance_filter.c:38-40`.
        1.0 / libm::sqrtf(value)
    }
}

#[cfg(test)]
mod tests {
    use super::RefloatBalanceFilter;

    #[test]
    fn balance_filter_update_integrates_positive_pitch_like_refloat_callback() {
        let mut filter = RefloatBalanceFilter::source_startup();

        filter.update([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 0.1);

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
        let (_, unit) = RefloatBalanceFilter::normalized_accel([0.0, 0.0, 2.0])
            .expect("nonzero accel normalizes");

        assert_eq!(unit, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn balance_filter_skips_accel_correction_for_tiny_sample_like_refloat() {
        let mut filter = RefloatBalanceFilter::source_startup();

        let gyro = filter.gyro_with_accel_correction([1.0, 2.0, 3.0], [0.0, 0.0, 0.005]);

        assert_eq!(gyro, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn balance_filter_integrates_gyro_components_like_refloat() {
        let mut filter = RefloatBalanceFilter::from_quaternions([1.0, 2.0, 3.0, 4.0]);

        filter.integrate_gyro([0.2, 0.4, 0.6], 0.5);

        assert!((filter.q0 - 0.0).abs() < 0.000001);
        assert!((filter.q1 - 2.1).abs() < 0.000001);
        assert!((filter.q2 - 3.0).abs() < 0.000001);
        assert!((filter.q3 - 4.2).abs() < 0.000001);
    }
}
