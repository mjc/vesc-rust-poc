//! VESC firmware buffer-compatible primitive encoders.

/// Append one byte using VESC's incrementing-index convention.
#[must_use]
#[inline(always)]
pub fn append_u8(buffer: &mut [u8], index: &mut usize, value: u8) -> Option<()> {
    append_be(buffer, index, value.to_be_bytes())
}

/// Append a big-endian unsigned 16-bit integer.
#[must_use]
#[inline(always)]
pub fn append_u16(buffer: &mut [u8], index: &mut usize, value: u16) -> Option<()> {
    append_be(buffer, index, value.to_be_bytes())
}

/// Append a big-endian signed 16-bit integer.
#[must_use]
#[inline(always)]
pub fn append_i16(buffer: &mut [u8], index: &mut usize, value: i16) -> Option<()> {
    append_be(buffer, index, value.to_be_bytes())
}

/// Append a big-endian unsigned 32-bit integer.
#[must_use]
#[inline(always)]
pub fn append_u32(buffer: &mut [u8], index: &mut usize, value: u32) -> Option<()> {
    append_be(buffer, index, value.to_be_bytes())
}

/// Append one big-endian signed 32-bit integer.
#[must_use]
#[inline(always)]
pub fn append_i32(buffer: &mut [u8], index: &mut usize, value: i32) -> Option<()> {
    append_be(buffer, index, value.to_be_bytes())
}

/// Append VESC's automatic 32-bit float representation.
#[must_use]
#[inline(always)]
pub fn append_float32_auto(buffer: &mut [u8], index: &mut usize, value: f32) -> Option<()> {
    append_u32(buffer, index, float32_auto_bits(value))
}

/// Read one big-endian unsigned 32-bit integer.
#[must_use]
#[inline(always)]
pub fn read_u32(buffer: &[u8], index: &mut usize) -> Option<u32> {
    read_be(buffer, index).map(u32::from_be_bytes)
}

/// Read one big-endian signed 32-bit integer.
#[must_use]
#[inline(always)]
pub fn read_i32(buffer: &[u8], index: &mut usize) -> Option<i32> {
    read_be(buffer, index).map(i32::from_be_bytes)
}

/// Read VESC's automatic 32-bit float representation.
#[must_use]
#[inline(always)]
pub fn read_float32_auto(buffer: &[u8], index: &mut usize) -> Option<f32> {
    read_u32(buffer, index).map(f32::from_bits)
}

#[inline(always)]
fn append_bytes(buffer: &mut [u8], index: &mut usize, bytes: &[u8]) -> Option<()> {
    let end = index.checked_add(bytes.len())?;
    buffer.get_mut(*index..end)?.copy_from_slice(bytes);
    *index = end;
    Some(())
}

#[inline(always)]
fn append_be<const N: usize>(buffer: &mut [u8], index: &mut usize, bytes: [u8; N]) -> Option<()> {
    append_bytes(buffer, index, &bytes)
}

#[inline(always)]
fn read_be<const N: usize>(buffer: &[u8], index: &mut usize) -> Option<[u8; N]> {
    let end = index.checked_add(N)?;
    let bytes = buffer.get(*index..end)?;
    let value = bytes.try_into().ok()?;
    *index = end;
    Some(value)
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
    use super::{
        append_float32_auto, append_i16, append_i32, append_u32, read_float32_auto, read_i32,
        read_u32,
    };

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
    fn signed_i32_encoder_preserves_twos_complement_wire_bits() {
        let mut bytes = [0; 4];
        let mut index = 0;

        assert_eq!(append_i32(&mut bytes, &mut index, -42), Some(()));
        assert_eq!(bytes, (-42_i32).to_be_bytes());
        assert_eq!(index, 4);
    }

    #[test]
    fn signed_i32_decoder_preserves_twos_complement_wire_bits() {
        let mut index = 0;
        assert_eq!(read_i32(&(-42_i32).to_be_bytes(), &mut index), Some(-42));
        assert_eq!(index, 4);
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

    #[test]
    fn fixed_width_decoders_reject_partial_input_without_advancing() {
        let mut index = 1;
        assert_eq!(read_u32(&[0xff, 0x01, 0x02, 0x03], &mut index), None);
        assert_eq!(index, 1);
    }

    #[test]
    fn float32_auto_round_trips_through_the_public_wire_boundary() {
        let mut bytes = [0; 4];
        let mut write_index = 0;
        append_float32_auto(&mut bytes, &mut write_index, -12.5).expect("four bytes");

        let mut read_index = 0;
        assert_eq!(read_float32_auto(&bytes, &mut read_index), Some(-12.5));
        assert_eq!(read_index, 4);
    }
}
