//! Package-owned, no-allocation Mahony and Madgwick attitude estimation.

use crate::{
    ImuAcceleration, ImuMagneticField, ImuOrientation, ImuQuaternion, ImuQuaternionW,
    ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ, ImuReadSample,
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
    pub const fn new() -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            integral: [0.0; 3],
            proportional_gain: 2.0,
            integral_gain: 0.05,
        }
    }

    /// Construct an estimator with explicit proportional and integral gains.
    pub const fn with_gains(proportional_gain: f32, integral_gain: f32) -> Self {
        Self {
            proportional_gain,
            integral_gain,
            ..Self::new()
        }
    }

    /// Replace the estimator gains, ignoring non-finite or negative values.
    pub fn set_gains(&mut self, proportional_gain: f32, integral_gain: f32) -> bool {
        if !proportional_gain.is_finite()
            || !integral_gain.is_finite()
            || proportional_gain < 0.0
            || integral_gain < 0.0
        {
            return false;
        }
        self.proportional_gain = proportional_gain;
        self.integral_gain = integral_gain;
        true
    }

    /// Reset the orientation and accumulated integral correction.
    pub fn reset(&mut self) {
        self.quaternion = [1.0, 0.0, 0.0, 0.0];
        self.integral = [0.0; 3];
    }

    /// Return the current normalized attitude quaternion.
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
    pub const fn new() -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            beta: 0.1,
        }
    }

    /// Construct an estimator with an explicit non-negative beta gain.
    pub const fn with_beta(beta: f32) -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            beta,
        }
    }

    /// Replace the beta gain, rejecting negative or non-finite values.
    pub fn set_beta(&mut self, beta: f32) -> bool {
        if !beta.is_finite() || beta < 0.0 {
            return false;
        }
        self.beta = beta;
        true
    }

    /// Reset the estimator to the identity orientation.
    pub fn reset(&mut self) {
        self.quaternion = [1.0, 0.0, 0.0, 0.0];
    }

    /// Return the current normalized attitude quaternion.
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
