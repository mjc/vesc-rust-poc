//! Shared Float Out Boy wire-format primitives.
//!
//! C map: Float Out Boy packet encoders forward through `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::{AngleDegrees, AngleRadians};

pub(crate) use vescpkg_rs::protocol_buffer::{
    saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
};

/// FOB-local fixed packet buffer using Refloat's incrementing-index writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyPacket<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FloatOutBoyPacket<N> {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, value: u8) {
        push_u8(&mut self.bytes, &mut self.len, value);
    }

    pub(crate) fn extend(&mut self, values: &[u8]) {
        push_bytes(&mut self.bytes, &mut self.len, values);
    }

    pub(crate) fn extend_fixed<const LEN: usize>(&mut self, value: &[u8]) {
        let copied = value.len().min(LEN);
        self.extend(value.get(..copied).unwrap_or_default());
        for _ in copied..LEN {
            self.push(0);
        }
    }

    pub(crate) fn push_u16(&mut self, value: u16) {
        push_u16(&mut self.bytes, &mut self.len, value);
    }

    pub(crate) fn push_i16(&mut self, value: i16) {
        self.extend(&value.to_be_bytes());
    }

    pub(crate) fn push_u32(&mut self, value: u32) {
        push_u32(&mut self.bytes, &mut self.len, value);
    }

    pub(crate) fn push_float32_auto(&mut self, value: f32) {
        push_float32_auto(&mut self.bytes, &mut self.len, value);
    }

    pub(crate) fn push_scaled_i16(&mut self, value: f32, scale: f32) {
        self.push_i16(saturating_trunc_f32_to_i16(value * scale));
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }

    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    pub(crate) const fn remaining(&self) -> usize {
        N.saturating_sub(self.len)
    }

    pub(crate) fn set(&mut self, index: usize, value: u8) -> bool {
        self.bytes.get_mut(index).is_some_and(|slot| {
            *slot = value;
            true
        })
    }

    pub(crate) const fn into_bytes(self) -> [u8; N] {
        self.bytes
    }
}

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
mod tests;
