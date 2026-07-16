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

/// Append a big-endian signed 16-bit integer.
#[inline(always)]
pub fn append_i16(buffer: &mut [u8], index: &mut usize, value: i16) {
    append_u16(buffer, index, value as u16);
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

/// Append VESC's automatic 32-bit float representation.
#[inline(always)]
pub fn append_float32_auto(buffer: &mut [u8], index: &mut usize, value: f32) {
    append_u32(buffer, index, float32_auto_bits(value));
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
    use super::{append_float32_auto, append_i16};

    #[test]
    fn signed_integer_encoder_uses_vesc_big_endian_bytes() {
        let mut bytes = [0xff; 4];
        let mut index = 1;

        append_i16(&mut bytes, &mut index, -123);

        assert_eq!(bytes, [0xff, 0xff, 0x85, 0xff]);
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
}
