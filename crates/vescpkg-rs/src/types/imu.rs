//! IMU semantic wrappers over firmware-proven units.

use core::{fmt, marker::PhantomData};

use crate::units::{
    AccelerationG, AngleRadians, AngularVelocity, MagneticFluxDensity, VescSeconds,
};

macro_rules! finite_imu_scalar {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Construct a checked firmware configuration value.
            pub fn try_new(value: f32) -> Option<Self> {
                value.is_finite().then_some(Self(value))
            }

            /// Return the scalar value without erasing its configuration meaning.
            pub const fn value(self) -> f32 {
                self.0
            }
        }
    };
}

finite_imu_scalar!(
    ImuMahonyProportionalGain,
    "Firmware Mahony proportional gain (`CFG_PARAM_IMU_mahony_kp`)."
);
finite_imu_scalar!(
    ImuMahonyIntegralGain,
    "Firmware Mahony integral gain (`CFG_PARAM_IMU_mahony_ki`)."
);
finite_imu_scalar!(
    ImuMadgwickBeta,
    "Firmware Madgwick beta gain (`CFG_PARAM_IMU_madgwick_beta`)."
);

/// Firmware accelerometer calibration offset in g units.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ImuAccelerationOffset(AccelerationG);

impl ImuAccelerationOffset {
    /// Construct an accelerometer offset from g units.
    pub fn try_new(value: AccelerationG) -> Option<Self> {
        value.as_g().is_finite().then_some(Self(value))
    }

    /// Return the offset in g units.
    pub const fn as_g(self) -> f32 {
        self.0.as_g()
    }
}

/// Firmware gyroscope calibration offset in degrees per second.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ImuAngularRateOffset(AngularVelocity);

impl ImuAngularRateOffset {
    /// Construct a gyroscope offset from degrees per second.
    pub fn try_new(value: AngularVelocity) -> Option<Self> {
        value
            .as_degrees_per_second()
            .is_finite()
            .then_some(Self(value))
    }

    /// Return the offset in degrees per second.
    pub const fn as_degrees_per_second(self) -> f32 {
        self.0.as_degrees_per_second()
    }
}

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

/// Firmware IMU attitude as roll, pitch, and yaw angles.
///
/// C map: VESC exposes these angles through `imu_get_roll`,
/// `imu_get_pitch`, and `imu_get_yaw` at
/// `third_party/vesc/imu/imu.c:375-384`; those values are radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuAttitude {
    roll: ImuRoll,
    pitch: ImuPitch,
    yaw: ImuYaw,
}
impl ImuAttitude {
    /// Group firmware IMU roll, pitch, and yaw into one attitude sample.
    pub const fn new(roll: ImuRoll, pitch: ImuPitch, yaw: ImuYaw) -> Self {
        Self { roll, pitch, yaw }
    }

    /// Return firmware IMU roll.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }

    /// Return firmware IMU pitch.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }

    /// Return firmware IMU yaw.
    pub const fn yaw(self) -> ImuYaw {
        self.yaw
    }
}

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
enum ImuMagneticFieldTag {}

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
axis_type!(
    ImuMagneticFieldX,
    MagneticFluxDensity,
    magnetic_flux_density,
    "Hardware IMU magnetic x-axis sample."
);
axis_type!(
    ImuMagneticFieldY,
    MagneticFluxDensity,
    magnetic_flux_density,
    "Hardware IMU magnetic y-axis sample."
);
axis_type!(
    ImuMagneticFieldZ,
    MagneticFluxDensity,
    magnetic_flux_density,
    "Hardware IMU magnetic z-axis sample."
);

/// Firmware IMU acceleration vector in g units.
///
/// C map: VESC copies the compensated acceleration vector into the public
/// IMU buffer at `third_party/vesc/imu/imu.c:391-393`.
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
///
/// C map: VESC copies the filtered degree-rate vector at
/// `third_party/vesc/imu/imu.c:395-396`; its package read callback converts
/// the same vector to radians/sec at `third_party/vesc/imu/imu.c:712-715`.
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

/// Firmware IMU magnetic-field vector in microteslas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuMagneticField(AxisVector3<MagneticFluxDensity, ImuMagneticFieldTag>);

impl ImuMagneticField {
    /// Wrap named hardware magnetic-field axes.
    pub const fn from_axes(
        x: ImuMagneticFieldX,
        y: ImuMagneticFieldY,
        z: ImuMagneticFieldZ,
    ) -> Self {
        Self(AxisVector3::new([
            x.magnetic_flux_density(),
            y.magnetic_flux_density(),
            z.magnetic_flux_density(),
        ]))
    }

    /// Visit named axes without exposing firmware-order arrays.
    pub fn map_axes<R>(
        self,
        f: impl FnOnce(ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ) -> R,
    ) -> R {
        f(
            ImuMagneticFieldX::new(self.0.x()),
            ImuMagneticFieldY::new(self.0.y()),
            ImuMagneticFieldZ::new(self.0.z()),
        )
    }
}

macro_rules! quaternion_component_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Wrap one typed quaternion component.
            #[must_use]
            pub const fn new(value: f32) -> Self {
                Self(value)
            }
        }

        impl From<$name> for f32 {
            fn from(component: $name) -> Self {
                component.0
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

/// Quaternion used by the firmware IMU orientation sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuQuaternion {
    w: ImuQuaternionW,
    x: ImuQuaternionX,
    y: ImuQuaternionY,
    z: ImuQuaternionZ,
}

impl ImuQuaternion {
    /// Create an IMU attitude quaternion from named components.
    #[must_use]
    pub const fn from_components(
        w: ImuQuaternionW,
        x: ImuQuaternionX,
        y: ImuQuaternionY,
        z: ImuQuaternionZ,
    ) -> Self {
        Self { w, x, y, z }
    }

    /// Return the scalar component.
    #[must_use]
    pub const fn w(self) -> ImuQuaternionW {
        self.w
    }

    /// Return the x component.
    #[must_use]
    pub const fn x(self) -> ImuQuaternionX {
        self.x
    }

    /// Return the y component.
    #[must_use]
    pub const fn y(self) -> ImuQuaternionY {
        self.y
    }

    /// Return the z component.
    #[must_use]
    pub const fn z(self) -> ImuQuaternionZ {
        self.z
    }

    /// Translate VESC's quaternion buffer at the private FFI boundary.
    ///
    /// C map: VESC writes q0..q3 into q[0..3] at
    /// `third_party/vesc/imu/imu.c:438-443`; Float Out Boy consumes that same
    /// order at `third_party/float-out-boy/src/balance_filter.c:54-59`.
    #[cfg(not(test))]
    pub(crate) const fn from_firmware_wxyz(components: [f32; 4]) -> Self {
        Self::from_components(
            ImuQuaternionW::new(components[0]),
            ImuQuaternionX::new(components[1]),
            ImuQuaternionY::new(components[2]),
            ImuQuaternionZ::new(components[3]),
        )
    }
}

/// Firmware IMU orientation sample.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct ImuOrientation(ImuQuaternion);

impl ImuOrientation {
    /// Identity orientation used before firmware has produced a sample.
    #[must_use]
    pub const fn identity() -> Self {
        Self::from_quaternion(ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(0.0),
            ImuQuaternionZ::new(0.0),
        ))
    }

    /// Wrap a typed quaternion as an IMU orientation.
    #[must_use]
    pub const fn from_quaternion(quaternion: ImuQuaternion) -> Self {
        Self(quaternion)
    }

    /// Return the typed quaternion represented by this orientation.
    #[must_use]
    pub const fn quaternion(self) -> ImuQuaternion {
        self.0
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
///
/// C map: VESC invokes the package callback with acceleration, converted
/// gyro radians/sec, magnetometer data, and elapsed seconds at
/// `third_party/vesc/imu/imu.c:581-595,743-745`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuReadSample {
    acceleration: ImuAcceleration,
    angular_rate: ImuAngularRate,
    magnetic_field: ImuMagneticField,
    period: ImuSamplePeriod,
}
impl ImuReadSample {
    /// Build a sample from typed hardware values.
    pub const fn from_parts(
        acceleration: ImuAcceleration,
        angular_rate: ImuAngularRate,
        magnetic_field: ImuMagneticField,
        period: ImuSamplePeriod,
    ) -> Self {
        Self {
            acceleration,
            angular_rate,
            magnetic_field,
            period,
        }
    }

    /// Return the hardware acceleration sample.
    pub const fn acceleration(self) -> ImuAcceleration {
        self.acceleration
    }

    /// Return the hardware angular-rate sample.
    pub const fn angular_rate(self) -> ImuAngularRate {
        self.angular_rate
    }

    /// Return the hardware magnetic-field sample.
    pub const fn magnetic_field(self) -> ImuMagneticField {
        self.magnetic_field
    }

    /// Return the sample period.
    pub const fn period(self) -> ImuSamplePeriod {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ImuAccelerationOffset, ImuAngularRateOffset, ImuMadgwickBeta, ImuMahonyIntegralGain,
        ImuMahonyProportionalGain,
    };
    use crate::units::{AccelerationG, AngularVelocity};

    #[test]
    fn configuration_scalars_reject_non_finite_values() {
        assert!(ImuMahonyProportionalGain::try_new(f32::NAN).is_none());
        assert!(ImuMahonyIntegralGain::try_new(f32::INFINITY).is_none());
        assert!(ImuMadgwickBeta::try_new(f32::NEG_INFINITY).is_none());
    }

    #[test]
    fn calibration_offsets_preserve_firmware_units() {
        let acceleration =
            ImuAccelerationOffset::try_new(AccelerationG::from_g(-0.125)).expect("finite g");
        assert_eq!(acceleration.as_g(), -0.125);

        let angular_rate =
            ImuAngularRateOffset::try_new(AngularVelocity::from_degrees_per_second(3.5))
                .expect("finite degree rate");
        assert_eq!(angular_rate.as_degrees_per_second(), 3.5);
    }
}
