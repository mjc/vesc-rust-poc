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
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| push_u8(buffer, ind, byte));
}

pub(crate) fn push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    // C map: `buffer_append_uint32` writes big-endian unsigned integers at
    // `third_party/float-out-boy/src/conf/buffer.c:83-90`.
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| push_u8(buffer, ind, byte));
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
