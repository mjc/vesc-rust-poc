//! Shared Float Out Boy wire-format primitives.
//!
//! C map: Float Out Boy packet encoders forward through `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::{AngleDegrees, AngleRadians};

pub(crate) fn push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    // C map: upstream packet writers increment a byte index and stop storing
    // once the buffer is full; Rust keeps that no-panics boundary behavior.
    if let Some(slot) = buffer.get_mut(*ind) {
        *slot = value;
    }
    *ind = ind.saturating_add(1);
}

pub(crate) fn push_u16(buffer: &mut [u8], ind: &mut usize, value: u16) {
    // C map: `buffer_append_uint16` writes big-endian unsigned integers at
    // `third_party/float-out-boy/src/conf/buffer.c:62-67`.
    for byte in value.to_be_bytes() {
        push_u8(buffer, ind, byte);
    }
}

pub(crate) fn push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    // C map: `buffer_append_uint32` writes big-endian unsigned integers at
    // `third_party/float-out-boy/src/conf/buffer.c:83-90`.
    for byte in value.to_be_bytes() {
        push_u8(buffer, ind, byte);
    }
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

pub(crate) fn saturating_trunc_f32_to_u32(value: f32) -> u32 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }

    // 2^32 is exactly representable as f32, whereas u32::MAX rounds up to it.
    // Testing this boundary first also handles positive infinity.
    if value >= 4_294_967_296.0 {
        return u32::MAX;
    }

    let bits = value.to_bits();
    let exponent_bits = (bits >> 23) & 0xff;
    let exponent = i32::from(u8::try_from(exponent_bits).unwrap_or(u8::MAX)) - 127;
    if exponent < 0 {
        return 0;
    }

    let significand = (bits & 0x007f_ffff) | 0x0080_0000;
    if exponent >= 23 {
        let shift = u32::try_from(exponent - 23).unwrap_or(u32::MAX);
        significand.checked_shl(shift).unwrap_or(u32::MAX)
    } else {
        let shift = u32::try_from(23 - exponent).unwrap_or(u32::MAX);
        significand.checked_shr(shift).unwrap_or(0)
    }
}

pub(crate) fn saturating_trunc_f32_to_u8(value: f32) -> u8 {
    u8::try_from(saturating_trunc_f32_to_u32(value)).unwrap_or(u8::MAX)
}

pub(crate) fn saturating_trunc_f32_to_i16(value: f32) -> i16 {
    if value.is_nan() {
        return 0;
    }
    if value >= 32_768.0 {
        return i16::MAX;
    }
    if value <= -32_768.0 {
        return i16::MIN;
    }

    let magnitude = saturating_trunc_f32_to_u32(value.abs());
    let signed = i32::try_from(magnitude).unwrap_or(i32::MAX);
    let signed = if value.is_sign_negative() {
        -signed
    } else {
        signed
    };
    i16::try_from(signed).unwrap_or(if value.is_sign_negative() {
        i16::MIN
    } else {
        i16::MAX
    })
}

pub(crate) const fn truncating_u64_to_u32(value: u64) -> u32 {
    // VESC timestamps wrap at 32 bits. Selecting the low bytes states that
    // wire behavior directly and cannot panic if a wider counter is supplied.
    let [byte0, byte1, byte2, byte3, ..] = value.to_le_bytes();
    u32::from_le_bytes([byte0, byte1, byte2, byte3])
}

pub(crate) const fn truncating_usize_to_u8(value: usize) -> u8 {
    // Packet string lengths are one byte in the upstream C format. Selecting
    // the low byte preserves that ABI without a potentially panicking index.
    let [low, ..] = value.to_le_bytes();
    low
}

#[cfg(test)]
mod tests {
    use super::{
        saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
        truncating_u64_to_u32, truncating_usize_to_u8,
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
    fn timestamp_conversion_keeps_the_low_wrapping_bits() {
        assert_eq!(truncating_u64_to_u32(0x0000_0001_ffff_ffff), u32::MAX);
        assert_eq!(truncating_u64_to_u32(0x0000_0001_0000_0000), 0);
        assert_eq!(truncating_usize_to_u8(0x1ff), u8::MAX);
    }
}
