use super::gravity::{HalfGravity, HalfGravityX, HalfGravityY, HalfGravityZ};
use super::rate::AngularRateHalfStep;
use super::scalar::AxisScalar;
use crate::domain::FloatOutBoyRealtimeBalancePitch;
use vescpkg_rs::asin;
use vescpkg_rs::prelude::AngleRadians;
use vescpkg_rs::prelude::ImuOrientation;

enum OrientationScalarTag {}
enum OrientationBodyXTag {}
enum OrientationBodyYTag {}
enum OrientationBodyZTag {}

type OrientationScalar = AxisScalar<OrientationScalarTag>;
type OrientationBodyX = AxisScalar<OrientationBodyXTag>;
type OrientationBodyY = AxisScalar<OrientationBodyYTag>;
type OrientationBodyZ = AxisScalar<OrientationBodyZTag>;

enum OrientationChangeScalarTag {}
enum OrientationChangeBodyXTag {}
enum OrientationChangeBodyYTag {}
enum OrientationChangeBodyZTag {}

type OrientationChangeScalar = AxisScalar<OrientationChangeScalarTag>;
type OrientationChangeBodyX = AxisScalar<OrientationChangeBodyXTag>;
type OrientationChangeBodyY = AxisScalar<OrientationChangeBodyYTag>;
type OrientationChangeBodyZ = AxisScalar<OrientationChangeBodyZTag>;

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrientationVector {
    x: OrientationBodyX,
    y: OrientationBodyY,
    z: OrientationBodyZ,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EstimatedOrientation([f32; 4]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OrientationChange([f32; 4]);

impl OrientationVector {
    #[inline]
    const fn new(x: OrientationBodyX, y: OrientationBodyY, z: OrientationBodyZ) -> Self {
        Self { x, y, z }
    }
}

impl EstimatedOrientation {
    #[inline]
    const fn new(scalar: OrientationScalar, vector: OrientationVector) -> Self {
        Self([scalar.0, vector.x.0, vector.y.0, vector.z.0])
    }

    pub(super) const fn source_startup() -> Self {
        Self::new(
            OrientationScalar::new(1.0),
            OrientationVector::new(
                OrientationBodyX::new(0.0),
                OrientationBodyY::new(0.0),
                OrientationBodyZ::new(0.0),
            ),
        )
    }

    pub(super) fn from_orientation(orientation: ImuOrientation) -> Self {
        let quaternion = orientation.quaternion();
        Self::new(
            OrientationScalar::new(f32::from(quaternion.w())),
            OrientationVector::new(
                OrientationBodyX::new(f32::from(quaternion.x())),
                OrientationBodyY::new(f32::from(quaternion.y())),
                OrientationBodyZ::new(f32::from(quaternion.z())),
            ),
        )
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:145-154`.
    #[inline]
    pub(super) fn balance_pitch(self) -> FloatOutBoyRealtimeBalancePitch {
        FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(asin(
            self.pitch_projection().clamp(-1.0, 1.0),
        )))
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:145-154`.
    #[inline]
    fn pitch_projection(self) -> f32 {
        let [scalar, body_x, body_y, body_z] = self.0;
        -2.0 * (body_x * body_z - scalar * body_y)
    }

    #[inline]
    fn length_squared(self) -> f32 {
        // C map: `third_party/float-out-boy/src/balance_filter.c:126-133` normalizes
        // by the quaternion magnitude after integration.
        let [scalar, body_x, body_y, body_z] = self.0;
        scalar * scalar + body_x * body_x + body_y * body_y + body_z * body_z
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:98-101`.
    #[inline]
    pub(super) fn estimated_half_gravity(self) -> HalfGravity {
        let [scalar, body_x, body_y, body_z] = self.0;
        HalfGravity::new(
            HalfGravityX::new(body_x * body_z - scalar * body_y),
            HalfGravityY::new(scalar * body_x + body_y * body_z),
            HalfGravityZ::new(scalar * scalar - 0.5 + body_z * body_z),
        )
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:118-124`.
    #[inline]
    pub(super) fn change_from_angular_rate(
        self,
        rotation: AngularRateHalfStep,
    ) -> OrientationChange {
        let [scalar, body_x, body_y, body_z] = self.0;
        let [roll_rotation, pitch_rotation, yaw_rotation] = rotation.0;
        let roll_rotation = roll_rotation.as_radians();
        let pitch_rotation = pitch_rotation.as_radians();
        let yaw_rotation = yaw_rotation.as_radians();
        let vector_dot_rotation =
            body_x * roll_rotation + body_y * pitch_rotation + body_z * yaw_rotation;
        let vector_cross_rotation = [
            body_y * yaw_rotation - body_z * pitch_rotation,
            body_z * roll_rotation - body_x * yaw_rotation,
            body_x * pitch_rotation - body_y * roll_rotation,
        ];
        OrientationChange::new(
            OrientationChangeScalar::new(-vector_dot_rotation),
            OrientationChangeBodyX::new(scalar * roll_rotation + vector_cross_rotation[0]),
            OrientationChangeBodyY::new(scalar * pitch_rotation + vector_cross_rotation[1]),
            OrientationChangeBodyZ::new(scalar * yaw_rotation + vector_cross_rotation[2]),
        )
    }

    #[inline]
    pub(super) fn apply_change(&mut self, change: OrientationChange) {
        // C map: `third_party/float-out-boy/src/balance_filter.c:118-124` adds the
        // integrated quaternion delta into the current orientation.
        let [dq0, dq1, dq2, dq3] = change.wxyz();
        self.0[0] += dq0;
        self.0[1] += dq1;
        self.0[2] += dq2;
        self.0[3] += dq3;
    }

    #[inline]
    pub(super) fn normalize(&mut self) {
        // C map: `third_party/float-out-boy/src/balance_filter.c:38-40` uses
        // `1.0 / sqrtf(x)` to renormalize the quaternion.
        let recip_norm = sqrt(self.length_squared()).recip();
        self.0[0] *= recip_norm;
        self.0[1] *= recip_norm;
        self.0[2] *= recip_norm;
        self.0[3] *= recip_norm;
    }

    #[cfg(test)]
    pub(super) const fn wxyz_for_test(self) -> [f32; 4] {
        self.0
    }
}

impl OrientationChange {
    #[inline]
    const fn new(
        scalar: OrientationChangeScalar,
        x: OrientationChangeBodyX,
        y: OrientationChangeBodyY,
        z: OrientationChangeBodyZ,
    ) -> Self {
        // C map: `third_party/float-out-boy/src/balance_filter.c:118-124` builds
        // the quaternion delta in firmware component order.
        Self([scalar.0, x.0, y.0, z.0])
    }

    #[inline]
    const fn wxyz(self) -> [f32; 4] {
        self.0
    }
}

use vescpkg_rs::sqrt;

#[cfg(test)]
mod tests;
