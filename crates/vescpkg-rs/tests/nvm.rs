#![cfg(feature = "test-support")]

//! Integration tests for the typed byte-addressed NVM capability.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{NvmError, NvmOffset};

#[test]
fn nvm_round_trips_a_checked_byte_range_and_wipes_it() {
    let firmware = FirmwareTest::new();
    let nvm = firmware.nvm();
    let offset = NvmOffset::new(12);
    let mut expected = [1, 2, 3, 4];

    nvm.write(offset, &mut expected)
        .expect("NVM write succeeds");

    let mut actual = [0; 4];
    nvm.read(offset, &mut actual).expect("NVM read succeeds");
    assert_eq!(actual, expected);

    nvm.wipe().expect("NVM wipe succeeds");
    actual.fill(0xff);
    nvm.read(offset, &mut actual).expect("NVM read after wipe");
    assert_eq!(actual, [0; 4]);
}

#[test]
fn nvm_rejects_ranges_that_overflow_the_firmware_offset() {
    let firmware = FirmwareTest::new();
    let mut bytes = [0; 2];

    assert_eq!(
        firmware.nvm().read(NvmOffset::new(u32::MAX), &mut bytes),
        Err(NvmError::InvalidRange)
    );
}

#[test]
fn nvm_reports_firmware_operation_failures() {
    let firmware = FirmwareTest::new();
    firmware.fail_nvm_operations();
    let offset = NvmOffset::new(0);
    let mut bytes = [0; 1];

    assert_eq!(
        firmware.nvm().read(offset, &mut bytes),
        Err(NvmError::FirmwareFailure)
    );
    assert_eq!(
        firmware.nvm().write(offset, &mut bytes),
        Err(NvmError::FirmwareFailure)
    );
    assert_eq!(firmware.nvm().wipe(), Err(NvmError::FirmwareFailure));
}
