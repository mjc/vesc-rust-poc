use crate::config::FloatOutBoyFilterConfig;
use crate::domain::FloatOutBoyRealtimeBalancePitch;
use vescpkg_rs::prelude::{AccelerationG, AngleRadians, MahonyPitchGain, MahonyRollGain};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{
    AngularVelocity, ImuAcceleration, ImuAngularRate, ImuOrientation, ImuReadSample,
};

/// Float Out Boy-owned balance filter state.
///
/// C map: `BalanceFilterData` is initialized from firmware quaternions at
/// `third_party/float-out-boy/src/balance_filter.c:53-61`, configured at `third_party/float-out-boy/src/balance_filter.c:64-70`,
/// updated from `imu_ref_callback` at `third_party/float-out-boy/src/main.c:760-765`, and read by
/// `imu_update` at `third_party/float-out-boy/src/imu.c:35-41`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BalanceFilter {
    orientation: [f32; 4],
    accel_magnitude: AccelerationG,
    feedback: [f32; 3],
}

impl Default for BalanceFilter {
    fn default() -> Self {
        Self {
            orientation: [1.0, 0.0, 0.0, 0.0],
            accel_magnitude: AccelerationG::from_g(1.0),
            feedback: [2.0, 1.4, 1.7],
        }
    }
}

impl BalanceFilter {
    #[cfg(test)]
    pub(crate) fn source_startup() -> Self {
        Self::default()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_orientation(orientation: ImuOrientation) -> Self {
        let quaternion = orientation.quaternion();
        Self {
            orientation: [
                f32::from(quaternion.w()),
                f32::from(quaternion.x()),
                f32::from(quaternion.y()),
                f32::from(quaternion.z()),
            ],
            ..Self::default()
        }
    }

    pub(crate) fn configure(&mut self, mahony_kp: MahonyPitchGain, mahony_kp_roll: MahonyRollGain) {
        // Float Out Boy copies `mahony_kp`/`mahony_kp_roll` into the filter and
        // averages yaw KP at `third_party/float-out-boy/src/balance_filter.c:64-70`.
        self.feedback = [
            mahony_kp.value(),
            mahony_kp_roll.value(),
            f32::midpoint(mahony_kp.value(), mahony_kp_roll.value()),
        ];
    }

    pub(crate) fn configure_from(&mut self, config: FloatOutBoyFilterConfig<'_>) {
        self.configure(config.mahony_kp(), config.mahony_kp_roll());
    }

    #[cfg(test)]
    pub(crate) const fn configured_gains(&self) -> (MahonyPitchGain, MahonyRollGain) {
        (
            MahonyPitchGain::new(self.feedback[0]),
            MahonyRollGain::new(self.feedback[1]),
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn update(&mut self, sample: ImuReadSample) {
        // Float Out Boy's callback feeds gyro first, accel second at
        // `third_party/float-out-boy/src/main.c:760-765`; the Mahony update itself is
        // `third_party/float-out-boy/src/balance_filter.c:73-134`.
        let gyro = self.gyro_with_accel_correction(sample.angular_rate(), sample.acceleration());
        self.integrate_gyro(gyro, sample.period().duration());
        self.normalize_quaternion();
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:145-154`.
    pub(crate) fn balance_pitch(&self) -> FloatOutBoyRealtimeBalancePitch {
        let [scalar, body_x, body_y, body_z] = self.orientation;
        let projection = -2.0 * (body_x * body_z - scalar * body_y);
        FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(vescpkg_rs::asin(
            projection.clamp(-1.0, 1.0),
        )))
    }

    #[cfg(test)]
    const fn orientation_for_test(&self) -> [f32; 4] {
        self.orientation
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn gyro_with_accel_correction(
        &mut self,
        gyro: ImuAngularRate,
        acceleration: ImuAcceleration,
    ) -> [AngularVelocity; 3] {
        let gyro = gyro.map_axes(|roll, pitch, yaw| {
            [
                roll.angular_velocity(),
                pitch.angular_velocity(),
                yaw.angular_velocity(),
            ]
        });
        Self::measured_gravity(acceleration).map_or_else(
            || gyro,
            |(accel_norm, measured_gravity)| {
                let confidence = self.accel_confidence(accel_norm);
                let error = self.accel_error(measured_gravity);
                let gains = self.feedback_gains(confidence);
                [
                    gyro[0] + AngularVelocity::from_radians_per_second(gains[0] * error[0]),
                    gyro[1] + AngularVelocity::from_radians_per_second(gains[1] * error[1]),
                    gyro[2] + AngularVelocity::from_radians_per_second(gains[2] * error[2]),
                ]
            },
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn measured_gravity(acceleration: ImuAcceleration) -> Option<(AccelerationG, [f32; 3])> {
        acceleration.map_axes(|x, y, z| {
            let measured = [
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            ];
            let norm = vescpkg_rs::sqrt(
                measured[0] * measured[0] + measured[1] * measured[1] + measured[2] * measured[2],
            );
            (norm > 0.01).then(|| {
                let reciprocal = 1.0 / norm;
                (
                    AccelerationG::from_g(norm),
                    [
                        measured[0] * reciprocal,
                        measured[1] * reciprocal,
                        measured[2] * reciprocal,
                    ],
                )
            })
        })
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_error(&self, measured: [f32; 3]) -> [f32; 3] {
        let [scalar, body_x, body_y, body_z] = self.orientation;
        let estimated = [
            body_x * body_z - scalar * body_y,
            scalar * body_x + body_y * body_z,
            scalar * scalar - 0.5 + body_z * body_z,
        ];
        [
            measured[1] * estimated[2] - measured[2] * estimated[1],
            measured[2] * estimated[0] - measured[0] * estimated[2],
            measured[0] * estimated[1] - measured[1] * estimated[0],
        ]
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn integrate_gyro(&mut self, gyro: [AngularVelocity; 3], dt: vescpkg_rs::prelude::VescSeconds) {
        let rotation = [
            (gyro[0] * dt * 0.5).as_radians(),
            (gyro[1] * dt * 0.5).as_radians(),
            (gyro[2] * dt * 0.5).as_radians(),
        ];
        let [scalar, body_x, body_y, body_z] = self.orientation;
        let [roll, pitch, yaw] = rotation;
        let dot = body_x * roll + body_y * pitch + body_z * yaw;
        let cross = [
            body_y * yaw - body_z * pitch,
            body_z * roll - body_x * yaw,
            body_x * pitch - body_y * roll,
        ];
        self.orientation[0] += -dot;
        self.orientation[1] += scalar * roll + cross[0];
        self.orientation[2] += scalar * pitch + cross[1];
        self.orientation[3] += scalar * yaw + cross[2];
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn normalize_quaternion(&mut self) {
        // C map: `third_party/float-out-boy/src/balance_filter.c:126-133` keeps the
        // integrated orientation on the unit-quaternion sphere.
        let [scalar, body_x, body_y, body_z] = self.orientation;
        let reciprocal = 1.0
            / vescpkg_rs::sqrt(
                scalar * scalar + body_x * body_x + body_y * body_y + body_z * body_z,
            );
        self.orientation = [
            scalar * reciprocal,
            body_x * reciprocal,
            body_y * reciprocal,
            body_z * reciprocal,
        ];
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, magnitude: AccelerationG) -> f32 {
        self.accel_magnitude = self.accel_magnitude * 0.9 + magnitude * 0.1;
        (1.0 - 0.02 * vescpkg_rs::sqrt((self.accel_magnitude.as_g() - 1.0).abs())).max(0.0)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn feedback_gains(&self, confidence: f32) -> [f32; 3] {
        let [pitch, roll, yaw] = self.feedback;
        [
            2.0 * roll * confidence,
            2.0 * pitch * confidence,
            2.0 * yaw * confidence,
        ]
    }
}

#[cfg(test)]
mod tests;
