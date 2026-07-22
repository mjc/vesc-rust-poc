#![cfg(feature = "test-support")]
//! Integration coverage for owned GNSS snapshots.

use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn gnss_snapshot_copies_firmware_owned_record() {
    let firmware = FirmwareTest::new();
    let snapshot = firmware.gnss().snapshot().unwrap();
    assert_eq!(snapshot.latitude().latitude().as_degrees(), 40.0);
    assert_eq!(snapshot.longitude().longitude().as_degrees(), -105.0);
    assert_eq!(snapshot.altitude().altitude().as_meters(), 1600.0);
    assert_eq!(snapshot.speed().speed().as_meters_per_second(), 3.5);
}
