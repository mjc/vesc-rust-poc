//! Public declaration-macro behavior.

use vescpkg_rs::{
    const_field_builders, const_field_getters, const_forward_getters, typed_fields, typed_newtype,
    wire_enum,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pair {
    left: u8,
    right: u8,
}

impl Pair {
    const_field_getters! {
        pub fn left -> u8 = left;
        pub fn right -> u8 = right;
    }

    const_field_builders! {
        pub fn with_left(value: u8) => left;
        pub fn with_right(value: u8) => right;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NestedPair {
    pair: Pair,
}

impl NestedPair {
    const_forward_getters! {
        pub fn left -> u8 = pair.left();
        pub fn right -> u8 = pair.right();
    }
}

typed_fields! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Fields {
        count: u8 => count => with_count,
        enabled: bool => enabled,
    }
}

typed_newtype! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Token(u16);
    new(value);
    get;
}

wire_enum! {
    enum WireMode {
        First = 3,
        Second = 9,
    }
}

#[test]
fn declaration_macros_generate_const_value_apis() {
    const PAIR: Pair = Pair { left: 1, right: 2 }.with_left(3).with_right(4);
    const NESTED_PAIR: NestedPair = NestedPair { pair: PAIR };
    const FIELDS: Fields = Fields::new(4, true).with_count(7);
    const TOKEN: Token = Token::new(42);

    assert_eq!((PAIR.left(), PAIR.right()), (3, 4));
    assert_eq!((NESTED_PAIR.left(), NESTED_PAIR.right()), (3, 4));
    assert_eq!((FIELDS.count(), FIELDS.enabled()), (7, true));
    assert_eq!(TOKEN.get(), 42);
}

#[test]
fn wire_enum_preserves_ids_and_rejects_unknown_values() {
    assert_eq!(WireMode::First.id(), 3);
    assert_eq!(WireMode::try_from(9), Ok(WireMode::Second));
    assert_eq!(WireMode::try_from(4), Err(4));
}
