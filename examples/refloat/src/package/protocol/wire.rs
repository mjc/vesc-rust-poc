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

fn refloat_float16_auto_bits(value: f32) -> u16 {
    vescpkg_rs::protocol_buffer::float16_auto_bits(value)
}

#[cfg(test)]
pub(super) fn refloat_realtime_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float32_auto` at
    // `third_party/refloat/src/conf/buffer.c:118-140`, preserving its exact
    // `1.5e-38` cutoff before big-endian encoding.
    crate::wire::push_float32_auto(buffer, ind, value);
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
