//! Integration tests for typed custom-EEPROM byte images.
#![cfg(feature = "test-support")]

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{CustomEepromAddress, EepromError, EepromWord, EepromWordOffset};

#[test]
fn byte_image_round_trips_complete_and_partial_words() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let expected = [1, 2, 3, 4, 5, 6];

    let effects = firmware.effects();
    assert!(eeprom.write_bytes(effects, &expected).is_ok());
    assert_eq!(
        eeprom.read(
            effects,
            CustomEepromAddress::from_index(1).expect("one fits"),
        ),
        Some(EepromWord::from_ne_bytes([5, 6, 0, 0]))
    );

    let mut actual = [0; 6];
    assert!(eeprom.read_bytes(effects, &mut actual).is_ok());
    assert_eq!(actual, expected);
}

#[test]
fn byte_image_operations_report_missing_reads_and_failed_writes() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let mut bytes = [0; 4];
    let effects = firmware.effects();
    assert_eq!(
        eeprom.read_bytes(effects, &mut bytes),
        Err(EepromError::Missing)
    );

    let failed = CustomEepromAddress::from_index(1).expect("one fits");
    firmware.fail_eeprom_write(failed);
    assert_eq!(
        eeprom.write_bytes(effects, &[1, 2, 3, 4, 5]),
        Err(EepromError::FirmwareRejected)
    );
    assert_eq!(eeprom.read(effects, failed), None);
}

#[test]
fn byte_image_read_reports_interrupted_image_without_erasing_prefix() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let first = CustomEepromAddress::from_index(100).expect("address fits");
    let effects = firmware.effects();
    assert!(
        eeprom
            .write(effects, first, EepromWord::from_ne_bytes([1, 2, 3, 4]))
            .is_ok()
    );

    let mut bytes = [0xaa; 8];
    assert_eq!(
        eeprom.read_bytes_at(effects, first, &mut bytes),
        Err(EepromError::Missing)
    );
    assert_eq!(&bytes[..4], &[1, 2, 3, 4]);
    assert_eq!(&bytes[4..], &[0xaa; 4]);
}

#[test]
fn eeprom_words_round_trip_supported_scalar_codecs() {
    assert_eq!(EepromWord::from_u32(0xfeed_beef).to_u32(), 0xfeed_beef);
    assert_eq!(EepromWord::from_i32(-42).to_i32(), -42);
    assert_eq!(
        EepromWord::from_f32(12.5).to_f32().to_bits(),
        12.5_f32.to_bits()
    );
}

#[test]
fn byte_images_can_start_at_an_explicit_word_address() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let start = CustomEepromAddress::from_index(3).expect("address fits");

    let effects = firmware.effects();
    assert!(
        eeprom
            .write_bytes_at(effects, start, &[9, 8, 7, 6, 5])
            .is_ok()
    );
    let mut bytes = [0; 5];
    assert!(eeprom.read_bytes_at(effects, start, &mut bytes).is_ok());
    assert_eq!(bytes, [9, 8, 7, 6, 5]);
}

#[test]
fn typed_offsets_round_trip_a_signature_prefixed_image() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let signature = EepromWordOffset::from_index(7);
    let image = [0xca, 0xfe, 0xba, 0xbe, 1, 2, 3, 4, 5];

    let effects = firmware.effects();
    assert!(eeprom.write_image_at(effects, signature, &image).is_ok());
    assert_eq!(eeprom.read_image_at::<9>(effects, signature), Ok(image));
}

#[test]
fn fixed_size_image_reads_are_owned_and_report_missing_words() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let start = EepromWordOffset::from_index(3);
    let image = [9, 8, 7, 6, 5];

    let effects = firmware.effects();
    assert!(eeprom.write_image_at(effects, start, &image).is_ok());
    assert_eq!(eeprom.read_image_at::<5>(effects, start), Ok(image));
    assert_eq!(eeprom.read_image::<5>(effects), Err(EepromError::Missing));
}

#[test]
fn typed_offset_conversion_rejects_an_abi_overflow() {
    let firmware = FirmwareTest::new();
    let eeprom = firmware.eeprom();
    let offset = EepromWordOffset::from_index(i32::MAX as usize + 1);
    let mut image = [0; 4];

    let effects = firmware.effects();
    assert_eq!(
        eeprom.read_image_at::<4>(effects, offset),
        Err(EepromError::AddressOverflow)
    );
    assert_eq!(
        eeprom.read_bytes_at_offset(effects, offset, &mut image),
        Err(EepromError::AddressOverflow)
    );
}

#[test]
fn eeprom_address_rejects_indices_outside_the_signed_abi_range() {
    assert!(CustomEepromAddress::from_index(usize::MAX).is_none());
}
