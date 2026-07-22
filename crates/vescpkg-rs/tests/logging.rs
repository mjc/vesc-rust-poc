#![cfg(feature = "test-support")]
//! Integration coverage for bounded firmware logging.

use core::fmt::Write;

use vescpkg_rs::{FirmwareLog, LogError};

#[test]
fn logging_formats_data_without_allocating_and_reports_truncation() {
    let mut log = FirmwareLog::<8>::new();
    write!(&mut log, "rpm={}", 1200).expect("formatting fits the buffer");

    assert_eq!(log.as_bytes(), b"rpm=120");
    assert!(log.is_truncated());
    assert_eq!(log.flush(), Err(LogError::Truncated));
}
