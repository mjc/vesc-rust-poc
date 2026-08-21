use super::{
    saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
    truncating_u64_to_u32,
};

const fn saturating_usize_to_u8(value: usize) -> u8 {
    // Packet string lengths are one byte in the upstream C format. Saturating
    // prevents a malformed or future oversized field from wrapping its length.
    if value > 255 {
        return u8::MAX;
    }
    let [low, ..] = value.to_le_bytes();
    low
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
