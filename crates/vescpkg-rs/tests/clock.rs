#![cfg(feature = "test-support")]

//! Integration tests for the distinct firmware clock domains.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{TimestampTicks, VescSeconds};

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
