//! Float Out Boy app-data protocol wire helpers.
//!
//! C map: app-data packet encoders forward through Float Out Boy buffer helpers in
//! `third_party/float-out-boy/src/conf/buffer.c:33-145`.

#![cfg_attr(not(test), deny(clippy::arithmetic_side_effects))]

use vescpkg_rs::prelude::AngleRadians;

pub(super) fn float_out_boy_degrees(angle: AngleRadians) -> f32 {
    // C map: this converts firmware `radians` telemetry into degrees before encoding
    // payload fields in `third_party/float-out-boy/src/main.c:1267-1310`.
    crate::wire::degrees(angle)
}

pub(super) fn push_float_out_boy_float16(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Float Out Boy forwards through `buffer_append_float16_auto` at
    // `third_party/float-out-boy/src/conf/buffer.c:143-145`, which writes `to_float16` big-endian.
    float_out_boy_realtime_push_u16(buffer, ind, encode_float_out_boy_float16(value));
}

#[must_use]
#[inline]
fn encode_float_out_boy_float16(value: f32) -> u16 {
    let bits = value.to_bits().wrapping_add(0x0000_1000);
    let exponent = (bits & 0x7f80_0000) >> 23;
    let mantissa = bits & 0x007f_ffff;
    let normalized = if exponent > 112 {
        ((exponent.saturating_sub(112) << 10) & 0x7c00) | (mantissa >> 13)
    } else {
        0
    };
    // In this branch the exponent makes the right shift 13 through 23 bits.
    // `wrapping_shr` also keeps a future out-of-range shift panic-free without
    // changing any valid Float Out Boy encoding.
    let denormalized = if exponent < 113 && exponent > 101 {
        (0x007f_f000_u32
            .saturating_add(mantissa)
            .wrapping_shr(125_u32.saturating_sub(exponent))
            .saturating_add(1))
            >> 1
    } else {
        0
    };
    let saturated = if exponent > 143 { 0x7fff } else { 0 };
    let encoded = ((bits & 0x8000_0000) >> 16) | normalized | denormalized | saturated;
    u16::try_from(encoded).unwrap_or(u16::MAX)
}

pub(super) fn float_out_boy_realtime_push_float32_auto(
    buffer: &mut [u8],
    ind: &mut usize,
    value: f32,
) {
    // Float Out Boy forwards through `buffer_append_float32_auto` at
    // `third_party/float-out-boy/src/conf/buffer.c:118-140`, preserving its exact
    // `1.5e-38` cutoff before big-endian encoding.
    crate::wire::push_float32_auto(buffer, ind, value);
}

pub(in crate::package) fn float_out_boy_realtime_push_u32(
    buffer: &mut [u8],
    ind: &mut usize,
    value: u32,
) {
    crate::wire::push_u32(buffer, ind, value);
}

fn float_out_boy_realtime_push_u16(buffer: &mut [u8], ind: &mut usize, value: u16) {
    crate::wire::push_u16(buffer, ind, value);
}

pub(in crate::package) fn float_out_boy_realtime_push_u8(
    buffer: &mut [u8],
    ind: &mut usize,
    value: u8,
) {
    crate::wire::push_u8(buffer, ind, value);
}

#[cfg(test)]
mod tests {
    use super::encode_float_out_boy_float16;

    #[test]
    fn float16_matches_float_out_boy_encoding() {
        for (value, expected) in [
            (0.0, 0x0000),
            (-0.0, 0x8000),
            (1.0, 0x3c00),
            (-1.0, 0xbc00),
            (5.960_464_5e-8, 0x0001),
            (0.000_061_035_156, 0x0400),
            (131_008.0, 0x7fff),
            (f32::INFINITY, 0x7fff),
            (f32::NEG_INFINITY, 0xffff),
        ] {
            assert_eq!(encode_float_out_boy_float16(value), expected);
        }
    }
}
