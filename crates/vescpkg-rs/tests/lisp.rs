#![cfg(feature = "test-support")]

//! Integration tests for the safe LispBM value predicates.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::LispValue;

#[test]
fn lisp_values_expose_explicit_kind_predicates() {
    let _firmware = FirmwareTest::new();
    let integer = LispValue::try_from(7).expect("immediate integer fits");

    assert!(integer.is_integer());
    assert!(integer.is_number());
    assert!(!integer.is_char());
    assert!(!integer.is_symbol());
    assert!(!integer.is_cons());
    assert!(!integer.is_byte_array());
}
