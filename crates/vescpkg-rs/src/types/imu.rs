//! IMU semantic wrappers over firmware-proven units.

use core::{fmt, marker::PhantomData};

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

struct AxisVector3<Component, Tag> {
    xyz: [Component; 3],
    _tag: PhantomData<Tag>,
}

impl<Component: Copy, Tag> Copy for AxisVector3<Component, Tag> {}

impl<Component: Copy, Tag> Clone for AxisVector3<Component, Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Component: fmt::Debug, Tag> fmt::Debug for AxisVector3<Component, Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AxisVector3").field(&self.xyz).finish()
    }
}

impl<Component: PartialEq, Tag> PartialEq for AxisVector3<Component, Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.xyz == other.xyz
    }
}

impl<Component, Tag> AxisVector3<Component, Tag> {
    const fn new(xyz: [Component; 3]) -> Self {
        Self {
            xyz,
            _tag: PhantomData,
        }
    }
}

impl<Component: Copy, Tag> AxisVector3<Component, Tag> {
    const fn xyz(self) -> [Component; 3] {
        self.xyz
    }

    const fn x(self) -> Component {
        self.xyz[0]
    }

    const fn y(self) -> Component {
        self.xyz[1]
    }

    const fn z(self) -> Component {
        self.xyz[2]
    }
}

enum ImuAccelerationTag {}
enum ImuAngularRateTag {}

/// Firmware IMU acceleration vector in g units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAcceleration(AxisVector3<AccelerationG, ImuAccelerationTag>);

impl ImuAcceleration {
    /// Wrap firmware acceleration axes.
    pub const fn new(xyz: [AccelerationG; 3]) -> Self {
        Self(AxisVector3::new(xyz))
    }

    /// Return typed acceleration axes without erasing them to primitives.
    pub const fn xyz(self) -> [AccelerationG; 3] {
        self.0.xyz()
    }

    /// Return x-axis acceleration.
    pub const fn x(self) -> AccelerationG {
        self.0.x()
    }

    /// Return y-axis acceleration.
    pub const fn y(self) -> AccelerationG {
        self.0.y()
    }

    /// Return z-axis acceleration.
    pub const fn z(self) -> AccelerationG {
        self.0.z()
    }
}

/// Firmware IMU angular-rate vector from `imu_get_gyro`, in degrees/sec.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAngularRate(AxisVector3<AngularVelocity, ImuAngularRateTag>);

impl ImuAngularRate {
    /// Wrap firmware public gyro getter axes.
    pub const fn new(xyz: [AngularVelocity; 3]) -> Self {
        Self(AxisVector3::new(xyz))
    }

    /// Return typed angular-rate axes without erasing them to primitives.
    pub const fn xyz(self) -> [AngularVelocity; 3] {
        self.0.xyz()
    }

    /// Return x-axis angular rate.
    pub const fn x(self) -> AngularVelocity {
        self.0.x()
    }

    /// Return y-axis angular rate.
    pub const fn y(self) -> AngularVelocity {
        self.0.y()
    }

    /// Return z-axis angular rate.
    pub const fn z(self) -> AngularVelocity {
        self.0.z()
    }

    /// Return roll-axis angular rate.
    pub const fn roll(self) -> AngularVelocity {
        self.x()
    }

    /// Return pitch-axis angular rate.
    pub const fn pitch(self) -> AngularVelocity {
        self.y()
    }

    /// Return yaw-axis angular rate.
    pub const fn yaw(self) -> AngularVelocity {
        self.z()
    }
}

/// Firmware IMU attitude quaternion components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct ImuQuaternion([f32; 4]);

impl ImuQuaternion {
    /// Create an IMU attitude quaternion from firmware-order `[q0, q1, q2, q3]` components.
    pub const fn from_components(components: [f32; 4]) -> Self {
        Self(components)
    }

    /// Compatibility constructor for call sites that already hold firmware-order components.
    pub const fn new(components: [f32; 4]) -> Self {
        Self::from_components(components)
    }

    /// Return quaternion components in firmware order.
    pub const fn components(self) -> [f32; 4] {
        self.0
    }
}
