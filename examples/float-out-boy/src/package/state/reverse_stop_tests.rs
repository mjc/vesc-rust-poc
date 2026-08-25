use super::reverse_stop::{ReverseStop, ReverseStopEntryPolicy};
use vescpkg_rs::prelude::{AngleDegrees, Distance, Rpm, SignedTripDistance, TimestampTicks};

fn trip_distance(meters: f32) -> SignedTripDistance {
    SignedTripDistance::new(Distance::from_meters(meters))
}

#[test]
fn reverse_distance_enters_and_returns_through_smooth_progress() {
    let mut reverse = ReverseStop::new();
    reverse.reset(trip_distance(1.0));

    reverse.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(1),
        ReverseStopEntryPolicy::Allow,
    );
    assert!(reverse.active());
    assert_eq!(reverse.setpoint(), AngleDegrees::ZERO);

    let mut distance = 0.97;
    for step in 1..=25 {
        distance -= 0.01;
        reverse.update(
            trip_distance(distance),
            Rpm::from_revolutions_per_minute(-500.0),
            reverse.setpoint(),
            TimestampTicks::from_ticks(step),
            ReverseStopEntryPolicy::Allow,
        );
    }
    assert!(reverse.setpoint().is_positive());
}

#[test]
fn disabled_reverse_stop_tracks_distance_without_activating() {
    let mut reverse = ReverseStop::new();
    reverse.update(
        trip_distance(-1.0),
        Rpm::from_revolutions_per_minute(-1_000.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(0),
        ReverseStopEntryPolicy::Block,
    );

    assert!(!reverse.active());
}

fn started_reverse_stop() -> ReverseStop {
    let mut reverse = ReverseStop::new();
    reverse.reset(trip_distance(1.0));
    reverse.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(1),
        ReverseStopEntryPolicy::Allow,
    );
    reverse
}

#[test]
fn reverse_stop_entry_preserves_the_timer_epoch_like_refloat() {
    let reverse = started_reverse_stop();

    // Refloat does not refresh the zero-valued timer when the maneuver starts.
    assert!(!reverse.should_stop(TimestampTicks::from_ticks(30_000)));
    assert!(reverse.should_stop(TimestampTicks::from_ticks(30_001)));
}

#[test]
fn reverse_stop_refreshes_timer_below_but_not_at_angle_threshold() {
    let mut at_threshold = started_reverse_stop();
    at_threshold.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::from_degrees(8.5),
        TimestampTicks::from_ticks(10_001),
        ReverseStopEntryPolicy::Allow,
    );
    assert!(at_threshold.should_stop(TimestampTicks::from_ticks(30_002)));

    let mut below_threshold = started_reverse_stop();
    below_threshold.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::from_degrees(8.4),
        TimestampTicks::from_ticks(10_001),
        ReverseStopEntryPolicy::Allow,
    );
    assert!(!below_threshold.should_stop(TimestampTicks::from_ticks(30_002)));
}

#[test]
fn reverse_stop_accepts_the_exact_entry_erpm_boundary() {
    let mut reverse = ReverseStop::new();
    reverse.reset(trip_distance(1.0));
    reverse.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-200.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(1),
        ReverseStopEntryPolicy::Allow,
    );

    assert!(reverse.active());
}

#[test]
fn exact_target_distance_completes_without_overshoot_or_timeout() {
    let mut reverse = started_reverse_stop();
    let exact_target_distance = trip_distance(0.72);
    let mut now = TimestampTicks::from_ticks(1);

    for tick in 2_u32..=10_000 {
        now = TimestampTicks::from_ticks(tick);
        reverse.update(
            exact_target_distance,
            Rpm::from_revolutions_per_minute(-500.0),
            AngleDegrees::ZERO,
            now,
            ReverseStopEntryPolicy::Allow,
        );
    }

    assert!(
        reverse.should_stop(now),
        "exactly reaching the configured reverse distance must complete without requiring overshoot",
    );
}

#[test]
fn reverse_stop_uses_strict_legacy_physical_pitch_thresholds() {
    let mut immediate = started_reverse_stop();
    assert!(!immediate.should_stop_for_pitch(
        TimestampTicks::from_ticks(2),
        AngleDegrees::from_degrees(18.0),
    ));
    assert!(immediate.should_stop_for_pitch(
        TimestampTicks::from_ticks(2),
        AngleDegrees::from_degrees(18.1),
    ));

    let mut fast = started_reverse_stop();
    assert!(!fast.should_stop_for_pitch(
        TimestampTicks::from_ticks(10_001),
        AngleDegrees::from_degrees(11.0),
    ));
    assert!(fast.should_stop_for_pitch(
        TimestampTicks::from_ticks(10_002),
        AngleDegrees::from_degrees(11.0),
    ));

    let mut slow = started_reverse_stop();
    assert!(!slow.should_stop_for_pitch(
        TimestampTicks::from_ticks(20_001),
        AngleDegrees::from_degrees(6.0),
    ));
    assert!(slow.should_stop_for_pitch(
        TimestampTicks::from_ticks(20_002),
        AngleDegrees::from_degrees(6.0),
    ));

    let mut refreshed = started_reverse_stop();
    assert!(!refreshed.should_stop_for_pitch(
        TimestampTicks::from_ticks(10_000),
        AngleDegrees::from_degrees(4.9),
    ));
    assert!(!refreshed.should_stop_for_pitch(
        TimestampTicks::from_ticks(30_000),
        AngleDegrees::from_degrees(6.0),
    ));
    assert!(refreshed.should_stop_for_pitch(
        TimestampTicks::from_ticks(30_001),
        AngleDegrees::from_degrees(6.0),
    ));
}

#[test]
fn returning_reverse_stop_does_not_apply_physical_pitch_cutoff() {
    let mut reverse = started_reverse_stop();

    assert!(!reverse.should_stop_for_pitch(
        TimestampTicks::from_ticks(9_001),
        AngleDegrees::from_degrees(11.0),
    ));
    reverse.update(
        trip_distance(1.0),
        Rpm::from_revolutions_per_minute(-500.0),
        reverse.setpoint(),
        TimestampTicks::from_ticks(9_002),
        ReverseStopEntryPolicy::Allow,
    );

    assert!(!reverse.is_stopping());
    assert!(!reverse.should_stop_for_pitch(
        TimestampTicks::from_ticks(10_002),
        AngleDegrees::from_degrees(11.0),
    ));
}
