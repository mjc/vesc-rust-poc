//! Shared Float Out Boy wire-format primitives.
//!
//! C map: Float Out Boy packet encoders forward through `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::{AngleDegrees, AngleRadians};

pub(crate) use vescpkg_rs::protocol_buffer::{
    saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
};

pub(crate) fn push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    // C map: upstream packet writers increment a byte index and stop storing
    // once the buffer is full; Rust keeps that no-panics boundary behavior.
    if let Some(slot) = buffer.get_mut(*ind) {
        *slot = value;
    }
    *ind = ind.saturating_add(1);
}

pub(crate) fn push_bytes(buffer: &mut [u8], ind: &mut usize, values: &[u8]) {
    values
        .iter()
        .copied()
        .for_each(|byte| push_u8(buffer, ind, byte));
}

pub(crate) fn push_u16(buffer: &mut [u8], ind: &mut usize, value: u16) {
    // C map: `buffer_append_uint16` writes big-endian unsigned integers at
    // `third_party/float-out-boy/src/conf/buffer.c:62-67`.
    push_bytes(buffer, ind, &value.to_be_bytes());
}

pub(crate) fn push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    // C map: `buffer_append_uint32` writes big-endian unsigned integers at
    // `third_party/float-out-boy/src/conf/buffer.c:83-90`.
    push_bytes(buffer, ind, &value.to_be_bytes());
}

pub(crate) fn push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // C map: the shared codec preserves `buffer_append_float32_auto`'s exact
    // `1.5e-38` cutoff from `third_party/float-out-boy/src/conf/buffer.c:118-140`.
    push_u32(buffer, ind, float32_auto_bits(value));
}

fn float32_auto_bits(value: f32) -> u32 {
    vescpkg_rs::protocol_buffer::float32_auto_bits(value)
}

pub(crate) fn degrees(angle: AngleRadians) -> f32 {
    // C map: Float Out Boy app-data packets emit firmware attitude radians as degrees
    // at `third_party/float-out-boy/src/main.c:1267-1399` and `third_party/float-out-boy/src/main.c:1881-1930`.
    AngleDegrees::from(angle).as_degrees()
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a narrowing integer cast specifies the required low-32-bit timestamp wrapping"
)]
pub(crate) const fn truncating_u64_to_u32(value: u64) -> u32 {
    value as u32
}

#[cfg(test)]
pub(crate) const fn saturating_usize_to_u8(value: usize) -> u8 {
    // Packet string lengths are one byte in the upstream C format. Saturating
    // prevents a malformed or future oversized field from wrapping its length.
    if value > 255 {
        return u8::MAX;
    }
    let [low, ..] = value.to_le_bytes();
    low
}

#[cfg(test)]
mod tests {
    use super::{
        saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
        saturating_usize_to_u8, truncating_u64_to_u32,
    };

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
}
