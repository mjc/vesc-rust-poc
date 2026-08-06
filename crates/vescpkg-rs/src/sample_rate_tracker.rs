//! Allocation-free observed sample-rate tracking.

// TODO(vescpkg-rs): Move Refloat's "running or first update" adoption gate and
// restart settling behavior into policy before presenting this tracker as
// package-neutral.

use crate::{Frequency, Ratio, SampleRate, TimestampTicks, VescSeconds};

/// Policy controlling when an observed sample rate becomes the new filter rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleRateTrackingPolicy {
    cutoff: Frequency,
    settle_seconds: u32,
    minimum_relative_change: Ratio,
}

impl SampleRateTrackingPolicy {
    /// Define the estimator cutoff and rate-change acceptance boundary.
    #[must_use]
    pub const fn new(
        cutoff: Frequency,
        settle_seconds: u32,
        minimum_relative_change: Ratio,
    ) -> Self {
        Self {
            cutoff,
            settle_seconds,
            minimum_relative_change,
        }
    }
}

/// Exponentially smoothed sample rate with guarded filter-rate updates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleRateTracker {
    elapsed: VescSeconds,
    frequency: SampleRate,
    filter_frequency: SampleRate,
    alpha: f32,
    filter_last_update: TimestampTicks,
    first: bool,
    running: bool,
}

impl SampleRateTracker {
    /// Seed a tracker from the expected rate and current firmware timestamp.
    #[must_use]
    pub fn new(
        frequency: SampleRate,
        now: TimestampTicks,
        policy: SampleRateTrackingPolicy,
    ) -> Self {
        Self {
            elapsed: VescSeconds::ZERO,
            frequency,
            filter_frequency: frequency,
            alpha: crate::ema_alpha(policy.cutoff, frequency),
            filter_last_update: now,
            first: true,
            running: false,
        }
    }

    /// Incorporate one observed sample period.
    pub fn update(&mut self, elapsed: VescSeconds) {
        self.elapsed = elapsed;
        let target = 1.0 / elapsed.as_seconds();
        self.frequency = SampleRate::from_hertz(
            self.frequency.as_hertz() + self.alpha * (target - self.frequency.as_hertz()),
        );
    }

    /// Adopt a sufficiently changed rate after its settling interval.
    pub fn check(
        &mut self,
        running: bool,
        now: TimestampTicks,
        policy: SampleRateTrackingPolicy,
    ) -> Option<SampleRate> {
        if !self.running && running {
            self.filter_last_update = now;
        }
        self.running = running;

        let change = (1.0 - self.frequency.as_hertz() / self.filter_frequency.as_hertz()).abs();
        if (running || self.first)
            && crate::timer_older_whole_seconds(now, self.filter_last_update, policy.settle_seconds)
            && change > policy.minimum_relative_change.as_ratio()
        {
            self.filter_frequency = self.frequency;
            self.alpha = crate::ema_alpha(policy.cutoff, self.filter_frequency);
            self.filter_last_update = now;
            self.first = false;
            Some(self.filter_frequency)
        } else {
            None
        }
    }

    /// Return the rate used to configure dependent filters.
    #[must_use]
    pub const fn filter_frequency(self) -> SampleRate {
        self.filter_frequency
    }

    /// Return the latest observed sample period.
    #[must_use]
    pub const fn elapsed(self) -> VescSeconds {
        self.elapsed
    }

    /// Return the smoothed observed sample rate.
    #[must_use]
    pub const fn frequency(self) -> SampleRate {
        self.frequency
    }
}

#[cfg(test)]
mod tests {
    use super::{SampleRateTracker, SampleRateTrackingPolicy};
    use crate::{Frequency, Ratio, SampleRate, TimestampTicks, VescSeconds};

    const POLICY: SampleRateTrackingPolicy =
        SampleRateTrackingPolicy::new(Frequency::from_hertz(1.0), 1, Ratio::from_ratio_const(0.03));

    fn tracker() -> SampleRateTracker {
        SampleRateTracker::new(
            SampleRate::from_hertz(500.0),
            TimestampTicks::from_ticks(0),
            POLICY,
        )
    }

    #[test]
    fn stable_cadence_preserves_the_seed_rate() {
        let mut tracker = tracker();
        tracker.update(VescSeconds::from_seconds(0.002));

        assert_eq!(tracker.elapsed(), VescSeconds::from_seconds(0.002));
        assert_eq!(tracker.frequency(), SampleRate::from_hertz(500.0));
        assert_eq!(
            tracker.check(false, TimestampTicks::from_ticks(10_001), POLICY),
            None,
        );
    }

    #[test]
    fn changed_rate_is_adopted_after_the_strict_settle_boundary() {
        let mut tracker = tracker();
        for _ in 0..500 {
            tracker.update(VescSeconds::from_seconds(0.004));
        }

        assert_eq!(
            tracker.check(false, TimestampTicks::from_ticks(10_000), POLICY),
            None,
        );
        let changed = tracker
            .check(false, TimestampTicks::from_ticks(10_001), POLICY)
            .expect("frequency changed");
        assert!(changed.as_hertz() < 300.0);
    }

    #[test]
    fn entering_running_state_restarts_the_settle_timer() {
        let mut tracker = tracker();
        for _ in 0..500 {
            tracker.update(VescSeconds::from_seconds(0.004));
        }

        assert_eq!(
            tracker.check(true, TimestampTicks::from_ticks(20_000), POLICY),
            None,
        );
        assert_eq!(
            tracker.check(true, TimestampTicks::from_ticks(30_000), POLICY),
            None,
        );
        assert!(
            tracker
                .check(true, TimestampTicks::from_ticks(30_001), POLICY)
                .is_some()
        );
    }

    #[test]
    fn zero_elapsed_sample_preserves_nonfinite_frequency_semantics() {
        let mut tracker = tracker();
        tracker.update(VescSeconds::ZERO);
        assert!(!tracker.frequency().as_hertz().is_finite());
    }
}
