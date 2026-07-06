//! IMU semantic wrappers over firmware-proven units.

use core::{fmt, marker::PhantomData};

use crate::units::{AccelerationG, AngleRadians, AngularVelocity, VescSeconds};

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

macro_rules! axis_type {
    ($name:ident, $unit:ty, $getter:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(transparent)]
        pub struct $name($unit);

        impl $name {
            /// Wrap one typed hardware axis.
            pub const fn new(value: $unit) -> Self {
                Self(value)
            }

            /// Return the typed unit for this axis.
            pub const fn $getter(self) -> $unit {
                self.0
            }
        }
    };
}

axis_type!(
    ImuAccelerationX,
    AccelerationG,
    acceleration,
    "Hardware IMU acceleration x-axis sample in g units."
);
axis_type!(
    ImuAccelerationY,
    AccelerationG,
    acceleration,
    "Hardware IMU acceleration y-axis sample in g units."
);
axis_type!(
    ImuAccelerationZ,
    AccelerationG,
    acceleration,
    "Hardware IMU acceleration z-axis sample in g units."
);
axis_type!(
    ImuAngularRateRoll,
    AngularVelocity,
    angular_velocity,
    "Hardware IMU roll-axis angular-rate sample."
);
axis_type!(
    ImuAngularRatePitch,
    AngularVelocity,
    angular_velocity,
    "Hardware IMU pitch-axis angular-rate sample."
);
axis_type!(
    ImuAngularRateYaw,
    AngularVelocity,
    angular_velocity,
    "Hardware IMU yaw-axis angular-rate sample."
);

/// Firmware IMU acceleration vector in g units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAcceleration(AxisVector3<AccelerationG, ImuAccelerationTag>);

impl ImuAcceleration {
    /// Wrap named hardware acceleration axes.
    pub const fn from_axes(x: ImuAccelerationX, y: ImuAccelerationY, z: ImuAccelerationZ) -> Self {
        Self(AxisVector3::new([
            x.acceleration(),
            y.acceleration(),
            z.acceleration(),
        ]))
    }

    pub(crate) const fn from_firmware_axes(xyz: [AccelerationG; 3]) -> Self {
        Self(AxisVector3::new(xyz))
    }

    /// Visit named axes without exposing firmware-order arrays.
    pub fn map_axes<R>(
        self,
        f: impl FnOnce(ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ) -> R,
    ) -> R {
        f(
            ImuAccelerationX::new(self.0.x()),
            ImuAccelerationY::new(self.0.y()),
            ImuAccelerationZ::new(self.0.z()),
        )
    }
}

/// Firmware IMU angular-rate vector from `imu_get_gyro`, in degrees/sec.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAngularRate(AxisVector3<AngularVelocity, ImuAngularRateTag>);

impl ImuAngularRate {
    /// Wrap named hardware angular-rate axes.
    pub const fn from_axes(
        roll: ImuAngularRateRoll,
        pitch: ImuAngularRatePitch,
        yaw: ImuAngularRateYaw,
    ) -> Self {
        Self(AxisVector3::new([
            roll.angular_velocity(),
            pitch.angular_velocity(),
            yaw.angular_velocity(),
        ]))
    }

    pub(crate) const fn from_firmware_axes(xyz: [AngularVelocity; 3]) -> Self {
        Self(AxisVector3::new(xyz))
    }

    /// Visit named axes without exposing firmware-order arrays.
    pub fn map_axes<R>(
        self,
        f: impl FnOnce(ImuAngularRateRoll, ImuAngularRatePitch, ImuAngularRateYaw) -> R,
    ) -> R {
        f(
            ImuAngularRateRoll::new(self.0.x()),
            ImuAngularRatePitch::new(self.0.y()),
            ImuAngularRateYaw::new(self.0.z()),
        )
    }

    /// Return roll-axis angular rate.
    pub const fn roll(self) -> AngularVelocity {
        self.0.x()
    }

    /// Return pitch-axis angular rate.
    pub const fn pitch(self) -> AngularVelocity {
        self.0.y()
    }

    /// Return yaw-axis angular rate.
    pub const fn yaw(self) -> AngularVelocity {
        self.0.z()
    }
}

macro_rules! quaternion_component_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Wrap one hardware quaternion component.
            pub const fn new(value: f32) -> Self {
                Self(value)
            }
        }
    };
}

quaternion_component_type!(
    ImuQuaternionW,
    "Hardware IMU attitude quaternion scalar component."
);
quaternion_component_type!(
    ImuQuaternionX,
    "Hardware IMU attitude quaternion x component."
);
quaternion_component_type!(
    ImuQuaternionY,
    "Hardware IMU attitude quaternion y component."
);
quaternion_component_type!(
    ImuQuaternionZ,
    "Hardware IMU attitude quaternion z component."
);

/// Firmware IMU attitude quaternion components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct ImuQuaternion([f32; 4]);

impl ImuQuaternion {
    /// Create an IMU attitude quaternion from named firmware-order components.
    pub const fn from_components(
        w: ImuQuaternionW,
        x: ImuQuaternionX,
        y: ImuQuaternionY,
        z: ImuQuaternionZ,
    ) -> Self {
        Self([w.0, x.0, y.0, z.0])
    }

    pub(crate) const fn from_firmware_wxyz(components: [f32; 4]) -> Self {
        Self(components)
    }

    /// Visit named components without exposing firmware-order arrays.
    pub fn map_components<R>(
        self,
        f: impl FnOnce(ImuQuaternionW, ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ) -> R,
    ) -> R {
        let [w, x, y, z] = self.0;
        f(
            ImuQuaternionW::new(w),
            ImuQuaternionX::new(x),
            ImuQuaternionY::new(y),
            ImuQuaternionZ::new(z),
        )
    }
}

/// Time between firmware IMU read callbacks.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ImuSamplePeriod(VescSeconds);

impl ImuSamplePeriod {
    /// Wrap the callback period.
    pub const fn new(duration: VescSeconds) -> Self {
        Self(duration)
    }

    /// Return the typed duration.
    pub const fn duration(self) -> VescSeconds {
        self.0
    }
}

/// Hardware IMU sample copied by the firmware read callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuReadSample {
    acceleration: ImuAcceleration,
    angular_rate: ImuAngularRate,
    period: ImuSamplePeriod,
}

impl ImuReadSample {
    /// Build a sample from typed hardware values.
    pub const fn from_parts(
        acceleration: ImuAcceleration,
        angular_rate: ImuAngularRate,
        period: ImuSamplePeriod,
    ) -> Self {
        Self {
            acceleration,
            angular_rate,
            period,
        }
    }

    pub(crate) fn from_firmware_raw(accel: [f32; 3], gyro: [f32; 3], dt: f32) -> Self {
        Self::from_parts(
            ImuAcceleration::from_firmware_axes([
                AccelerationG::from_g(accel[0]),
                AccelerationG::from_g(accel[1]),
                AccelerationG::from_g(accel[2]),
            ]),
            ImuAngularRate::from_firmware_axes([
                AngularVelocity::from_degrees_per_second(gyro[0]),
                AngularVelocity::from_degrees_per_second(gyro[1]),
                AngularVelocity::from_degrees_per_second(gyro[2]),
            ]),
            ImuSamplePeriod::new(VescSeconds::from_seconds(dt)),
        )
    }

    /// Return the hardware acceleration sample.
    pub const fn acceleration(self) -> ImuAcceleration {
        self.acceleration
    }

    /// Return the hardware angular-rate sample.
    pub const fn angular_rate(self) -> ImuAngularRate {
        self.angular_rate
    }

    /// Return the sample period.
    pub const fn period(self) -> ImuSamplePeriod {
        self.period
    }
}
