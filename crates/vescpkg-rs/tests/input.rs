#![cfg(feature = "test-support")]

//! Integration tests for optional controller-input firmware slots.

use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn missing_controller_input_slots_fall_back_to_neutral_stale_samples() {
    let firmware = FirmwareTest::new();
    firmware.set_ppm_supported(false);
    firmware.set_remote_supported(false);

    let (ppm, ppm_age) = firmware.input().ppm();
    assert_eq!(ppm.ratio().as_ratio().to_bits(), 0.0_f32.to_bits());
    assert!(ppm_age.duration().as_seconds().is_infinite());

    let remote = firmware.input().remote();
    assert_eq!(
        remote.joystick_y().ratio().as_ratio().to_bits(),
        0.0_f32.to_bits()
    );
    assert!(remote.age().duration().as_seconds().is_infinite());
}
