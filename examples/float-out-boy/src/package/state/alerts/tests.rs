use super::{FAULT_NAME_MAX_BYTES, bounded_fault_name};

#[test]
fn fault_names_match_refloats_fifty_byte_wire_limit() {
    let long_name = [b'X'; FAULT_NAME_MAX_BYTES + 1];

    assert_eq!(bounded_fault_name(b"SHORT"), b"SHORT");
    assert_eq!(
        bounded_fault_name(&long_name),
        &long_name[..FAULT_NAME_MAX_BYTES],
    );
}

#[test]
fn fault_names_drop_refloats_firmware_prefix() {
    assert_eq!(
        bounded_fault_name(b"FAULT_CODE_OVER_VOLTAGE"),
        b"OVER_VOLTAGE",
    );
}
