//! Refloat app-data protocol wire helpers.
//!
//! C map: app-data packet encoders forward through Refloat buffer helpers in
//! `third_party/refloat/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::AngleRadians;

pub(super) fn refloat_degrees(angle: AngleRadians) -> f32 {
    // C map: this converts firmware `radians` telemetry into degrees before encoding
    // payload fields in `third_party/refloat/src/main.c:1267-1310`.
    crate::wire::degrees(angle)
}

pub(super) fn refloat_realtime_push_float16_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float16_auto` at
    // `third_party/refloat/src/conf/buffer.c:143-145`, which writes `to_float16` big-endian.
    refloat_realtime_push_u16(buffer, ind, refloat_float16_auto_bits(value));
}

#[cfg(test)]
pub(super) fn refloat_realtime_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float32_auto` at
    // `third_party/refloat/src/conf/buffer.c:118-140`, zeroing denormal/subnormal values before
    // writing big-endian IEEE-754 bits.
    crate::wire::push_float32_auto(buffer, ind, value);
}

fn refloat_float16_auto_bits(value: f32) -> u16 {
    // Refloat's `to_float16` is defined at `third_party/refloat/src/conf/buffer.c:33-43`.
    let b = value.to_bits().wrapping_add(0x0000_1000);
    let e = (b & 0x7f80_0000) >> 23;
    let m = b & 0x007f_ffff;
    let normalized = if e > 112 {
        (((e - 112) << 10) & 0x7c00) | (m >> 13)
    } else {
        0
    };
    let denormalized = if e < 113 && e > 101 {
        (((0x007f_f000 + m) >> (125 - e)) + 1) >> 1
    } else {
        0
    };
    let saturated = if e > 143 { 0x7fff } else { 0 };
    (((b & 0x8000_0000) >> 16) | normalized | denormalized | saturated) as u16
}

pub(super) fn refloat_realtime_push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    crate::wire::push_u32(buffer, ind, value);
}

fn refloat_realtime_push_u16(buffer: &mut [u8], ind: &mut usize, value: u16) {
    crate::wire::push_u16(buffer, ind, value);
}

pub(super) fn refloat_realtime_push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    crate::wire::push_u8(buffer, ind, value);
}
