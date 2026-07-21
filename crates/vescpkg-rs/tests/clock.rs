#![cfg(feature = "test-support")]

//! Integration tests for the distinct firmware clock domains.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{TimerInstant, TimestampTicks, VescSeconds};

#[test]
fn firmware_clock_exposes_ticks_uptime_and_timestamp_age_separately() {
    let firmware = FirmwareTest::new();
    firmware.set_clock_ticks(12_500);
    let clock = firmware.clock();

    assert_eq!(clock.now(), TimestampTicks::from_ticks(12_500));
    assert_eq!(clock.uptime(), VescSeconds::from_seconds(1.25));
    assert_eq!(
        clock.age(TimestampTicks::from_ticks(2_500)),
        VescSeconds::from_seconds(1.0)
    );
}

#[test]
fn firmware_clock_keeps_high_resolution_timer_instants_distinct() {
    let firmware = FirmwareTest::new();
    firmware.set_timer_ticks(1_000_000);
    let clock = firmware.clock();
    let earlier = TimerInstant::from_raw(500_000);

    assert_eq!(clock.timer_now(), TimerInstant::from_raw(1_000_000));
    assert_eq!(
        clock.timer_elapsed_since(earlier),
        VescSeconds::from_seconds(0.5)
    );
}

#[test]
fn firmware_clock_ages_roll_over_without_mixing_clock_domains() {
    let firmware = FirmwareTest::new();
    firmware.set_clock_ticks(5);
    firmware.set_timer_ticks(5);
    let clock = firmware.clock();

    assert_eq!(
        clock.age(TimestampTicks::from_ticks(u32::MAX - 4)),
        VescSeconds::from_seconds(0.001)
    );
    assert_eq!(
        clock.timer_elapsed_since(TimerInstant::from_raw(u32::MAX - 4)),
        VescSeconds::from_seconds(0.00001)
    );
}
