#![cfg(feature = "test-support")]
//! Integration coverage for the exclusive UART capability.

use vescpkg_rs::{BaudRate, test_support::FirmwareTest};

#[test]
fn uart_lease_forwards_checked_io_and_releases_ownership() {
    let firmware = FirmwareTest::new();
    let uart = firmware.uart();
    let baud = BaudRate::try_new(115_200).unwrap();
    let lease = uart.open(baud, false).unwrap();
    assert_eq!(lease.write(b"abc").unwrap(), 3);
    assert_eq!(lease.read(), Ok(Some(b'A')));
    assert!(uart.open(baud, true).is_err());
    drop(lease);
    assert!(uart.open(baud, true).is_ok());
}
