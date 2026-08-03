//! Shared Float Out Boy wire-format primitives.
//!
//! C map: Float Out Boy packet encoders forward through `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::{AngleDegrees, AngleRadians};

pub(crate) use vescpkg_rs::protocol_buffer::{
    FixedBuffer as FloatOutBoyPacket, saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8,
    saturating_trunc_f32_to_u32,
};

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
mod tests;
