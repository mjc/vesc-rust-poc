//! Float Out Boy wire-format primitives.
//!
//! C map: Float Out Boy packet encoders forward through `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::{AngleDegrees, AngleRadians};

pub use vescpkg_rs::protocol_buffer::{
    FixedBuffer as FloatOutBoyPacket, saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8,
    saturating_trunc_f32_to_u32, truncating_u64_to_u32,
};

/// Convert a firmware radian angle to the degrees used by FOB app data.
#[must_use]
pub fn degrees(angle: AngleRadians) -> f32 {
    // C map: Float Out Boy app-data packets emit firmware attitude radians as degrees
    // at `third_party/float-out-boy/src/main.c:1267-1399` and `third_party/float-out-boy/src/main.c:1881-1930`.
    AngleDegrees::from(angle).as_degrees()
}

#[cfg(test)]
/// Saturate a host-size test length to the one-byte FOB string length.
#[must_use]
pub const fn saturating_usize_to_u8(value: usize) -> u8 {
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
