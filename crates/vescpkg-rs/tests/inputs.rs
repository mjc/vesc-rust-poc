#![cfg(feature = "test-support")]
//! Integration coverage for typed controller input and safety state.

use vescpkg_rs::{InputError, PpmInput, RemoteInputSnapshot};

#[test]
fn inputs_copy_remote_state_and_ppm_age() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let inputs = firmware.inputs();

    let remote: RemoteInputSnapshot = inputs.remote().expect("remote capability");
    assert_eq!(remote.joystick_x().ratio().as_ratio(), -0.25);
    assert_eq!(remote.joystick_y().ratio().as_ratio(), 0.75);
    assert!(remote.bluetooth_connected());
    assert!(remote.reverse());
    assert_eq!(remote.age().duration().as_seconds(), 0.2);

    let ppm = inputs.ppm().expect("PPM capability");
    assert_eq!(
        ppm.value(),
        PpmInput::new(vescpkg_rs::SignedRatio::from_ratio_const(0.5))
    );
    assert_eq!(ppm.age().duration().as_seconds(), 0.1);
}

#[test]
fn inputs_expose_output_disable_and_explicit_backup_store() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let inputs = firmware.inputs();

    assert!(!inputs.output_disabled().expect("output state capability"));
    inputs.store_backup().expect("backup store capability");
}

#[test]
fn inputs_expose_timeout_state_and_reset() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let inputs = firmware.inputs();
    let timeout = inputs.timeout();

    assert!(timeout.has_timed_out());
    assert_eq!(timeout.since_update().duration().as_seconds(), 1.5);
    inputs.reset_timeout();
}

#[test]
fn input_error_is_non_exhaustive_for_absent_capabilities() {
    assert_eq!(
        InputError::Unsupported.to_string(),
        "firmware does not expose this input capability"
    );
}
