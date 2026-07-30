use super::reverse_stop::ReverseStop;
use vescpkg_rs::prelude::{AngleDegrees, Rpm, TimestampTicks, VescSeconds};

#[test]
fn reverse_distance_enters_and_returns_through_smooth_progress() {
    let mut reverse = ReverseStop::new();
    let elapsed = VescSeconds::from_seconds(1.0 / 832.0);
    reverse.reset(1.0);

    reverse.update(
        0.97,
        Rpm::from_revolutions_per_minute(-500.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(1),
        true,
        elapsed,
    );
    assert!(reverse.active());
    assert_eq!(reverse.setpoint(), AngleDegrees::ZERO);

    let mut distance = 0.97;
    for step in 1..=25 {
        distance -= 0.01;
        reverse.update(
            distance,
            Rpm::from_revolutions_per_minute(-500.0),
            reverse.setpoint(),
            TimestampTicks::from_ticks(step),
            true,
            elapsed,
        );
    }
    assert!(reverse.setpoint().is_positive());
}

#[test]
fn disabled_reverse_stop_tracks_distance_without_activating() {
    let mut reverse = ReverseStop::new();
    reverse.update(
        -1.0,
        Rpm::from_revolutions_per_minute(-1_000.0),
        AngleDegrees::ZERO,
        TimestampTicks::from_ticks(0),
        false,
        VescSeconds::from_seconds(1.0 / 832.0),
    );

    assert!(!reverse.active());
}
