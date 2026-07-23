#![cfg(feature = "test-support")]

//! Integration tests for the safe LispBM value predicates.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{
    LispContextId, LispFlatValue, LispFlatValueError, LispProcess, LispSymbol, LispValue,
};

#[test]
fn lisp_values_expose_explicit_kind_predicates() {
    let firmware = FirmwareTest::new();
    let integer = LispValue::try_from(7).expect("immediate integer fits");
    assert!(LispValue::nil().is_nil());
    assert!(LispValue::true_value().is_true());
    assert!(!integer.is_nil());
    assert!(!integer.is_true());

    assert!(integer.is_integer());
    assert!(integer.is_number());
    assert_eq!(integer.decode_char(), None);
    assert_eq!(integer.car(), None);
    assert_eq!(integer.cdr(), None);
    assert!(!integer.is_char());
    assert!(!integer.is_symbol());
    assert!(!integer.is_cons());
    assert!(!integer.is_byte_array());
    assert_eq!(integer.decode_i32_exact(), Some(7));
    assert_eq!(integer.decode_u32_exact(), Some(7));
    assert_eq!(integer.decode_i64_exact(), Some(7));
    assert_eq!(integer.decode_u64_exact(), Some(7));
    assert_eq!(LispValue::from_i32(-1).decode_u32_exact(), None);
    assert_eq!(LispValue::from_i32(-1).decode_u64_exact(), None);
    assert_eq!(integer.decode_number_as_u32(), Some(7));
    assert_eq!(integer.decode_number_as_u64(), Some(7));
    assert_eq!(integer.decode_number_as_i64(), Some(7));

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
    assert_eq!(floating.decode_number_as_f64(), Some(3.5));
    assert_eq!(floating.decode_f32_exact(), Some(3.5));
    assert_eq!(floating.decode_f64_exact(), Some(3.5));
    assert_eq!(integer.decode_f32_exact(), None);
    assert_eq!(integer.decode_f64_exact(), None);
    assert_eq!(LispValue::from_f64(3.5), Some(floating));
    assert_eq!(LispValue::from_f64(3.1), None);

    let pair = LispValue::cons(integer, character);
    assert!(pair.is_cons());
    assert_eq!(pair.car(), Some(integer));
    assert_eq!(pair.cdr(), Some(character));
    assert_eq!(pair.reverse_list(), Some(pair));
    assert_eq!(integer.reverse_list(), None);

    let string = LispValue::try_byte_array(4).expect("host fake allocates byte arrays");
    assert!(string.is_byte_array());
    assert!(string.is_array());
    assert!(string.is_string());
    assert!(!string.is_number());
    assert_eq!(
        string.with_str(|value| value.to_bytes() == b"vesc"),
        Some(true)
    );
    assert_eq!(integer.with_str(|value| value.to_bytes() == b"vesc"), None);

    assert_eq!(LispValue::try_byte_array(usize::MAX), None);
    assert!(!integer.is_array());
    assert!(!integer.is_string());

    let symbol = LispSymbol::new(7);
    let symbol_value = LispValue::from_symbol(symbol);
    assert!(symbol_value.is_symbol());
    assert_eq!(symbol_value.symbol_id(), Some(symbol));
    assert_eq!(integer.symbol_id(), None);

    assert!(integer.send_to(LispContextId::new(9)).is_ok());
    firmware.fail_lisp_messages();
    assert_eq!(
        integer.send_to(LispContextId::new(9)),
        Err(vescpkg_rs::LispMessageError::Rejected)
    );

    let current = LispProcess::current();
    assert!(!LispProcess::is_evaluation_paused());
    LispProcess::pause_evaluation(32);
    assert!(LispProcess::is_evaluation_paused());
    LispProcess::continue_evaluation();
    assert!(!LispProcess::is_evaluation_paused());
    LispProcess::block_current();
    assert!(LispProcess::unblock(current, integer).is_ok());
}

#[test]
fn lisp_lists_validate_tails_and_iterate_fallibly() {
    let _firmware = FirmwareTest::new();
    let integer = LispValue::try_from(7).expect("immediate integer fits");
    let character = LispValue::from_char(b'V');

    assert!(LispValue::nil().is_list());
    assert!(!integer.is_list());
    let proper = LispValue::cons(integer, LispValue::nil());
    assert!(proper.is_list());
    let mut list = proper.list();
    assert_eq!(list.next_value().unwrap(), Some(integer));
    assert_eq!(list.next_value().unwrap(), None);
    let mut iterator = proper.list();
    assert_eq!(iterator.next(), Some(Ok(integer)));
    assert_eq!(iterator.next(), None);

    let improper_pair = LispValue::cons(integer, character);
    assert!(!improper_pair.is_list());
    let mut improper = improper_pair.list();
    assert_eq!(improper.next_value().unwrap(), Some(integer));
    assert_eq!(
        improper.next_value(),
        Err(vescpkg_rs::LispListError::ImproperTail)
    );
    let mut improper_iterator = improper_pair.list();
    assert_eq!(improper_iterator.next(), Some(Ok(integer)));
    assert_eq!(
        improper_iterator.next(),
        Some(Err(vescpkg_rs::LispListError::ImproperTail))
    );
    assert_eq!(improper_iterator.next(), None);
}

#[test]
fn lisp_process_sets_error_reason_from_a_scoped_c_string() {
    let _firmware = FirmwareTest::new();
    let reason = c"invalid argument";

    assert_eq!(LispProcess::set_error_reason(reason), 1);
    assert_eq!(LispProcess::set_error_reason(c""), 1);
}

#[test]
fn lisp_symbols_can_be_looked_up_from_a_scoped_c_string() {
    let _firmware = FirmwareTest::new();

    assert_eq!(LispSymbol::lookup(c"vesc"), Some(LispSymbol::new(7)));
    assert_eq!(LispSymbol::lookup(c"missing"), None);
}

#[test]
fn lisp_flat_values_encode_wide_values_and_unblock_contexts() {
    let _firmware = FirmwareTest::new();
    let mut value = LispFlatValue::try_new(32).expect("flat-value slots available");

    value.push_i64(-42).unwrap();
    value.push_u64(0xfeed_beef).unwrap();
    value.push_i(-7).unwrap();
    value.push_cons().unwrap();
    value.push_symbol(LispSymbol::new(7)).unwrap();
    value.push_i32(-7).unwrap();
    value.push_u32(23).unwrap();
    value.push_float(3.5).unwrap();
    value.push_byte(b'V').unwrap();
    value.push_byte_array(b"vesc").unwrap();
    value.finish().unwrap();
    assert_eq!(value.finish(), Ok(()));
    assert_eq!(value.push_i32(1), Err(LispFlatValueError::AlreadyFinished));
    LispProcess::unblock_flat(LispContextId::new(9), value).expect("context accepts value");

    let value = LispFlatValue::try_new(4).expect("flat-value slots available");
    drop(value);
    assert!(LispFlatValue::try_new(257).is_none());

    let mut value = LispFlatValue::try_new(4).expect("flat-value slots available");
    value.push_byte(b'V').unwrap();
    LispProcess::unblock_flat(LispContextId::new(9), value)
        .expect("unblock finishes an unfinished value");
}
