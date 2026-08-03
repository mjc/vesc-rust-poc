use super::{
    FloatOutBoyPacket, saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8,
    saturating_trunc_f32_to_u32, saturating_usize_to_u8, truncating_u64_to_u32,
};

#[test]
fn fob_packet_preserves_incrementing_partial_overflow_writes() {
    let mut packet = FloatOutBoyPacket::<5>::new();
    packet.push(0x12);
    packet.push_u16(0x3456);
    packet.push_u32(0x789a_bcde);

    assert_eq!(packet.as_bytes(), [0x12, 0x34, 0x56, 0x78, 0x9a]);
    assert_eq!(packet.len, 7);
}

#[test]
fn fob_packet_encodes_signed_scaled_and_float_values() {
    let mut packet = FloatOutBoyPacket::<8>::new();
    packet.push_scaled_i16(-12.39, 10.0);
    packet.push_float32_auto(1.4e-38);
    packet.push_i16(0x1234);

    assert_eq!(packet.into_bytes(), [0xff, 0x85, 0, 0, 0, 0, 0x12, 0x34]);
}

#[test]
fn unsigned_wire_conversion_saturates_without_panicking() {
    assert_eq!(saturating_trunc_f32_to_u32(f32::NAN), 0);
    assert_eq!(saturating_trunc_f32_to_u32(f32::NEG_INFINITY), 0);
    assert_eq!(saturating_trunc_f32_to_u32(-1.0), 0);
    assert_eq!(saturating_trunc_f32_to_u32(42.9), 42);
    assert_eq!(saturating_trunc_f32_to_u32(f32::INFINITY), u32::MAX);
    assert_eq!(saturating_trunc_f32_to_u32(f32::MAX), u32::MAX);
    assert_eq!(saturating_trunc_f32_to_u8(255.9), u8::MAX);
    assert_eq!(saturating_trunc_f32_to_u8(256.0), u8::MAX);
}

#[test]
fn signed_wire_conversion_saturates_without_panicking() {
    assert_eq!(saturating_trunc_f32_to_i16(f32::NAN), 0);
    assert_eq!(saturating_trunc_f32_to_i16(f32::NEG_INFINITY), i16::MIN);
    assert_eq!(saturating_trunc_f32_to_i16(-42.9), -42);
    assert_eq!(saturating_trunc_f32_to_i16(42.9), 42);
    assert_eq!(saturating_trunc_f32_to_i16(f32::INFINITY), i16::MAX);
    assert_eq!(saturating_trunc_f32_to_i16(f32::MAX), i16::MAX);
}

#[test]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the characterization oracle is Rust's specified float-to-integer cast"
)]
fn wire_float_conversions_match_rust_casts_across_the_bit_space_and_boundaries() {
    for bits in (u32::MIN..=u32::MAX).step_by(65_537) {
        let value = f32::from_bits(bits);
        assert_eq!(saturating_trunc_f32_to_u32(value), value as u32);
        assert_eq!(saturating_trunc_f32_to_u8(value), value as u8);
        assert_eq!(saturating_trunc_f32_to_i16(value), value as i16);
    }

    for value in [
        f32::NEG_INFINITY,
        -32_768.0,
        -0.999_999_94,
        -0.0,
        f32::from_bits(1),
        0.999_999_94,
        255.999_98,
        32_767.998,
        4_294_967_040.0,
        4_294_967_296.0,
        f32::INFINITY,
        f32::NAN,
    ] {
        assert_eq!(saturating_trunc_f32_to_u32(value), value as u32);
        assert_eq!(saturating_trunc_f32_to_u8(value), value as u8);
        assert_eq!(saturating_trunc_f32_to_i16(value), value as i16);
    }
}

#[test]
fn timestamp_conversion_keeps_the_low_wrapping_bits() {
    assert_eq!(truncating_u64_to_u32(0x0000_0001_ffff_ffff), u32::MAX);
    assert_eq!(truncating_u64_to_u32(0x0000_0001_0000_0000), 0);
    assert_eq!(saturating_usize_to_u8(0x1ff), u8::MAX);
}
