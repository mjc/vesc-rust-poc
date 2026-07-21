#![cfg(feature = "test-support")]

//! Integration tests for the safe LispBM value predicates.

use vescpkg_rs::LispValue;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn lisp_values_expose_explicit_kind_predicates() {
    let _firmware = FirmwareTest::new();
    let integer = LispValue::try_from(7).expect("immediate integer fits");

    assert!(integer.is_integer());
    assert!(integer.is_number());
    assert_eq!(integer.decode_char(), None);
    assert_eq!(integer.car(), None);
    assert_eq!(integer.cdr(), None);
    assert!(!integer.is_char());
    assert!(!integer.is_symbol());
    assert!(!integer.is_cons());
    assert!(!integer.is_byte_array());
    assert_eq!(integer.decode_number_as_u32(), Some(7));

    let encoded = LispValue::from_u32(23);
    assert_eq!(encoded.decode_number_as_u32(), Some(23));

    let signed = LispValue::from_i32(41);
    assert_eq!(signed.decode_number_as_i32(), Some(41));

    let character = LispValue::from_char(b'V');
    assert!(character.is_char());
    assert_eq!(character.decode_char(), Some(b'V'));

    let floating = LispValue::from_f32(3.5);
    assert!(floating.is_number());
    assert_eq!(floating.decode_number_as_f32(), Some(3.5));

    let pair = LispValue::cons(integer, character);
    assert!(pair.is_cons());
    assert_eq!(pair.car(), Some(integer));
    assert_eq!(pair.cdr(), Some(character));
    assert_eq!(pair.reverse_list(), Some(pair));
    assert_eq!(integer.reverse_list(), None);

    let string = LispValue::from_u32(0x1234);
    assert_eq!(
        string.with_str(|value| value.to_bytes() == b"vesc"),
        Some(true)
    );
    assert_eq!(integer.with_str(|value| value.to_bytes() == b"vesc"), None);

    let bytes = LispValue::try_byte_array(4).expect("host fake allocates byte arrays");
    assert!(bytes.is_byte_array());
    assert_eq!(LispValue::try_byte_array(usize::MAX), None);
}
