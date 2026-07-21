#![cfg(feature = "test-support")]

//! Integration tests for typed custom-EEPROM byte images.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{CustomEepromAddress, EepromWord};

#[test]
fn byte_image_round_trips_complete_and_partial_words() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let expected = [1, 2, 3, 4, 5, 6];

    assert!(eeprom.write_bytes(&expected));
    assert_eq!(
        eeprom.read(CustomEepromAddress::from_index(1).expect("one fits")),
        Some(EepromWord::from_ne_bytes([5, 6, 0, 0]))
    );

    let mut actual = [0; 6];
    assert!(eeprom.read_bytes(&mut actual));
    assert_eq!(actual, expected);
}

#[test]
fn byte_image_operations_report_missing_reads_and_failed_writes() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let mut bytes = [0; 4];
    assert!(!eeprom.read_bytes(&mut bytes));

    let failed = CustomEepromAddress::from_index(1).expect("one fits");
    firmware.fail_eeprom_write(failed);
    assert!(!eeprom.write_bytes(&[1, 2, 3, 4, 5]));
    assert_eq!(eeprom.read(failed), None);
}
