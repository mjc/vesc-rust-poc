#![cfg(feature = "test-support")]
//! Integration coverage for encoder callback ownership.

use core::ffi::CStr;
use vescpkg_rs::{
    AngleDegrees, EncoderHandler, EncoderRegistration, PackageRuntimeState, PackageStateStore,
    test_support::FirmwareTest,
};

struct Handler;

impl EncoderHandler for Handler {
    fn read_degrees() -> AngleDegrees {
        AngleDegrees::from_degrees(12.0)
    }
    fn has_fault() -> bool {
        false
    }
    fn info() -> &'static CStr {
        c"SDK encoder"
    }
}

struct PackageState {
    _registration: Option<EncoderRegistration<Handler>>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
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

#[test]
fn encoder_registration_reports_absent_optional_slots() {
    let firmware = FirmwareTest::new();
    firmware.set_encoder_available(false);

    assert!(matches!(
        firmware.encoder().register::<Handler>(),
        Err(vescpkg_rs::EncoderError::Unavailable)
    ));
}

#[test]
fn package_stop_releases_encoder_state_before_next_registration() {
    let firmware = FirmwareTest::new();
    let registration = firmware
        .encoder()
        .register::<Handler>()
        .expect("encoder callback");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _registration: Some(registration),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    firmware
        .encoder()
        .register::<Handler>()
        .expect("stop released encoder callback");
}
