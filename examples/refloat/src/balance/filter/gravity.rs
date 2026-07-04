use super::scalar::AxisScalar;
use vescpkg_rs::prelude::{ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AccelMagnitude(f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MeasuredGravity {
    x: MeasuredGravityX,
    y: MeasuredGravityY,
    z: MeasuredGravityZ,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HalfGravity {
    x: HalfGravityX,
    y: HalfGravityY,
    z: HalfGravityZ,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GravityError {
    roll: RollGravityError,
    pitch: PitchGravityError,
    yaw: YawGravityError,
}

pub(super) enum RollGravityErrorTag {}
pub(super) enum PitchGravityErrorTag {}
pub(super) enum YawGravityErrorTag {}
enum MeasuredGravityXTag {}
enum MeasuredGravityYTag {}
enum MeasuredGravityZTag {}
pub(super) enum HalfGravityXTag {}
pub(super) enum HalfGravityYTag {}
pub(super) enum HalfGravityZTag {}

pub(super) type RollGravityError = AxisScalar<RollGravityErrorTag>;
pub(super) type PitchGravityError = AxisScalar<PitchGravityErrorTag>;
pub(super) type YawGravityError = AxisScalar<YawGravityErrorTag>;
type MeasuredGravityX = AxisScalar<MeasuredGravityXTag>;
type MeasuredGravityY = AxisScalar<MeasuredGravityYTag>;
type MeasuredGravityZ = AxisScalar<MeasuredGravityZTag>;
pub(super) type HalfGravityX = AxisScalar<HalfGravityXTag>;
pub(super) type HalfGravityY = AxisScalar<HalfGravityYTag>;
pub(super) type HalfGravityZ = AxisScalar<HalfGravityZTag>;

impl AccelMagnitude {
    /// C map: `calculate_acc_confidence` low-pass filters `data->acc_mag` at
    /// `third_party/refloat/src/balance_filter.c:42-50`.
    #[inline(always)]
    pub(super) const fn blend_with_filtered(self, filtered_magnitude: f32) -> f32 {
        filtered_magnitude * 0.9 + self.0 * 0.1
    }
}

impl MeasuredGravity {
    /// C map: `third_party/refloat/src/balance_filter.c:82-96`.
    #[inline(always)]
    pub(super) fn from_acceleration(
        acceleration: ImuAcceleration,
    ) -> Option<(AccelMagnitude, Self)> {
        acceleration.map_axes(Self::from_axes)
    }

    fn from_axes(
        x: ImuAccelerationX,
        y: ImuAccelerationY,
        z: ImuAccelerationZ,
    ) -> Option<(AccelMagnitude, Self)> {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` extracts
        // axis samples before normalizing the measured gravity vector.
        Self::from_measured_axes(
            MeasuredGravityX::new(x.acceleration().as_g()),
            MeasuredGravityY::new(y.acceleration().as_g()),
            MeasuredGravityZ::new(z.acceleration().as_g()),
        )
    }

    fn from_measured_axes(
        x: MeasuredGravityX,
        y: MeasuredGravityY,
        z: MeasuredGravityZ,
    ) -> Option<(AccelMagnitude, Self)> {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` rejects
        // tiny accel vectors, otherwise stores the norm and normalized sample.
        let sample = Self { x, y, z };
        let length_squared = sample.length_squared();
        let accel_norm = sqrt(length_squared);
        match accel_norm {
            norm if norm > 0.01 => Some((AccelMagnitude(norm), sample.scaled(1.0 / norm))),
            _ => None,
        }
    }

    #[inline(always)]
    fn scaled(self, scale: f32) -> Self {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` multiplies
        // the measured gravity vector by the reciprocal norm.
        Self {
            x: MeasuredGravityX::new(self.x.0 * scale),
            y: MeasuredGravityY::new(self.y.0 * scale),
            z: MeasuredGravityZ::new(self.z.0 * scale),
        }
    }

    #[inline(always)]
    fn length_squared(self) -> f32 {
        // C map: `third_party/refloat/src/balance_filter.c:82-96` measures the
        // accel vector magnitude before confidence and normalization.
        self.x.0 * self.x.0 + self.y.0 * self.y.0 + self.z.0 * self.z.0
    }

    /// C map: `third_party/refloat/src/balance_filter.c:103-106`.
    #[inline(always)]
    pub(super) fn error_against(self, estimated_gravity: HalfGravity) -> GravityError {
        GravityError::new(
            RollGravityError::new(
                self.y.0 * estimated_gravity.z.0 - self.z.0 * estimated_gravity.y.0,
            ),
            PitchGravityError::new(
                self.z.0 * estimated_gravity.x.0 - self.x.0 * estimated_gravity.z.0,
            ),
            YawGravityError::new(
                self.x.0 * estimated_gravity.y.0 - self.y.0 * estimated_gravity.x.0,
            ),
        )
    }

    #[cfg(test)]
    pub(super) const fn x(self) -> f32 {
        self.x.0
    }

    #[cfg(test)]
    pub(super) const fn y(self) -> f32 {
        self.y.0
    }

    #[cfg(test)]
    pub(super) const fn z(self) -> f32 {
        self.z.0
    }
}

impl HalfGravity {
    #[inline(always)]
    pub(super) const fn new(x: HalfGravityX, y: HalfGravityY, z: HalfGravityZ) -> Self {
        // C map: `third_party/refloat/src/balance_filter.c:98-101` stores the
        // estimated gravity half-vector in firmware component order.
        Self { x, y, z }
    }
}

impl GravityError {
    #[inline(always)]
    const fn new(roll: RollGravityError, pitch: PitchGravityError, yaw: YawGravityError) -> Self {
        // C map: `third_party/refloat/src/balance_filter.c:103-106` stores
        // the gravity cross-product error components.
        Self { roll, pitch, yaw }
    }

    #[inline(always)]
    pub(super) const fn roll_error(self) -> RollGravityError {
        self.roll
    }

    #[inline(always)]
    pub(super) const fn pitch_error(self) -> PitchGravityError {
        self.pitch
    }

    #[inline(always)]
    pub(super) const fn yaw_error(self) -> YawGravityError {
        self.yaw
    }
}
use vescpkg_rs::sqrt;
