//! Package-owned, no-allocation Mahony and Madgwick attitude estimation.
#![allow(
    clippy::missing_errors_doc,
    reason = "error variants document failures"
)]

use crate::{
    AccelerationG, AngleRadians, AngularVelocity, ImuAcceleration, ImuAngularRate,
    ImuMagneticField, ImuOrientation, ImuQuaternion, ImuQuaternionW, ImuQuaternionX,
    ImuQuaternionY, ImuQuaternionZ, ImuReadSample, MahonyPitchGain, MahonyRollGain, Ratio,
};

fn initial_quaternion(
    acceleration: ImuAcceleration,
    magnetic: ImuMagneticField,
) -> Option<[f32; 4]> {
    let (ax, ay, az) = acceleration.map_axes(|x, y, z| {
        (
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        )
    });
    let (mx, my, mz) = magnetic.map_axes(|x, y, z| {
        (
            x.magnetic_flux_density().as_microteslas(),
            y.magnetic_flux_density().as_microteslas(),
            z.magnetic_flux_density().as_microteslas(),
        )
    });
    let accel_norm = crate::sqrt(ax * ax + ay * ay + az * az);
    let mag_norm = crate::sqrt(mx * mx + my * my + mz * mz);
    if !accel_norm.is_finite()
        || !mag_norm.is_finite()
        || accel_norm <= f32::EPSILON
        || mag_norm <= f32::EPSILON
    {
        return None;
    }
    let roll = crate::atan2(ay, az);
    let pitch = crate::atan2(-ax, crate::sqrt(ay * ay + az * az));
    let (sr, cr) = (crate::sin(roll), crate::cos(roll));
    let (sp, cp) = (crate::sin(pitch), crate::cos(pitch));
    let horizontal_x = mx * cp + mz * sp;
    let horizontal_y = mx * sr * sp + my * cr - mz * sr * cp;
    if crate::sqrt(horizontal_x * horizontal_x + horizontal_y * horizontal_y) <= f32::EPSILON {
        return None;
    }
    let yaw = crate::atan2(-horizontal_y, horizontal_x);
    let (sr, cr) = (crate::sin(roll * 0.5), crate::cos(roll * 0.5));
    let (sp, cp) = (crate::sin(pitch * 0.5), crate::cos(pitch * 0.5));
    let (sy, cy) = (crate::sin(yaw * 0.5), crate::cos(yaw * 0.5));
    Some([
        cr * cp * cy + sr * sp * sy,
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
    ])
}

fn orientation_from_quaternion(quaternion: [f32; 4]) -> ImuOrientation {
    ImuOrientation::from_quaternion(ImuQuaternion::from_components(
        ImuQuaternionW::new(quaternion[0]),
        ImuQuaternionX::new(quaternion[1]),
        ImuQuaternionY::new(quaternion[2]),
        ImuQuaternionZ::new(quaternion[3]),
    ))
}

/// Fixed-state Mahony filter with independent pitch and roll feedback gains.
///
/// Callers select their acceleration-magnitude smoothing and confidence falloff
/// on each update, keeping product policy outside the reusable filter state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMahony {
    orientation: [f32; 4],
    accel_magnitude: AccelerationG,
    feedback: [f32; 3],
}

impl Default for AxisMahony {
    fn default() -> Self {
        Self {
            orientation: [1.0, 0.0, 0.0, 0.0],
            accel_magnitude: AccelerationG::from_g(1.0),
            feedback: [2.0, 1.4, 1.7],
        }
    }
}

impl AxisMahony {
    /// Build the filter from an existing firmware orientation.
    #[must_use]
    pub fn from_orientation(orientation: ImuOrientation) -> Self {
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

    /// Configure pitch and roll feedback; yaw uses their midpoint.
    pub fn configure(&mut self, pitch: MahonyPitchGain, roll: MahonyRollGain) {
        self.feedback = [
            pitch.value(),
            roll.value(),
            f32::midpoint(pitch.value(), roll.value()),
        ];
    }

    /// Return the configured pitch and roll feedback gains.
    #[must_use]
    pub const fn configured_gains(&self) -> (MahonyPitchGain, MahonyRollGain) {
        (
            MahonyPitchGain::new(self.feedback[0]),
            MahonyRollGain::new(self.feedback[1]),
        )
    }

    /// Return the current normalized orientation.
    #[must_use]
    pub fn orientation(&self) -> ImuOrientation {
        orientation_from_quaternion(self.orientation)
    }

    /// Return pitch from the current quaternion with a bounded projection.
    #[must_use]
    pub fn pitch(&self) -> AngleRadians {
        let [scalar, body_x, body_y, body_z] = self.orientation;
        let projection = -2.0 * (body_x * body_z - scalar * body_y);
        AngleRadians::from_radians(crate::asin(projection.clamp(-1.0, 1.0)))
    }

    /// Integrate one IMU sample with caller-owned acceleration confidence policy.
    pub fn update(
        &mut self,
        sample: ImuReadSample,
        acceleration_smoothing: Ratio,
        confidence_falloff: f32,
    ) {
        let gyro = self.gyro_with_accel_correction(
            sample.angular_rate(),
            sample.acceleration(),
            acceleration_smoothing,
            confidence_falloff,
        );
        self.integrate_gyro(gyro, sample.period().duration());
        self.normalize_quaternion();
    }

    fn gyro_with_accel_correction(
        &mut self,
        gyro: ImuAngularRate,
        acceleration: ImuAcceleration,
        acceleration_smoothing: Ratio,
        confidence_falloff: f32,
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
                let confidence =
                    self.accel_confidence(accel_norm, acceleration_smoothing, confidence_falloff);
                let error = self.accel_error(measured_gravity);
                let gains = self.feedback_gains(confidence);
                core::array::from_fn(|axis| {
                    gyro[axis] + AngularVelocity::from_radians_per_second(gains[axis] * error[axis])
                })
            },
        )
    }

    fn measured_gravity(acceleration: ImuAcceleration) -> Option<(AccelerationG, [f32; 3])> {
        acceleration.map_axes(|x, y, z| {
            let measured = [
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            ];
            let norm = crate::sqrt(
                measured[0] * measured[0] + measured[1] * measured[1] + measured[2] * measured[2],
            );
            (norm > 0.01).then(|| {
                let reciprocal = 1.0 / norm;
                (
                    AccelerationG::from_g(norm),
                    measured.map(|axis| axis * reciprocal),
                )
            })
        })
    }

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

    fn integrate_gyro(&mut self, gyro: [AngularVelocity; 3], dt: crate::VescSeconds) {
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

    fn normalize_quaternion(&mut self) {
        let norm = crate::sqrt(self.orientation.iter().map(|value| value * value).sum());
        for value in &mut self.orientation {
            *value /= norm;
        }
    }

    fn accel_confidence(
        &mut self,
        magnitude: AccelerationG,
        smoothing: Ratio,
        falloff: f32,
    ) -> f32 {
        self.accel_magnitude = self.accel_magnitude * smoothing.complement().as_ratio()
            + magnitude * smoothing.as_ratio();
        (1.0 - falloff * crate::sqrt((self.accel_magnitude.as_g() - 1.0).abs())).max(0.0)
    }

    fn feedback_gains(&self, confidence: f32) -> [f32; 3] {
        let [pitch, roll, yaw] = self.feedback;
        [
            2.0 * roll * confidence,
            2.0 * pitch * confidence,
            2.0 * yaw * confidence,
        ]
    }
}

/// Invalid parameter returned by a package-owned AHRS configuration change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AhrsParameterError {
    /// The parameter was NaN or infinite.
    NonFinite,
    /// The parameter was finite but negative.
    Negative,
}

impl_error!(AhrsParameterError {
    NonFinite => "AHRS parameter must be finite",
    Negative => "AHRS parameter must not be negative",
});

fn validate_nonnegative(value: f32) -> Result<(), AhrsParameterError> {
    if !value.is_finite() {
        Err(AhrsParameterError::NonFinite)
    } else if value < 0.0 {
        Err(AhrsParameterError::Negative)
    } else {
        Ok(())
    }
}

/// Package-owned attitude estimator state.
#[derive(Debug, Clone, Copy)]
pub struct Ahrs {
    quaternion: [f32; 4],
    integral: [f32; 3],
    proportional_gain: f32,
    integral_gain: f32,
}

impl Default for Ahrs {
    fn default() -> Self {
        Self::new()
    }
}

impl Ahrs {
    /// Construct a Mahony estimator with conservative default gains.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            integral: [0.0; 3],
            proportional_gain: 2.0,
            integral_gain: 0.05,
        }
    }

    /// Construct an estimator with explicit proportional and integral gains.
    pub fn with_gains(
        proportional_gain: f32,
        integral_gain: f32,
    ) -> Result<Self, AhrsParameterError> {
        let mut estimator = Self::new();
        estimator.set_gains(proportional_gain, integral_gain)?;
        Ok(estimator)
    }

    /// Replace the estimator gains after validating both values.
    pub fn set_gains(
        &mut self,
        proportional_gain: f32,
        integral_gain: f32,
    ) -> Result<(), AhrsParameterError> {
        validate_nonnegative(proportional_gain)?;
        validate_nonnegative(integral_gain)?;
        self.proportional_gain = proportional_gain;
        self.integral_gain = integral_gain;
        Ok(())
    }

    /// Reset the orientation and accumulated integral correction.
    pub fn reset(&mut self) {
        self.quaternion = [1.0, 0.0, 0.0, 0.0];
        self.integral = [0.0; 3];
    }

    /// Return the current normalized attitude quaternion.
    #[must_use]
    pub fn orientation(&self) -> ImuOrientation {
        orientation_from_quaternion(self.quaternion)
    }

    /// Initialize package-owned attitude from gravity and magnetic north.
    pub fn update_initial_orientation(
        &mut self,
        acceleration: ImuAcceleration,
        magnetic: ImuMagneticField,
    ) -> ImuOrientation {
        if let Some(quaternion) = initial_quaternion(acceleration, magnetic) {
            self.quaternion = quaternion;
            self.integral = [0.0; 3];
        } else {
            self.reset();
        }
        self.orientation()
    }

    /// Integrate one copied firmware IMU sample.
    pub fn update(&mut self, sample: ImuReadSample) -> ImuOrientation {
        let dt = sample.period().duration().as_seconds();
        if !dt.is_finite() || dt <= 0.0 {
            return self.orientation();
        }

        let (ax, ay, az) = sample.acceleration().map_axes(|x, y, z| {
            (
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            )
        });
        let (mut gx, mut gy, mut gz) = (
            sample.angular_rate().roll().as_radians_per_second(),
            sample.angular_rate().pitch().as_radians_per_second(),
            sample.angular_rate().yaw().as_radians_per_second(),
        );
        let (q0, q1, q2, q3) = (
            self.quaternion[0],
            self.quaternion[1],
            self.quaternion[2],
            self.quaternion[3],
        );

        let accel_norm = crate::sqrt(ax * ax + ay * ay + az * az);
        if accel_norm.is_finite() && accel_norm > f32::EPSILON {
            let ax = ax / accel_norm;
            let ay = ay / accel_norm;
            let az = az / accel_norm;
            let vx = 2.0 * (q1 * q3 - q0 * q2);
            let vy = 2.0 * (q0 * q1 + q2 * q3);
            let vz = q0 * q0 - q1 * q1 - q2 * q2 + q3 * q3;
            let error = [ay * vz - az * vy, az * vx - ax * vz, ax * vy - ay * vx];
            for (integral, error) in self.integral.iter_mut().zip(error) {
                *integral += self.integral_gain * error * dt;
            }
            gx += self.proportional_gain * error[0] + self.integral[0];
            gy += self.proportional_gain * error[1] + self.integral[1];
            gz += self.proportional_gain * error[2] + self.integral[2];
        }

        let half_dt = 0.5 * dt;
        self.quaternion[0] += (-q1 * gx - q2 * gy - q3 * gz) * half_dt;
        self.quaternion[1] += (q0 * gx + q2 * gz - q3 * gy) * half_dt;
        self.quaternion[2] += (q0 * gy - q1 * gz + q3 * gx) * half_dt;
        self.quaternion[3] += (q0 * gz + q1 * gy - q2 * gx) * half_dt;
        let norm = crate::sqrt(self.quaternion.iter().map(|value| value * value).sum());
        if norm.is_finite() && norm > f32::EPSILON {
            for value in &mut self.quaternion {
                *value /= norm;
            }
        } else {
            self.reset();
        }
        self.orientation()
    }
}

/// Package-owned six-degree-of-freedom Madgwick attitude estimator.
///
/// This estimator uses the copied accelerometer and gyroscope sample. The
/// magnetometer is intentionally ignored because the firmware sample does not
/// provide a calibrated magnetic reference for this package API.
#[derive(Debug, Clone, Copy)]
pub struct Madgwick {
    quaternion: [f32; 4],
    beta: f32,
}

impl Default for Madgwick {
    fn default() -> Self {
        Self::new()
    }
}

impl Madgwick {
    /// Construct an estimator with the conventional beta gain of `0.1`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            beta: 0.1,
        }
    }

    /// Construct an estimator with an explicit non-negative beta gain.
    pub fn with_beta(beta: f32) -> Result<Self, AhrsParameterError> {
        let mut estimator = Self::new();
        estimator.set_beta(beta)?;
        Ok(estimator)
    }

    /// Replace the beta gain after validating it is finite and non-negative.
    pub fn set_beta(&mut self, beta: f32) -> Result<(), AhrsParameterError> {
        validate_nonnegative(beta)?;
        self.beta = beta;
        Ok(())
    }

    /// Reset the estimator to the identity orientation.
    pub fn reset(&mut self) {
        self.quaternion = [1.0, 0.0, 0.0, 0.0];
    }

    /// Return the current normalized attitude quaternion.
    #[must_use]
    pub fn orientation(&self) -> ImuOrientation {
        orientation_from_quaternion(self.quaternion)
    }

    /// Initialize package-owned attitude from gravity and magnetic north.
    pub fn update_initial_orientation(
        &mut self,
        acceleration: ImuAcceleration,
        magnetic: ImuMagneticField,
    ) -> ImuOrientation {
        self.quaternion =
            initial_quaternion(acceleration, magnetic).unwrap_or([1.0, 0.0, 0.0, 0.0]);
        self.orientation()
    }

    /// Integrate one copied firmware IMU sample.
    pub fn update(&mut self, sample: ImuReadSample) -> ImuOrientation {
        let dt = sample.period().duration().as_seconds();
        if !dt.is_finite() || dt <= 0.0 {
            return self.orientation();
        }

        let (mut ax, mut ay, mut az) = sample.acceleration().map_axes(|x, y, z| {
            (
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            )
        });
        let (gx, gy, gz) = (
            sample.angular_rate().roll().as_radians_per_second(),
            sample.angular_rate().pitch().as_radians_per_second(),
            sample.angular_rate().yaw().as_radians_per_second(),
        );
        let (q0, q1, q2, q3) = (
            self.quaternion[0],
            self.quaternion[1],
            self.quaternion[2],
            self.quaternion[3],
        );

        let accel_norm = crate::sqrt(ax * ax + ay * ay + az * az);
        let mut s = [0.0; 4];
        if accel_norm.is_finite() && accel_norm > f32::EPSILON {
            ax /= accel_norm;
            ay /= accel_norm;
            az /= accel_norm;
            let f1 = 2.0 * (q1 * q3 - q0 * q2) - ax;
            let f2 = 2.0 * (q0 * q1 + q2 * q3) - ay;
            let f3 = 2.0 * (0.5 - q1 * q1 - q2 * q2) - az;
            s = [
                -2.0 * q2 * f1 + 2.0 * q1 * f2,
                2.0 * q3 * f1 + 2.0 * q0 * f2 - 4.0 * q1 * f3,
                -2.0 * q0 * f1 + 2.0 * q3 * f2 - 4.0 * q2 * f3,
                2.0 * q1 * f1 + 2.0 * q2 * f2,
            ];
            let gradient_norm = crate::sqrt(s.iter().map(|value| value * value).sum());
            if gradient_norm.is_finite() && gradient_norm > f32::EPSILON {
                for value in &mut s {
                    *value /= gradient_norm;
                }
            } else {
                s = [0.0; 4];
            }
        }

        let half = 0.5;
        let qdot = [
            half * (-q1 * gx - q2 * gy - q3 * gz) - self.beta * s[0],
            half * (q0 * gx + q2 * gz - q3 * gy) - self.beta * s[1],
            half * (q0 * gy - q1 * gz + q3 * gx) - self.beta * s[2],
            half * (q0 * gz + q1 * gy - q2 * gx) - self.beta * s[3],
        ];
        for (value, derivative) in self.quaternion.iter_mut().zip(qdot) {
            *value += derivative * dt;
        }
        let norm = crate::sqrt(self.quaternion.iter().map(|value| value * value).sum());
        if norm.is_finite() && norm > f32::EPSILON {
            for value in &mut self.quaternion {
                *value /= norm;
            }
        } else {
            self.reset();
        }
        self.orientation()
    }
}
