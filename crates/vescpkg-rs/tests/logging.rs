#![cfg(feature = "test-support")]
//! Integration coverage for bounded firmware logging.

use core::fmt::Write;

use vescpkg_rs::test_support::with_firmware_effects;
use vescpkg_rs::{FirmwareLog, LogError};

#[test]
fn logging_formats_data_without_allocating_and_reports_truncation() {
    let mut log = FirmwareLog::<8>::new();
    write!(&mut log, "rpm={}", 1200).expect("formatting fits the buffer");

    assert_eq!(log.as_bytes(), b"rpm=120");
    assert!(log.is_truncated());
    assert_eq!(
        with_firmware_effects(|effects| log.flush(effects)),
        Err(LogError::Truncated)
    );
}

#[test]
fn logging_flushes_a_complete_message_through_the_firmware_slot() {
    let mut log = FirmwareLog::<16>::new();
    log.write_bytes(b"duty=0.25");

    assert_eq!(with_firmware_effects(|effects| log.flush(effects)), Ok(9));
}

#[test]
fn logging_rejects_c_strings_with_embedded_nuls() {
    let mut log = FirmwareLog::<16>::new();
    log.write_bytes(b"bad\0value");

    assert_eq!(
        with_firmware_effects(|effects| log.flush(effects)),
        Err(LogError::InteriorNul)
    );
}

#[test]
fn logging_writes_every_u8_as_allocation_free_decimal() {
    for (value, expected) in [
        (0, b"0".as_slice()),
        (9, b"9".as_slice()),
        (10, b"10".as_slice()),
        (99, b"99".as_slice()),
        (100, b"100".as_slice()),
        (255, b"255".as_slice()),
    ] {
        let mut log = FirmwareLog::<4>::new();
        log.write_u8_decimal(value);
        assert_eq!(log.as_bytes(), expected);
        assert!(!log.is_truncated());
    }
}
