//! VESC firmware buffer-compatible primitive encoders.

/// Append one byte using VESC's incrementing-index convention.
#[must_use]
pub fn append_u8(buffer: &mut [u8], index: &mut usize, value: u8) -> Option<()> {
    append_bytes(buffer, index, &[value])
}

/// Append a big-endian unsigned 16-bit integer.
#[must_use]
pub fn append_u16(buffer: &mut [u8], index: &mut usize, value: u16) -> Option<()> {
    append_bytes(buffer, index, &value.to_be_bytes())
}

/// Append a big-endian signed 16-bit integer.
#[must_use]
pub fn append_i16(buffer: &mut [u8], index: &mut usize, value: i16) -> Option<()> {
    append_u16(buffer, index, value.cast_unsigned())
}

/// Append a big-endian unsigned 32-bit integer.
#[must_use]
pub fn append_u32(buffer: &mut [u8], index: &mut usize, value: u32) -> Option<()> {
    append_bytes(buffer, index, &value.to_be_bytes())
}

/// Append VESC's automatic 32-bit float representation.
#[must_use]
pub fn append_float32_auto(buffer: &mut [u8], index: &mut usize, value: f32) -> Option<()> {
    append_u32(buffer, index, float32_auto_bits(value))
}

fn append_bytes(buffer: &mut [u8], index: &mut usize, bytes: &[u8]) -> Option<()> {
    let end = index.checked_add(bytes.len())?;
    buffer.get_mut(*index..end)?.copy_from_slice(bytes);
    *index = end;
    Some(())
}

/// Convert a float to VESC's automatic 32-bit wire representation.
#[must_use]
pub fn float32_auto_bits(value: f32) -> u32 {
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    value.to_bits()
}

#[cfg(test)]
mod tests {
    use super::{append_float32_auto, append_i16, append_u32};

    #[test]
    fn fixed_width_encoder_rejects_partial_output() {
        let mut bytes = [0xff; 3];
        let mut index = 1;

        assert_eq!(append_u32(&mut bytes, &mut index, 0x1234_5678), None);
        assert_eq!(bytes, [0xff; 3]);
        assert_eq!(index, 1);
    }

    #[test]
    fn signed_integer_encoder_uses_vesc_big_endian_bytes() {
        let mut bytes = [0xff; 4];
        let mut index = 1;

        assert_eq!(append_i16(&mut bytes, &mut index, -123), Some(()));

        assert_eq!(bytes, [0xff, 0xff, 0x85, 0xff]);
        assert_eq!(index, 3);
    }

    #[test]
    fn float32_auto_preserves_vesc_cutoff() {
        let mut bytes = [0xff; 8];
        let mut index = 0;

        assert_eq!(
            append_float32_auto(&mut bytes, &mut index, 1.4e-38),
            Some(()),
        );
        assert_eq!(
            append_float32_auto(&mut bytes, &mut index, 1.6e-38),
            Some(()),
        );

        assert_eq!(bytes, [0, 0, 0, 0, 0x00, 0xae, 0x39, 0x7e]);
    }
}
