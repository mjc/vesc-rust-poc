use super::frequency_tracker::*;
use vescpkg_rs::prelude::{SampleRate, TimestampTicks, VescSeconds};

#[test]
fn tracker_preserves_nominal_frequency_at_stable_cadence() {
    let mut tracker =
        FrequencyTracker::new(SampleRate::from_hertz(500.0), TimestampTicks::from_ticks(0));

    tracker.update(VescSeconds::from_seconds(0.002));

    assert_eq!(tracker.elapsed(), VescSeconds::from_seconds(0.002));
    assert_eq!(tracker.frequency(), SampleRate::from_hertz(500.0));
    assert_eq!(
        tracker.check(false, TimestampTicks::from_ticks(10_001)),
        None
    );
    assert_eq!(tracker.recalculations(), 0);
}

#[test]
fn tracker_recomputes_after_a_strict_second_and_three_percent_change() {
    let mut tracker =
        FrequencyTracker::new(SampleRate::from_hertz(500.0), TimestampTicks::from_ticks(0));
    for _ in 0..500 {
        tracker.update(VescSeconds::from_seconds(0.004));
    }

    assert_eq!(
        tracker.check(false, TimestampTicks::from_ticks(10_000)),
        None
    );
    let changed = tracker
        .check(false, TimestampTicks::from_ticks(10_001))
        .expect("frequency changed");
    assert!(changed.as_hertz() < 300.0);
    assert_eq!(tracker.recalculations(), 1);
}

#[test]
fn engaging_restarts_the_frequency_settle_timer() {
    let mut tracker =
        FrequencyTracker::new(SampleRate::from_hertz(500.0), TimestampTicks::from_ticks(0));
    for _ in 0..500 {
        tracker.update(VescSeconds::from_seconds(0.004));
    }

    assert_eq!(
        tracker.check(true, TimestampTicks::from_ticks(20_000)),
        None
    );
    assert_eq!(
        tracker.check(true, TimestampTicks::from_ticks(30_000)),
        None
    );
    assert!(
        tracker
            .check(true, TimestampTicks::from_ticks(30_001))
            .is_some()
    );
}

#[test]
fn zero_elapsed_sample_matches_refloat_nonfinite_frequency() {
    let mut tracker =
        FrequencyTracker::new(SampleRate::from_hertz(500.0), TimestampTicks::from_ticks(0));

    tracker.update(VescSeconds::ZERO);

    assert!(!tracker.frequency().as_hertz().is_finite());
}

#[test]
fn firmware_602_zero_imu_rate_uses_refloat_settling_seed() {
    assert_eq!(
        imu_start_frequency(SampleRate::from_hertz(0.0)),
        SampleRate::from_hertz(620.0),
    );
    assert_eq!(
        imu_start_frequency(SampleRate::from_hertz(833.0)),
        SampleRate::from_hertz(833.0),
    );
}
