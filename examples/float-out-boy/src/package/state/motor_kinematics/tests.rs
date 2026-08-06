use super::*;
use pin_init::PtrPinWith;
use vescpkg_rs::prelude::{Rpm, SampleRate, VescSeconds};

fn tracker(sample_rate: SampleRate) -> MotorKinematicsTracker {
    let mut tracker = MotorKinematicsTracker::default();
    MOTOR_KINEMATICS_CONFIG.configure(&mut tracker.0, sample_rate);
    tracker
}

#[test]
fn in_place_default_matches_the_regular_default() {
    let actual =
        Box::pin_with(MotorKinematicsTracker::default_in_place()).expect("infallible initializer");

    assert_eq!(
        *actual,
        tracker(crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE)
    );
}

#[test]
fn tracker_layout_covers_the_valid_config_range_without_unbounded_storage() {
    assert_eq!(core::mem::size_of::<MotorKinematicsTracker>(), 920);
}

#[test]
fn acceleration_is_normalized_to_elapsed_seconds() {
    let mut tracker = tracker(SampleRate::from_hertz(500.0));

    for step in 1_u8..=27 {
        tracker.0.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(0.002),
        );
    }

    assert_f32_eq!(tracker.0.average().as_revolutions_per_minute(), 5_000.0);
}

#[test]
fn acceleration_window_tracks_refloat_eight_hertz_cutoff() {
    let mut at_500_hz = tracker(SampleRate::from_hertz(500.0));
    let mut at_832_hz = tracker(SampleRate::from_hertz(832.0));

    for step in 1_u8..=27 {
        at_500_hz.0.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(0.002),
        );
    }
    for step in 1_u8..=46 {
        at_832_hz.0.record(
            Rpm::from_revolutions_per_minute(f32::from(step) * 10.0),
            VescSeconds::from_seconds(1.0 / 832.0),
        );
    }

    assert_f32_eq!(at_500_hz.0.average().as_revolutions_per_minute(), 5_000.0);
    assert_f32_eq!(at_832_hz.0.average().as_revolutions_per_minute(), 8_320.0);
}

#[test]
fn absolute_erpm_uses_refloat_ten_hertz_ema() {
    let mut tracker = tracker(SampleRate::from_hertz(500.0));

    tracker.0.record(
        Rpm::from_revolutions_per_minute(-1_000.0),
        VescSeconds::from_seconds(0.002),
    );

    assert_f32_eq!(
        tracker.0.smoothed_abs_erpm().as_revolutions_per_minute(),
        117.768_03,
    );
}
