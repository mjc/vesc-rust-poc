use super::*;

#[test]
fn tracker_layout_covers_the_valid_config_range_without_unbounded_storage() {
    assert_eq!(core::mem::size_of::<MotorKinematicsTracker>(), 148);
}

#[test]
fn acceleration_is_normalized_to_elapsed_seconds() {
    let mut tracker = MotorKinematicsTracker::default();
    tracker.configure(SampleRate::from_hertz(500.0));

    for step in 1_u8..=27 {
        tracker.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(0.002),
        );
    }

    assert_f32_eq!(tracker.average().as_erpm_per_second(), 5_000.0);
}

#[test]
fn initial_acceleration_window_starts_zero_filled_like_refloat() {
    let mut tracker = MotorKinematicsTracker::default();

    tracker.record(
        Rpm::from_revolutions_per_minute(10.0),
        VescSeconds::from_seconds(0.002),
    );

    assert_f32_eq!(tracker.average().as_erpm_per_second(), 5_000.0 / 27.0,);
}

#[test]
fn acceleration_window_tracks_refloat_eight_hertz_cutoff() {
    let mut at_500_hz = MotorKinematicsTracker::default();
    at_500_hz.configure(SampleRate::from_hertz(500.0));
    let mut at_832_hz = MotorKinematicsTracker::default();
    at_832_hz.configure(SampleRate::from_hertz(832.0));

    for step in 1_u8..=27 {
        at_500_hz.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(0.002),
        );
    }
    for step in 1_u8..=46 {
        at_832_hz.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(1.0 / 832.0),
        );
    }

    assert_f32_eq!(at_500_hz.average().as_erpm_per_second(), 5_000.0);
    assert_f32_eq!(at_832_hz.average().as_erpm_per_second(), 8_320.0);
}

#[test]
fn acceleration_reconfiguration_preserves_the_live_average_like_refloat() {
    let mut tracker = MotorKinematicsTracker::default();
    tracker.configure(SampleRate::from_hertz(500.0));
    for step in 1_u8..=27 {
        tracker.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(0.002),
        );
    }
    let before = tracker.average();

    tracker.configure(SampleRate::from_hertz(550.0));

    assert_eq!(tracker.average(), before);

    let mut motor_erpm = Rpm::from_revolutions_per_minute(270.0);
    for _ in 0..MAX_WINDOW {
        if tracker.pending_window.is_none() {
            break;
        }
        motor_erpm = motor_erpm + Rpm::from_revolutions_per_minute(5_000.0 / 550.0);
        tracker.record(motor_erpm, VescSeconds::from_seconds(1.0 / 550.0));
    }

    assert_eq!(tracker.pending_window, None);
    assert!(
        (tracker.average().as_erpm_per_second() - 5_000.0).abs() < 0.01,
        "average={:?}",
        tracker.average()
    );
}

#[test]
fn acceleration_reconfiguration_preserves_the_live_average_when_window_shrinks() {
    let mut tracker = MotorKinematicsTracker::default();
    tracker.configure(SampleRate::from_hertz(450.0));
    let mut motor_erpm = Rpm::ZERO;

    for _ in 0..=27 {
        motor_erpm = motor_erpm + Rpm::from_revolutions_per_minute(10.0);
        tracker.record(motor_erpm, VescSeconds::from_seconds(0.002));
    }

    assert!(
        (tracker.average().as_erpm_per_second() - 5_000.0).abs() < 0.01,
        "average={:?}",
        tracker.average()
    );
}

#[test]
fn absolute_erpm_uses_refloat_ten_hertz_ema() {
    let mut tracker = MotorKinematicsTracker::default();
    tracker.configure(SampleRate::from_hertz(500.0));

    tracker.record(
        Rpm::from_revolutions_per_minute(-1_000.0),
        VescSeconds::from_seconds(0.002),
    );

    assert_f32_eq!(
        tracker.smoothed_abs_erpm().as_revolutions_per_minute(),
        117.768_03,
    );
}
