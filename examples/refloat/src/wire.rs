//! Shared Refloat wire-format primitives.
//!
//! C map: Refloat packet encoders forward through `third_party/refloat/src/conf/buffer.c:33-145`.

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
    // `third_party/refloat/src/conf/buffer.c:62-67`.
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| push_u8(buffer, ind, byte));
}

pub(crate) fn push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    // C map: `buffer_append_uint32` writes big-endian unsigned integers at
    // `third_party/refloat/src/conf/buffer.c:83-90`.
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| push_u8(buffer, ind, byte));
}

pub(crate) fn push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // C map: `buffer_append_float32_auto` zeros subnormals before writing the
    // big-endian IEEE-754 bits at `third_party/refloat/src/conf/buffer.c:118-140`.
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    push_u32(buffer, ind, value.to_bits());
}

pub(crate) fn degrees(angle: AngleRadians) -> f32 {
    // C map: Refloat app-data packets emit firmware attitude radians as degrees
    // at `third_party/refloat/src/main.c:1267-1399` and `third_party/refloat/src/main.c:1881-1930`.
    AngleDegrees::from(angle).as_degrees()
}
