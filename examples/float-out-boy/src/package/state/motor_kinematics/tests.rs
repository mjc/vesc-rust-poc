use super::{MotorKinematicsTracker, WINDOW_U8};
use vescpkg_rs::prelude::Rpm;

#[test]
fn tracker_layout_fits_the_existing_package_state_slot() {
    assert_eq!(core::mem::size_of::<MotorKinematicsTracker>(), 176);
}

#[test]
fn record_matches_float_out_boy_rolling_erpm_delta_average() {
    let mut tracker = MotorKinematicsTracker::default();

    for step in 1..=WINDOW_U8 {
        tracker.record(Rpm::from_revolutions_per_minute(f32::from(step) * 10.0));
    }

    assert_f32_eq!(tracker.average().as_revolutions_per_minute(), 10.0);

    tracker.record(Rpm::from_revolutions_per_minute(410.0));

    assert_f32_eq!(tracker.average().as_revolutions_per_minute(), 10.0);

    tracker.record(Rpm::from_revolutions_per_minute(450.0));

    // Float Out Boy replaces the oldest 10 ERPM sample with the current 40 ERPM sample:
    // `10 + (40 - 10) / ACCEL_ARRAY_SIZE`.
    assert_f32_eq!(tracker.average().as_revolutions_per_minute(), 10.75);
}

#[test]
fn record_matches_float_out_boy_absolute_erpm_smoothing() {
    let mut tracker = MotorKinematicsTracker::default();

    tracker.record(Rpm::from_revolutions_per_minute(-1_000.0));

    assert_eq!(
        tracker.smoothed_abs_erpm(),
        Rpm::from_revolutions_per_minute(100.0)
    );
}
