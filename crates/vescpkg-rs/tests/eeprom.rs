#![cfg(feature = "test-support")]

//! Integration tests for typed custom-EEPROM byte images.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{CustomEepromAddress, EepromError, EepromWord};

#[test]
fn byte_image_round_trips_complete_and_partial_words() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let expected = [1, 2, 3, 4, 5, 6];

    assert!(eeprom.write_bytes(&expected).is_ok());
    assert_eq!(
        eeprom.read(CustomEepromAddress::from_index(1).expect("one fits")),
        Some(EepromWord::from_ne_bytes([5, 6, 0, 0]))
    );

    let mut actual = [0; 6];
    assert!(eeprom.read_bytes(&mut actual).is_ok());
    assert_eq!(actual, expected);
}

#[test]
fn byte_image_operations_report_missing_reads_and_failed_writes() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let mut bytes = [0; 4];
    assert_eq!(eeprom.read_bytes(&mut bytes), Err(EepromError::Missing));

    let failed = CustomEepromAddress::from_index(1).expect("one fits");
    firmware.fail_eeprom_write(failed);
    assert_eq!(
        eeprom.write_bytes(&[1, 2, 3, 4, 5]),
        Err(EepromError::FirmwareRejected)
    );
    assert_eq!(eeprom.read(failed), None);
}

#[test]
fn byte_image_read_reports_interrupted_image_without_erasing_prefix() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let first = CustomEepromAddress::from_index(100).expect("address fits");
    assert!(
        eeprom
            .write(first, EepromWord::from_ne_bytes([1, 2, 3, 4]))
            .is_ok()
    );

    let mut bytes = [0xaa; 8];
    assert_eq!(
        eeprom.read_bytes_at(first, &mut bytes),
        Err(EepromError::Missing)
    );
    assert_eq!(&bytes[..4], &[1, 2, 3, 4]);
    assert_eq!(&bytes[4..], &[0xaa; 4]);
}

#[test]
fn eeprom_words_round_trip_supported_scalar_codecs() {
    assert_eq!(EepromWord::from_u32(0xfeed_beef).to_u32(), 0xfeed_beef);
    assert_eq!(EepromWord::from_i32(-42).to_i32(), -42);
    assert_eq!(EepromWord::from_f32(12.5).to_f32(), 12.5);
}

#[test]
fn byte_images_can_start_at_an_explicit_word_address() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let start = CustomEepromAddress::from_index(3).expect("address fits");

    assert!(eeprom.write_bytes_at(start, &[9, 8, 7, 6, 5]).is_ok());
    let mut bytes = [0; 5];
    assert!(eeprom.read_bytes_at(start, &mut bytes).is_ok());
    assert_eq!(bytes, [9, 8, 7, 6, 5]);
}

#[test]
fn eeprom_address_rejects_indices_outside_the_signed_abi_range() {
    assert!(CustomEepromAddress::from_index(usize::MAX).is_none());
}
