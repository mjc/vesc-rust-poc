use super::reverse_stop::{ReverseStop, ReverseStopEntryPolicy};
use vescpkg_rs::prelude::{
    AngleDegrees, Distance, Rpm, SignedTripDistance, TimestampTicks, VescSeconds,
};

fn trip_distance(meters: f32) -> SignedTripDistance {
    SignedTripDistance::new(Distance::from_meters(meters))
}

#[test]
fn reverse_distance_enters_and_returns_through_smooth_progress() {
    let mut reverse = ReverseStop::new();
    let elapsed = VescSeconds::from_seconds(1.0 / 832.0);
    reverse.reset(trip_distance(1.0));

    reverse.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(1),
        ReverseStopEntryPolicy::Allow,
        elapsed,
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
            elapsed,
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
        VescSeconds::from_seconds(1.0 / 832.0),
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
        VescSeconds::from_seconds(1.0 / 832.0),
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
    let elapsed = VescSeconds::from_seconds(1.0 / 832.0);
    let mut at_threshold = started_reverse_stop();
    at_threshold.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::from_degrees(8.5),
        TimestampTicks::from_ticks(10_001),
        ReverseStopEntryPolicy::Allow,
        elapsed,
    );
    assert!(at_threshold.should_stop(TimestampTicks::from_ticks(30_002)));

    let mut below_threshold = started_reverse_stop();
    below_threshold.update(
        trip_distance(0.97),
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::from_degrees(8.4),
        TimestampTicks::from_ticks(10_001),
        ReverseStopEntryPolicy::Allow,
        elapsed,
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
        VescSeconds::from_seconds(1.0 / 832.0),
    );

    assert!(reverse.active());
}
