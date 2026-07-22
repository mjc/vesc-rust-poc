#![cfg(feature = "test-support")]
//! Integration coverage for encoder callback ownership.

use core::ffi::CStr;
use vescpkg_rs::{AngleDegrees, EncoderHandler, test_support::FirmwareTest};

struct Handler;

impl EncoderHandler for Handler {
    fn read_degrees() -> AngleDegrees { AngleDegrees::from_degrees(12.0) }
    fn has_fault() -> bool { false }
    fn info() -> &'static CStr { c"SDK encoder" }
}

#[test]
fn encoder_registration_is_exclusive_and_clears_on_drop() {
    let firmware = FirmwareTest::new();
    let encoder = firmware.encoder();
    let registration = encoder.register::<Handler>().unwrap();
    assert!(encoder.register::<Handler>().is_err());
    drop(registration);
    assert!(encoder.register::<Handler>().is_ok());
}
