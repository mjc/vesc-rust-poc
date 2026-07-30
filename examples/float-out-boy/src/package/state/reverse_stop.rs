use vescpkg_rs::WrappingTimer;
use vescpkg_rs::prelude::{AngleDegrees, Frequency, Rpm, SampleRate, TimestampTicks, VescSeconds};

const REVERSE_STOP_DISTANCE_METERS: f32 = 0.25;
const TARGET_STOP_ANGLE_DEGREES: f32 = 17.0;
const DISTANCE_PER_DEGREE: f32 = REVERSE_STOP_DISTANCE_METERS / TARGET_STOP_ANGLE_DEGREES;
const TIMER_ANGLE_THRESHOLD_DEGREES: f32 = TARGET_STOP_ANGLE_DEGREES / 2.0;
const MOTOR_STEP_DISTANCE_METERS: f32 = 0.01;
const ENTRY_ERPM: f32 = -200.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ReverseStop {
    start_setpoint: AngleDegrees,
    target_setpoint: AngleDegrees,
    start_distance_meters: f32,
    target_distance_meters: f32,
    current_distance_meters: f32,
    progress: f32,
    timer: WrappingTimer,
}

impl ReverseStop {
    pub(super) const fn new() -> Self {
        Self {
            start_setpoint: AngleDegrees::ZERO,
            target_setpoint: AngleDegrees::ZERO,
            start_distance_meters: 0.0,
            target_distance_meters: 0.0,
            current_distance_meters: 0.0,
            progress: 1.0,
            timer: WrappingTimer::started_at(TimestampTicks::from_ticks(0)),
        }
    }

    pub(super) fn reset(&mut self, distance_meters: f32) {
        self.start_setpoint = AngleDegrees::ZERO;
        self.target_setpoint = AngleDegrees::ZERO;
        self.start_distance_meters = distance_meters;
        self.target_distance_meters = 0.0;
        self.current_distance_meters = 0.0;
        self.progress = 1.0;
    }

    pub(super) fn update(
        &mut self,
        distance_meters: f32,
        erpm: Rpm,
        setpoint: AngleDegrees,
        now: TimestampTicks,
        enabled: bool,
        elapsed: VescSeconds,
    ) {
        if (!enabled || erpm.as_revolutions_per_minute() > ENTRY_ERPM) && self.progress >= 1.0 {
            self.start_distance_meters = distance_meters;
            return;
        }

        let new_distance = distance_meters - self.start_distance_meters;
        let distance_diff = (new_distance - self.current_distance_meters)
            * source_sign(self.target_distance_meters);

        if distance_diff < -2.0 * MOTOR_STEP_DISTANCE_METERS {
            self.target_setpoint = if self.target_setpoint.is_positive() {
                AngleDegrees::ZERO
            } else {
                AngleDegrees::from_degrees(TARGET_STOP_ANGLE_DEGREES)
            };
            self.start_setpoint = setpoint;
            self.start_distance_meters = distance_meters;
            self.current_distance_meters = 0.0;
            self.target_distance_meters = (self.start_setpoint - self.target_setpoint)
                .as_degrees()
                .abs()
                * DISTANCE_PER_DEGREE;
            if self.target_setpoint.is_positive() {
                self.target_distance_meters *= -1.0;
            }
            self.progress = if self.target_distance_meters.abs() < MOTOR_STEP_DISTANCE_METERS {
                self.target_distance_meters = 0.0;
                1.0
            } else {
                0.0
            };
            return;
        }

        if self.progress >= 1.0 {
            if distance_diff > 0.0 {
                self.start_distance_meters = distance_meters;
            }
            return;
        }

        if distance_diff > 0.0 {
            self.current_distance_meters = new_distance;
        }
        let target_progress = self.current_distance_meters / self.target_distance_meters;
        let alpha = vescpkg_rs::ema_alpha(
            Frequency::from_hertz(1.0),
            SampleRate::from_hertz(1.0 / elapsed.as_seconds()),
        );
        self.progress += alpha * (target_progress - self.progress);

        if self.progress >= 1.0 {
            self.target_distance_meters = 0.0;
            self.current_distance_meters = 0.0;
        }
        if !self.target_setpoint.is_positive()
            || setpoint.as_degrees() < TIMER_ANGLE_THRESHOLD_DEGREES
        {
            self.timer.restart(now);
        }
    }

    pub(super) fn setpoint(self) -> AngleDegrees {
        let progress = self.progress * self.progress * (3.0 - 2.0 * self.progress);
        self.start_setpoint + (self.target_setpoint - self.start_setpoint) * progress
    }

    pub(super) fn active(self) -> bool {
        self.target_setpoint.is_positive() || self.progress < 1.0
    }

    pub(super) fn should_stop(self, now: TimestampTicks) -> bool {
        let threshold = VescSeconds::from_seconds(3.0 - 2.0 * self.progress);
        self.timer.older_than(now, threshold)
            || self.target_setpoint.is_positive() && self.progress >= 1.0
    }
}

impl Default for ReverseStop {
    fn default() -> Self {
        Self::new()
    }
}

fn source_sign(value: f32) -> f32 {
    if value < 0.0 { -1.0 } else { 1.0 }
}
