use super::feedback::MahonyFeedbackGains;
use super::gravity::GravityError;
use vescpkg_rs::prelude::{
    AngleRadians, AngularVelocity, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll,
    ImuAngularRateYaw, VescSeconds,
};

pub(super) enum RollAngularRateTag {}
pub(super) enum PitchAngularRateTag {}
pub(super) enum YawAngularRateTag {}
enum RollAngularHalfStepTag {}
enum PitchAngularHalfStepTag {}
enum YawAngularHalfStepTag {}

pub(super) type RollAngularRate = AngularRateAxis<RollAngularRateTag>;
pub(super) type PitchAngularRate = AngularRateAxis<PitchAngularRateTag>;
pub(super) type YawAngularRate = AngularRateAxis<YawAngularRateTag>;
type RollAngularHalfStep = AngularHalfStepAxis<RollAngularHalfStepTag>;
type PitchAngularHalfStep = AngularHalfStepAxis<PitchAngularHalfStepTag>;
type YawAngularHalfStep = AngularHalfStepAxis<YawAngularHalfStepTag>;

pub(super) struct AngularRateAxis<Tag>(AngularVelocity, core::marker::PhantomData<fn() -> Tag>);

struct AngularHalfStepAxis<Tag>(AngleRadians, core::marker::PhantomData<fn() -> Tag>);

impl<Tag> core::fmt::Debug for AngularRateAxis<Tag> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("AngularRateAxis").field(&self.0).finish()
    }
}

impl<Tag> Clone for AngularRateAxis<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag> Copy for AngularRateAxis<Tag> {}

impl<Tag> PartialEq for AngularRateAxis<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Tag> core::fmt::Debug for AngularHalfStepAxis<Tag> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("AngularHalfStepAxis").field(&self.0).finish()
    }
}

impl<Tag> Clone for AngularHalfStepAxis<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag> Copy for AngularHalfStepAxis<Tag> {}

impl<Tag> PartialEq for AngularHalfStepAxis<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MeasuredAngularRate {
    roll: RollAngularRate,
    pitch: PitchAngularRate,
    yaw: YawAngularRate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CorrectedAngularRate {
    roll: RollAngularRate,
    pitch: PitchAngularRate,
    yaw: YawAngularRate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AngularRateHalfStep(pub(super) [AngleRadians; 3]);

impl<Tag> AngularRateAxis<Tag> {
    #[inline(always)]
    pub(super) const fn new(value: AngularVelocity) -> Self {
        // C map: `imu_update` keeps callback gyro axes as angular velocity
        // inputs before the balance filter integrates them at
        // `third_party/float-out-boy/src/main.c:760-765` and
        // `third_party/float-out-boy/src/balance_filter.c:73-76`.
        Self(value, core::marker::PhantomData)
    }

    #[inline(always)]
    const fn angular_velocity(self) -> AngularVelocity {
        self.0
    }
}

impl<Tag> AngularHalfStepAxis<Tag> {
    #[inline(always)]
    const fn new(value: AngleRadians) -> Self {
        // C map: `balance_filter_update` and `imu_update` carry these half
        // step rotation values as raw radians at
        // `third_party/float-out-boy/src/balance_filter.c:114-117`.
        Self(value, core::marker::PhantomData)
    }
}

impl MeasuredAngularRate {
    #[inline(always)]
    pub(super) const fn new(
        roll: RollAngularRate,
        pitch: PitchAngularRate,
        yaw: YawAngularRate,
    ) -> Self {
        Self { roll, pitch, yaw }
    }

    fn from_axes(
        roll: ImuAngularRateRoll,
        pitch: ImuAngularRatePitch,
        yaw: ImuAngularRateYaw,
    ) -> Self {
        // C map: Float Out Boy forwards the callback gyro sample into `balance_filter_update`
        // at `third_party/float-out-boy/src/main.c:760-765`; the filter integrates those axes
        // at `third_party/float-out-boy/src/balance_filter.c:73-76`.
        Self::new(
            RollAngularRate::new(roll.angular_velocity()),
            PitchAngularRate::new(pitch.angular_velocity()),
            YawAngularRate::new(yaw.angular_velocity()),
        )
    }

    #[inline(always)]
    pub(super) const fn without_accel_feedback(self) -> CorrectedAngularRate {
        CorrectedAngularRate {
            roll: self.roll,
            pitch: self.pitch,
            yaw: self.yaw,
        }
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:107-111`.
    #[inline(always)]
    pub(super) fn with_gravity_feedback(
        self,
        error: GravityError,
        gains: MahonyFeedbackGains,
    ) -> CorrectedAngularRate {
        CorrectedAngularRate {
            roll: RollAngularRate::new(
                self.roll.angular_velocity() + gains.roll_correction(error.roll_error()),
            ),
            pitch: PitchAngularRate::new(
                self.pitch.angular_velocity() + gains.pitch_correction(error.pitch_error()),
            ),
            yaw: YawAngularRate::new(
                self.yaw.angular_velocity() + gains.yaw_correction(error.yaw_error()),
            ),
        }
    }
}

impl From<ImuAngularRate> for MeasuredAngularRate {
    #[inline(always)]
    fn from(angular_rate: ImuAngularRate) -> Self {
        angular_rate.map_axes(Self::from_axes)
    }
}

impl CorrectedAngularRate {
    #[cfg(test)]
    pub(super) const fn new(
        roll: RollAngularRate,
        pitch: PitchAngularRate,
        yaw: YawAngularRate,
    ) -> Self {
        Self { roll, pitch, yaw }
    }

    /// C map: `third_party/float-out-boy/src/balance_filter.c:114-117`.
    #[inline(always)]
    pub(super) fn half_step(self, dt: VescSeconds) -> AngularRateHalfStep {
        AngularRateHalfStep::new(
            RollAngularHalfStep::new(self.roll.angular_velocity() * dt * 0.5),
            PitchAngularHalfStep::new(self.pitch.angular_velocity() * dt * 0.5),
            YawAngularHalfStep::new(self.yaw.angular_velocity() * dt * 0.5),
        )
    }

    #[cfg(test)]
    pub(super) const fn roll(self) -> AngularVelocity {
        self.roll.angular_velocity()
    }

    #[cfg(test)]
    pub(super) const fn pitch(self) -> AngularVelocity {
        self.pitch.angular_velocity()
    }

    #[cfg(test)]
    pub(super) const fn yaw(self) -> AngularVelocity {
        self.yaw.angular_velocity()
    }
}

impl AngularRateHalfStep {
    #[inline(always)]
    const fn new(
        roll: RollAngularHalfStep,
        pitch: PitchAngularHalfStep,
        yaw: YawAngularHalfStep,
    ) -> Self {
        Self([roll.0, pitch.0, yaw.0])
    }
}
