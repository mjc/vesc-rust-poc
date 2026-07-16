//! VESC firmware buffer-compatible primitive encoders.

/// Append one byte using VESC's incrementing-index convention.
#[inline(always)]
pub fn append_u8(buffer: &mut [u8], index: &mut usize, value: u8) {
    if let Some(slot) = buffer.get_mut(*index) {
        *slot = value;
    }
    *index = index.saturating_add(1);
}

/// Append a big-endian unsigned 16-bit integer.
#[inline(always)]
pub fn append_u16(buffer: &mut [u8], index: &mut usize, value: u16) {
    let [high, low] = value.to_be_bytes();
    append_u8(buffer, index, high);
    append_u8(buffer, index, low);
}

/// Append a big-endian unsigned 32-bit integer.
#[inline(always)]
pub fn append_u32(buffer: &mut [u8], index: &mut usize, value: u32) {
    let [byte3, byte2, byte1, byte0] = value.to_be_bytes();
    append_u8(buffer, index, byte3);
    append_u8(buffer, index, byte2);
    append_u8(buffer, index, byte1);
    append_u8(buffer, index, byte0);
}

/// Decode a signed big-endian 16-bit value using VESC's scaled-float convention.
#[must_use]
#[inline(always)]
pub fn get_float16(buffer: &[u8], scale: f32, index: &mut usize) -> Option<f32> {
    let end = index.checked_add(2)?;
    let bytes: [u8; 2] = buffer.get(*index..end)?.try_into().ok()?;
    *index = end;
    Some(f32::from(i16::from_be_bytes(bytes)) / scale)
}

/// Append VESC's automatic 16-bit float representation.
#[inline(always)]
pub fn append_float16_auto(buffer: &mut [u8], index: &mut usize, value: f32) {
    append_u16(buffer, index, float16_auto_bits(value));
}

/// Append VESC's automatic 32-bit float representation.
#[inline(always)]
pub fn append_float32_auto(buffer: &mut [u8], index: &mut usize, value: f32) {
    append_u32(buffer, index, float32_auto_bits(value));
}

/// Convert a float to VESC's automatic 16-bit wire representation.
#[must_use]
#[inline(always)]
pub fn float16_auto_bits(value: f32) -> u16 {
    let bits = value.to_bits().wrapping_add(0x0000_1000);
    let exponent = (bits & 0x7f80_0000) >> 23;
    let mantissa = bits & 0x007f_ffff;
    let normalized = if exponent > 112 {
        (((exponent - 112) << 10) & 0x7c00) | (mantissa >> 13)
    } else {
        0
    };
    let denormalized = if exponent < 113 && exponent > 101 {
        (((0x007f_f000 + mantissa) >> (125 - exponent)) + 1) >> 1
    } else {
        0
    };
    let saturated = if exponent > 143 { 0x7fff } else { 0 };
    (((bits & 0x8000_0000) >> 16) | normalized | denormalized | saturated) as u16
}

/// Convert a float to VESC's automatic 32-bit wire representation.
#[must_use]
#[inline(always)]
pub fn float32_auto_bits(value: f32) -> u32 {
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    value.to_bits()
}

#[cfg(test)]
mod tests {
    use super::{append_float16_auto, append_float32_auto, get_float16};

    #[test]
    fn float16_decoder_matches_vesc_scaling_and_bounds() {
        let mut index = 1;

        assert_eq!(
            get_float16(&[0xff, 0xff, 0x85], 10.0, &mut index),
            Some(-12.3)
        );
        assert_eq!(index, 3);
        assert_eq!(get_float16(&[0xff, 0xff, 0x85], 10.0, &mut index), None);
        assert_eq!(index, 3);
    }

    #[test]
    fn float32_auto_preserves_vesc_cutoff() {
        let mut bytes = [0xff; 8];
        let mut index = 0;

        append_float32_auto(&mut bytes, &mut index, 1.4e-38);
        append_float32_auto(&mut bytes, &mut index, 1.6e-38);

        assert_eq!(bytes, [0, 0, 0, 0, 0x00, 0xae, 0x39, 0x7e]);
    }

    #[test]
    fn float16_auto_matches_vesc_saturation_and_denormal_encoding() {
        let mut bytes = [0; 6];
        let mut index = 0;

        append_float16_auto(&mut bytes, &mut index, 1.0);
        append_float16_auto(&mut bytes, &mut index, 0.000_061_035_156);
        append_float16_auto(&mut bytes, &mut index, f32::INFINITY);

        assert_eq!(bytes, [0x3c, 0x00, 0x04, 0x00, 0x7f, 0xff]);
    }
}
