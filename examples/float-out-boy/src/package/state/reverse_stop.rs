use core::ops::{Div, Sub};
use vescpkg_rs::prelude::{
    AngleDegrees, Distance, Frequency, Ratio, Rpm, SampleRate, SignedTripDistance, TimestampTicks,
    VescSeconds,
};

const REVERSE_STOP_DISTANCE: Distance = Distance::from_meters(0.25);
const TARGET_STOP_ANGLE: AngleDegrees = AngleDegrees::from_degrees(17.0);
const TIMER_ANGLE_THRESHOLD: AngleDegrees = AngleDegrees::from_degrees(8.5);
const MOTOR_STEP_DISTANCE: Distance = Distance::from_meters(0.01);
const ENTRY_ERPM: Rpm = Rpm::from_revolutions_per_minute(-200.0);
const COMPLETE: Ratio = Ratio::from_ratio_const(1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReverseStopEntryPolicy {
    Allow,
    Block,
}

impl ReverseStopEntryPolicy {
    #[must_use]
    pub(super) const fn from_enabled(enabled: bool) -> Self {
        if enabled { Self::Allow } else { Self::Block }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct DistancePerDegree(f32);

impl DistancePerDegree {
    const REVERSE_STOP: Self =
        Self(REVERSE_STOP_DISTANCE.as_meters() / TARGET_STOP_ANGLE.as_degrees());

    const fn distance_for(self, angle: AngleDegrees) -> Distance {
        Distance::from_meters(angle.as_degrees() * self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ReverseStop {
    start_setpoint: AngleDegrees,
    target_setpoint: AngleDegrees,
    start_distance: Distance,
    target_distance: Distance,
    current_distance: Distance,
    progress: Ratio,
    timer: TimestampTicks,
}

impl ReverseStop {
    pub(super) const fn new() -> Self {
        Self {
            start_setpoint: AngleDegrees::ZERO,
            target_setpoint: AngleDegrees::ZERO,
            start_distance: Distance::ZERO,
            target_distance: Distance::ZERO,
            current_distance: Distance::ZERO,
            progress: COMPLETE,
            timer: TimestampTicks::from_ticks(0),
        }
    }

    pub(super) fn reset(&mut self, distance: SignedTripDistance) {
        // C map: Refloat's `reverse_stop_reset` deliberately leaves the
        // existing timer epoch untouched.
        self.start_setpoint = AngleDegrees::ZERO;
        self.target_setpoint = AngleDegrees::ZERO;
        self.start_distance = distance.distance();
        self.target_distance = Distance::ZERO;
        self.current_distance = Distance::ZERO;
        self.progress = COMPLETE;
    }

    pub(super) fn update(
        &mut self,
        distance: SignedTripDistance,
        erpm: Rpm,
        setpoint: AngleDegrees,
        now: TimestampTicks,
        entry_policy: ReverseStopEntryPolicy,
        elapsed: VescSeconds,
    ) {
        if (matches!(entry_policy, ReverseStopEntryPolicy::Block) || erpm > ENTRY_ERPM)
            && self.progress == COMPLETE
        {
            self.start_distance = distance.distance();
            return;
        }

        let new_distance = distance.distance().sub(self.start_distance);
        let distance_diff = new_distance
            .sub(self.current_distance)
            .scaled_by(self.target_distance.signum());

        if distance_diff < MOTOR_STEP_DISTANCE.scaled_by(-2.0) {
            self.target_setpoint = if self.target_setpoint.is_positive() {
                AngleDegrees::ZERO
            } else {
                TARGET_STOP_ANGLE
            };
            self.start_setpoint = setpoint;
            self.start_distance = distance.distance();
            self.current_distance = Distance::ZERO;
            self.target_distance = DistancePerDegree::REVERSE_STOP
                .distance_for((self.start_setpoint - self.target_setpoint).abs());
            if self.target_setpoint.is_positive() {
                self.target_distance = self.target_distance.scaled_by(-1.0);
            }
            self.progress = if self.target_distance.abs() < MOTOR_STEP_DISTANCE {
                self.target_distance = Distance::ZERO;
                COMPLETE
            } else {
                Ratio::from_ratio_const(0.0)
            };
            // C map: Refloat returns before refreshing the timer on entry.
            return;
        }

        if self.progress == COMPLETE {
            if distance_diff.is_positive() {
                self.start_distance = distance.distance();
            }
            return;
        }

        if distance_diff.is_positive() {
            self.current_distance = new_distance;
        }
        let target_progress = self.current_distance.div(self.target_distance);
        let alpha = super::motor_kinematics::refloat_ema_alpha(
            Frequency::from_hertz(1.0),
            SampleRate::from_hertz(1.0 / elapsed.as_seconds()),
        );
        self.progress = Ratio::clamped(
            self.progress.as_ratio()
                + alpha.as_ratio() * (target_progress - self.progress.as_ratio()),
        );

        if self.progress == COMPLETE {
            self.target_distance = Distance::ZERO;
            self.current_distance = Distance::ZERO;
        }
        if !self.target_setpoint.is_positive() || setpoint < TIMER_ANGLE_THRESHOLD {
            self.timer = now;
        }
    }

    pub(super) fn setpoint(self) -> AngleDegrees {
        let progress = self.progress.as_ratio();
        let smoothstep = progress * progress * (3.0 - 2.0 * progress);
        self.start_setpoint + (self.target_setpoint - self.start_setpoint) * smoothstep
    }

    pub(super) fn active(self) -> bool {
        self.target_setpoint.is_positive() || self.progress != COMPLETE
    }

    pub(super) fn should_stop(self, now: TimestampTicks) -> bool {
        let threshold = VescSeconds::from_seconds(3.0 - 2.0 * self.progress.as_ratio());
        vescpkg_rs::timer_older(now, self.timer, threshold)
            || self.target_setpoint.is_positive() && self.progress == COMPLETE
    }
}

impl Default for ReverseStop {
    fn default() -> Self {
        Self::new()
    }
}
