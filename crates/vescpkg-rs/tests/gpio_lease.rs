#![cfg(feature = "test-support")]
//! Integration coverage for leased GPIO access.

use vescpkg_rs::{DigitalOutputLevel, DigitalPin, GpioError, GpioMode};

#[test]
fn gpio_leases_are_exclusive_and_release_on_drop() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let mut ppm = gpio.acquire_digital(DigitalPin::PPM).expect("first lease");

    assert_eq!(gpio.acquire_digital(DigitalPin::PPM), Err(GpioError::Busy));
    ppm.set_mode(GpioMode::Output).expect("output mode");
    ppm.write(DigitalOutputLevel::High).expect("write output");
    drop(ppm);

    assert!(gpio.acquire_digital(DigitalPin::PPM).is_ok());
}
