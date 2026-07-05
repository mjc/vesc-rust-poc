use crate::domain::RefloatRealtimeBalancePitch;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::ImuBindings;
use vescpkg_rs::prelude::AngleRadians;

/// Refloat-owned balance filter state.
///
/// C map: `BalanceFilterData` is initialized from firmware quaternions at
/// `third_party/refloat/src/balance_filter.c:53-61`, configured at `third_party/refloat/src/balance_filter.c:64-70`,
/// updated from `imu_ref_callback` at `third_party/refloat/src/main.c:760-765`, and read by
/// `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RefloatBalanceFilter {
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
    pub(super) const fn source_startup() -> Self {
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

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn from_quaternions([q0, q1, q2, q3]: [f32; 4]) -> Self {
        Self {
            q0,
            q1,
            q2,
            q3,
            ..Self::source_startup()
        }
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    pub(super) fn from_firmware_quaternions() -> Self {
        Self::from_quaternions(vescpkg_rs::RealImuBindings.quaternions())
    }

    pub(super) fn configure(&mut self, mahony_kp: f32, mahony_kp_roll: f32) {
        // Refloat copies `mahony_kp`/`mahony_kp_roll` into the filter and
        // averages yaw KP at `third_party/refloat/src/balance_filter.c:64-70`.
        self.kp_pitch = mahony_kp;
        self.kp_roll = mahony_kp_roll;
        self.kp_yaw = (mahony_kp + mahony_kp_roll) / 2.0;
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn update(&mut self, gyro: [f32; 3], accel: [f32; 3], dt: f32) {
        // Refloat's callback feeds gyro first, accel second at
        // `third_party/refloat/src/main.c:760-765`; the Mahony update itself is
        // `third_party/refloat/src/balance_filter.c:73-134`.
        let [mut gx, mut gy, mut gz] = gyro;
        let [mut ax, mut ay, mut az] = accel;
        let accel_norm = libm::sqrtf(ax * ax + ay * ay + az * az);

        if accel_norm > 0.01 {
            let accel_confidence = self.accel_confidence(accel_norm);
            let two_kp_pitch = 2.0 * self.kp_pitch * accel_confidence;
            let two_kp_roll = 2.0 * self.kp_roll * accel_confidence;
            let two_kp_yaw = 2.0 * self.kp_yaw * accel_confidence;
            let recip_norm = Self::inv_sqrt(ax * ax + ay * ay + az * az);
            ax *= recip_norm;
            ay *= recip_norm;
            az *= recip_norm;

            let halfvx = self.q1 * self.q3 - self.q0 * self.q2;
            let halfvy = self.q0 * self.q1 + self.q2 * self.q3;
            let halfvz = self.q0 * self.q0 - 0.5 + self.q3 * self.q3;
            let halfex = ay * halfvz - az * halfvy;
            let halfey = az * halfvx - ax * halfvz;
            let halfez = ax * halfvy - ay * halfvx;

            gx += two_kp_roll * halfex;
            gy += two_kp_pitch * halfey;
            gz += two_kp_yaw * halfez;
        }

        gx *= 0.5 * dt;
        gy *= 0.5 * dt;
        gz *= 0.5 * dt;
        let qa = self.q0;
        let qb = self.q1;
        let qc = self.q2;
        self.q0 += -qb * gx - qc * gy - self.q3 * gz;
        self.q1 += qa * gx + qc * gz - self.q3 * gy;
        self.q2 += qa * gy - qb * gz + self.q3 * gx;
        self.q3 += qa * gz + qb * gy - qc * gx;

        let recip_norm = Self::inv_sqrt(
            self.q0 * self.q0 + self.q1 * self.q1 + self.q2 * self.q2 + self.q3 * self.q3,
        );
        self.q0 *= recip_norm;
        self.q1 *= recip_norm;
        self.q2 *= recip_norm;
        self.q3 *= recip_norm;
    }

    pub(super) fn balance_pitch(&self) -> RefloatRealtimeBalancePitch {
        RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(self.pitch_radians()))
    }

    pub(super) fn pitch_radians(&self) -> f32 {
        // Refloat computes pitch as `asin(-2 * (q1*q3 - q0*q2))`, clamped to
        // +/- pi/2, at `third_party/refloat/src/balance_filter.c:145-154`.
        let sin = -2.0 * (self.q1 * self.q3 - self.q0 * self.q2);
        if sin < -1.0 {
            -core::f32::consts::FRAC_PI_2
        } else if sin > 1.0 {
            core::f32::consts::FRAC_PI_2
        } else {
            libm::asinf(sin)
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, new_acc_mag: f32) -> f32 {
        // Refloat filters accelerometer magnitude and clamps confidence at
        // zero in `third_party/refloat/src/balance_filter.c:42-50`.
        self.acc_mag = self.acc_mag * 0.9 + new_acc_mag * 0.1;
        let confidence = 1.0 - 0.02 * libm::sqrtf((self.acc_mag - 1.0).abs());
        if confidence > 0.0 { confidence } else { 0.0 }
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
}
