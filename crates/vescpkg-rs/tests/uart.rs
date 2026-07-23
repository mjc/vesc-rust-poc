#![cfg(feature = "test-support")]
//! Integration coverage for the exclusive UART capability.

use vescpkg_rs::{
    BaudRate, PackageRuntimeState, PackageStateStore, UartDuplexMode, UartLease,
    test_support::FirmwareTest,
};

struct PackageState {
    _lease: Option<UartLease>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

#[test]
fn uart_duplex_mode_has_explicit_abi_mapping() {
    assert!(!UartDuplexMode::FullDuplex.is_half_duplex());
    assert!(UartDuplexMode::HalfDuplex.is_half_duplex());
}

#[test]
fn uart_lease_forwards_checked_io_and_releases_ownership() {
    let firmware = FirmwareTest::new();
    let uart = firmware.uart();
    let baud = BaudRate::try_new(115_200).unwrap();
    let lease = uart.open(baud, UartDuplexMode::FullDuplex).unwrap();
    assert_eq!(lease.write(b"abc").unwrap(), 3);
    assert_eq!(lease.read(), Ok(Some(b'A')));
    assert!(uart.open(baud, UartDuplexMode::HalfDuplex).is_err());
    drop(lease);
    assert!(uart.open(baud, UartDuplexMode::HalfDuplex).is_ok());
}

#[test]
fn uart_reports_absent_optional_slots() {
    let firmware = FirmwareTest::new();
    firmware.set_uart_available(false);
    let baud = BaudRate::try_new(115_200).unwrap();

    assert!(matches!(
        firmware.uart().open(baud, UartDuplexMode::FullDuplex),
        Err(vescpkg_rs::UartError::Unavailable)
    ));
}

#[test]
fn package_stop_releases_uart_state_before_next_open() {
    let firmware = FirmwareTest::new();
    let baud = BaudRate::try_new(115_200).unwrap();
    let lease = firmware
        .uart()
        .open(baud, UartDuplexMode::FullDuplex)
        .expect("UART lease");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _lease: Some(lease),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    firmware
        .uart()
        .open(baud, UartDuplexMode::HalfDuplex)
        .expect("stop released UART lease");
}
