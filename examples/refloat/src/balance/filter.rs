use crate::domain::RefloatRealtimeBalancePitch;
use vescpkg_rs::prelude::AngleRadians;

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
        // Quaternion (q0, q1, q2, q3) is (w, x, y, z). This is the
        // orientation projection that upstream feeds to `asin` for pitch.
        -2.0 * (self.q1 * self.q3 - self.q0 * self.q2)
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

        // Mahony proportional feedback: measured-vs-estimated gravity error,
        // scaled by accelerometer confidence and per-axis KP, corrects gyro.
        [
            gx + 2.0 * self.kp_roll * confidence * halfex,
            gy + 2.0 * self.kp_pitch * confidence * halfey,
            gz + 2.0 * self.kp_yaw * confidence * halfez,
        ]
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalized_accel([ax, ay, az]: [f32; 3]) -> Option<(f32, [f32; 3])> {
        let accel_norm = libm::sqrtf(ax * ax + ay * ay + az * az);
        match accel_norm {
            // Below this threshold upstream treats the accelerometer sample as
            // unusable and leaves the gyro uncorrected for this tick.
            norm if norm > 0.01 => {
                let recip_norm = Self::inv_sqrt(ax * ax + ay * ay + az * az);
                Some((norm, [ax * recip_norm, ay * recip_norm, az * recip_norm]))
            }
            _ => None,
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, [ax, ay, az]: [f32; 3]) -> [f32; 3] {
        // Estimated gravity half-vector from the current quaternion.
        let halfvx = self.q1 * self.q3 - self.q0 * self.q2;
        let halfvy = self.q0 * self.q1 + self.q2 * self.q3;
        let halfvz = self.q0 * self.q0 - 0.5 + self.q3 * self.q3;

        // Cross measured gravity (accelerometer) against estimated gravity.
        [
            ay * halfvz - az * halfvy,
            az * halfvx - ax * halfvz,
            ax * halfvy - ay * halfvx,
        ]
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, [gx, gy, gz]: [f32; 3], dt: f32) {
        // Quaternion derivative uses half angular displacement over this tick.
        let [gx, gy, gz] = [gx * 0.5 * dt, gy * 0.5 * dt, gz * 0.5 * dt];
        let [q0, q1, q2, q3] = [self.q0, self.q1, self.q2, self.q3];

        // Integrate q_dot = 0.5 * q * gyro, preserving upstream component order.
        self.q0 += -q1 * gx - q2 * gy - q3 * gz;
        self.q1 += q0 * gx + q2 * gz - q3 * gy;
        self.q2 += q0 * gy - q1 * gz + q3 * gx;
        self.q3 += q0 * gz + q1 * gy - q2 * gx;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalize_quaternion(&mut self) {
        // Keep the integrated orientation on the unit-quaternion sphere.
        let recip_norm = Self::inv_sqrt(
            self.q0 * self.q0 + self.q1 * self.q1 + self.q2 * self.q2 + self.q3 * self.q3,
        );
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
