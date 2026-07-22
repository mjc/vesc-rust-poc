#![cfg(feature = "test-support")]
//! Integration coverage for leased GPIO access.

use vescpkg_rs::{
    DigitalOutputLevel, DigitalPin, GpioError, GpioMode, PackageRuntimeState, PackageStateStore,
};

struct PackageState;

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

#[test]
fn gpio_leases_are_exclusive_and_release_on_drop() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let ppm = gpio.acquire_digital(DigitalPin::PPM).expect("first lease");

    assert!(matches!(
        gpio.acquire_digital(DigitalPin::PPM),
        Err(GpioError::Busy)
    ));
    ppm.set_mode(GpioMode::Output).expect("output mode");
    ppm.write(DigitalOutputLevel::High).expect("write output");
    drop(ppm);

    assert!(gpio.acquire_digital(DigitalPin::PPM).is_ok());
}

#[test]
fn gpio_leases_reject_wrong_mode_and_cover_analog_ownership() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let ppm = gpio
        .acquire_digital(DigitalPin::PPM)
        .expect("digital lease");

    assert_eq!(
        ppm.write(DigitalOutputLevel::Low),
        Err(GpioError::WrongMode)
    );
    ppm.set_mode(GpioMode::InputPullUp).expect("input mode");
    assert_eq!(ppm.read(), Ok(false));
    assert_eq!(
        ppm.write(DigitalOutputLevel::High),
        Err(GpioError::WrongMode)
    );
    drop(ppm);

    let adc = gpio
        .acquire_analog(vescpkg_rs::AnalogPin::ADC1)
        .expect("analog lease");
    assert_eq!(adc.read(), Err(GpioError::WrongMode));
    adc.set_mode(GpioMode::Analog).expect("analog mode");
    assert_eq!(adc.read().expect("analog read").voltage().as_volts(), 1.2);
}

#[test]
fn gpio_leases_cover_pull_and_open_drain_modes() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let modes = [
        GpioMode::Input,
        GpioMode::InputPullDown,
        GpioMode::Output,
        GpioMode::OpenDrain,
        GpioMode::OpenDrainPullUp,
        GpioMode::OpenDrainPullDown,
    ];

    for mode in modes {
        let pin = gpio
            .acquire_digital(DigitalPin::HW_1)
            .expect("mode test lease");
        pin.set_mode(mode).expect("pinned mode");
        if matches!(mode, GpioMode::Input | GpioMode::InputPullDown) {
            assert_eq!(pin.read(), Ok(false));
        } else {
            pin.write(DigitalOutputLevel::High)
                .expect("output mode write");
        }
    }
}

#[test]
fn package_stop_invalidates_retained_gpio_leases() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let retained = gpio
        .acquire_digital(DigitalPin::HW_2)
        .expect("retained lease");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState)
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    let replacement = gpio
        .acquire_digital(DigitalPin::HW_2)
        .expect("stop invalidated the retained lease");
    drop(replacement);
    drop(retained);
}

#[test]
fn every_pinned_digital_pin_has_an_exclusive_safe_lease() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gpio = firmware.gpio();
    let pins = [
        DigitalPin::COMM_RX,
        DigitalPin::COMM_TX,
        DigitalPin::SWDIO,
        DigitalPin::SWCLK,
        DigitalPin::HALL1,
        DigitalPin::HALL2,
        DigitalPin::HALL3,
        DigitalPin::HALL4,
        DigitalPin::HALL5,
        DigitalPin::HALL6,
        DigitalPin::PPM,
        DigitalPin::HW_1,
        DigitalPin::HW_2,
    ];

    for pin in pins {
        let lease = gpio.acquire_digital(pin).expect("pin has a lease");
        lease.set_mode(GpioMode::Input).expect("input mode");
        assert_eq!(lease.read(), Ok(false));
    }
}
