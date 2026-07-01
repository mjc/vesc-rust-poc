//! IMU semantic wrappers over firmware-proven units.

use crate::units::{AccelerationG, AngleRadians, AngularVelocity};

macro_rules! attitude_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(AngleRadians);

        impl $name {
            /// Wrap a generic radian angle with VESC IMU meaning.
            pub const fn new(angle: AngleRadians) -> Self {
                Self(angle)
            }

            /// Return the typed angle without erasing it to a primitive.
            pub const fn angle(self) -> AngleRadians {
                self.0
            }
        }
    };
}

attitude_type!(ImuRoll, "IMU roll angle returned by firmware in radians.");
attitude_type!(ImuPitch, "IMU pitch angle returned by firmware in radians.");
attitude_type!(ImuYaw, "IMU yaw angle returned by firmware in radians.");

/// Firmware IMU acceleration vector in g units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAcceleration([AccelerationG; 3]);

impl ImuAcceleration {
    /// Wrap firmware acceleration axes.
    pub const fn new(xyz: [AccelerationG; 3]) -> Self {
        Self(xyz)
    }

    /// Return typed acceleration axes without erasing them to primitives.
    pub const fn xyz(self) -> [AccelerationG; 3] {
        self.0
    }
}

/// Firmware IMU angular-rate vector from `imu_get_gyro`, in degrees/sec.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAngularRate([AngularVelocity; 3]);

impl ImuAngularRate {
    /// Wrap firmware public gyro getter axes.
    pub const fn new(xyz: [AngularVelocity; 3]) -> Self {
        Self(xyz)
    }

    /// Return typed angular-rate axes without erasing them to primitives.
    pub const fn xyz(self) -> [AngularVelocity; 3] {
        self.0
    }
}

/// Firmware IMU attitude quaternion components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuQuaternion([f32; 4]);

impl ImuQuaternion {
    /// Wrap quaternion components in firmware order `[q0, q1, q2, q3]`.
    pub const fn new(components: [f32; 4]) -> Self {
        Self(components)
    }

    /// Return quaternion components in firmware order.
    pub const fn components(self) -> [f32; 4] {
        self.0
    }
}
