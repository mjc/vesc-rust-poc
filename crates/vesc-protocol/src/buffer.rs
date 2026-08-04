//! VESC firmware buffer-compatible primitive encoders.

/// Fixed-capacity writer using VESC's incrementing-index overflow convention.
///
/// Writes past the physical buffer are discarded while the logical length keeps
/// advancing, matching firmware helpers that increment their output index after
/// every attempted byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    /// Build an empty zero-filled buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    /// Append one byte.
    #[inline]
    pub fn push(&mut self, value: u8) {
        if let Some(slot) = self.bytes.get_mut(self.len) {
            *slot = value;
        }
        self.len = self.len.saturating_add(1);
    }

    /// Append every byte from a slice.
    #[inline]
    pub fn extend(&mut self, values: &[u8]) {
        extend_incrementing(&mut self.bytes, &mut self.len, values);
    }

    /// Append a slice padded or truncated to exactly `LEN` logical bytes.
    #[inline]
    pub fn extend_fixed<const LEN: usize>(&mut self, value: &[u8]) {
        let copied = value.len().min(LEN);
        self.extend(value.get(..copied).unwrap_or_default());
        for _ in copied..LEN {
            self.push(0);
        }
    }

    /// Append a big-endian unsigned 16-bit integer.
    #[inline]
    pub fn push_u16(&mut self, value: u16) {
        self.extend(&value.to_be_bytes());
    }

    /// Append a big-endian signed 16-bit integer.
    #[inline]
    pub fn push_i16(&mut self, value: i16) {
        self.extend(&value.to_be_bytes());
    }

    /// Append a big-endian unsigned 32-bit integer.
    #[inline]
    pub fn push_u32(&mut self, value: u32) {
        self.extend(&value.to_be_bytes());
    }

    /// Append VESC's automatic 32-bit float representation.
    #[inline]
    pub fn push_float32_auto(&mut self, value: f32) {
        self.push_u32(float32_auto_bits(value));
    }

    /// Append VESC's automatic 16-bit float representation.
    #[inline]
    pub fn push_float16_auto(&mut self, value: f32) {
        self.push_u16(float16_auto_bits(value));
    }

    /// Append a scaled, truncating, saturating signed 16-bit value.
    #[inline]
    pub fn push_scaled_i16(&mut self, value: f32, scale: f32) {
        self.push_i16(saturating_trunc_f32_to_i16(value * scale));
    }

    /// Return the physically stored prefix.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }

    /// Return the logical number of attempted bytes, including overflow writes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether no writes have been attempted.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the physical capacity remaining for stored bytes.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        N.saturating_sub(self.len)
    }

    /// Replace one physically present byte.
    #[must_use]
    pub fn set(&mut self, index: usize, value: u8) -> bool {
        self.bytes.get_mut(index).is_some_and(|slot| {
            *slot = value;
            true
        })
    }

    /// Consume the writer and return its entire fixed-capacity storage.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; N] {
        self.bytes
    }
}

#[inline(never)]
fn extend_incrementing(buffer: &mut [u8], len: &mut usize, values: &[u8]) {
    for &value in values {
        if let Some(slot) = buffer.get_mut(*len) {
            *slot = value;
        }
        *len = len.saturating_add(1);
    }
}

impl<const N: usize> Default for FixedBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Append one byte using VESC's incrementing-index convention.
#[must_use]
pub fn append_u8(buffer: &mut [u8], index: &mut usize, value: u8) -> Option<()> {
    append_bytes(buffer, index, value.to_be_bytes())
}

/// Append a big-endian unsigned 16-bit integer.
#[must_use]
pub fn append_u16(buffer: &mut [u8], index: &mut usize, value: u16) -> Option<()> {
    append_bytes(buffer, index, value.to_be_bytes())
}

/// Append a big-endian signed 16-bit integer.
#[must_use]
pub fn append_i16(buffer: &mut [u8], index: &mut usize, value: i16) -> Option<()> {
    append_bytes(buffer, index, value.to_be_bytes())
}

/// Append a big-endian unsigned 32-bit integer.
#[must_use]
pub fn append_u32(buffer: &mut [u8], index: &mut usize, value: u32) -> Option<()> {
    append_bytes(buffer, index, value.to_be_bytes())
}

/// Append one big-endian signed 32-bit integer.
#[must_use]
pub fn append_i32(buffer: &mut [u8], index: &mut usize, value: i32) -> Option<()> {
    append_bytes(buffer, index, value.to_be_bytes())
}

/// Append VESC's automatic 32-bit float representation.
#[must_use]
pub fn append_float32_auto(buffer: &mut [u8], index: &mut usize, value: f32) -> Option<()> {
    append_u32(buffer, index, float32_auto_bits(value))
}

/// Read one big-endian unsigned 32-bit integer.
#[must_use]
pub fn read_u32(buffer: &[u8], index: &mut usize) -> Option<u32> {
    read_be(buffer, index).map(u32::from_be_bytes)
}

/// Read one big-endian signed 32-bit integer.
#[must_use]
pub fn read_i32(buffer: &[u8], index: &mut usize) -> Option<i32> {
    read_be(buffer, index).map(i32::from_be_bytes)
}

/// Read VESC's automatic 32-bit float representation.
#[must_use]
pub fn read_float32_auto(buffer: &[u8], index: &mut usize) -> Option<f32> {
    read_u32(buffer, index).map(f32::from_bits)
}

fn append_bytes<const N: usize>(
    buffer: &mut [u8],
    index: &mut usize,
    bytes: [u8; N],
) -> Option<()> {
    let end = index.checked_add(N)?;
    buffer.get_mut(*index..end)?.copy_from_slice(&bytes);
    *index = end;
    Some(())
}

fn read_be<const N: usize>(buffer: &[u8], index: &mut usize) -> Option<[u8; N]> {
    let end = index.checked_add(N)?;
    let bytes = buffer.get(*index..end)?;
    let value = bytes.try_into().ok()?;
    *index = end;
    Some(value)
}

/// Convert a float to VESC's automatic 32-bit wire representation.
#[must_use]
pub fn float32_auto_bits(value: f32) -> u32 {
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    value.to_bits()
}

/// Convert a float to VESC's automatic 16-bit wire representation.
#[must_use]
pub fn float16_auto_bits(value: f32) -> u16 {
    let bits = value.to_bits().wrapping_add(0x0000_1000);
    let exponent = (bits & 0x7f80_0000) >> 23;
    let mantissa = bits & 0x007f_ffff;
    let normalized = if exponent > 112 {
        ((exponent.saturating_sub(112) << 10) & 0x7c00) | (mantissa >> 13)
    } else {
        0
    };
    let denormalized = if exponent < 113 && exponent > 101 {
        (0x007f_f000_u32
            .saturating_add(mantissa)
            .wrapping_shr(125_u32.saturating_sub(exponent))
            .saturating_add(1))
            >> 1
    } else {
        0
    };
    let saturated = if exponent > 143 { 0x7fff } else { 0 };
    let encoded = ((bits & 0x8000_0000) >> 16) | normalized | denormalized | saturated;
    u16::try_from(encoded).unwrap_or(u16::MAX)
}

/// Convert a float using Rust's truncating, saturating unsigned semantics
/// without linking the comparatively large Cortex-M float-cast runtime helper.
#[must_use]
pub fn saturating_trunc_f32_to_u32(value: f32) -> u32 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    if value >= 4_294_967_296.0 {
        return u32::MAX;
    }

    let bits = value.to_bits();
    let [exponent_bits, ..] = ((bits >> 23) & 0xff).to_le_bytes();
    let exponent = i32::from(exponent_bits).saturating_sub(127);
    if exponent < 0 {
        return 0;
    }

    let significand = (bits & 0x007f_ffff) | 0x0080_0000;
    let shift = exponent.abs_diff(23);
    if exponent >= 23 {
        significand << shift
    } else {
        significand >> shift
    }
}

/// Convert a float to a saturating, truncating byte without a runtime helper.
#[must_use]
pub fn saturating_trunc_f32_to_u8(value: f32) -> u8 {
    let value = saturating_trunc_f32_to_u32(value);
    if value > u32::from(u8::MAX) {
        return u8::MAX;
    }
    let [value, ..] = value.to_le_bytes();
    value
}

/// Convert a float to a saturating, truncating signed 16-bit integer without a
/// runtime helper.
#[must_use]
pub fn saturating_trunc_f32_to_i16(value: f32) -> i16 {
    if value.is_nan() {
        return 0;
    }
    if value >= 32_768.0 {
        return i16::MAX;
    }
    if value <= -32_768.0 {
        return i16::MIN;
    }

    let magnitude = saturating_trunc_f32_to_u32(value.abs());
    let [low, high, ..] = magnitude.to_le_bytes();
    let magnitude = i16::from_le_bytes([low, high]);
    if value.is_sign_negative() {
        magnitude.saturating_neg()
    } else {
        magnitude
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FixedBuffer, append_float32_auto, append_i16, append_i32, append_u32, float16_auto_bits,
        read_float32_auto, read_i32, read_u32, saturating_trunc_f32_to_i16,
        saturating_trunc_f32_to_u8, saturating_trunc_f32_to_u32,
    };

    #[test]
    fn fixed_buffer_preserves_incrementing_partial_overflow_writes() {
        let mut packet = FixedBuffer::<5>::new();
        packet.push(0x12);
        packet.push_u16(0x3456);
        packet.push_u32(0x789a_bcde);

        assert_eq!(packet.as_bytes(), [0x12, 0x34, 0x56, 0x78, 0x9a]);
        assert_eq!(packet.len(), 7);
    }

    #[test]
    fn fixed_buffer_encodes_signed_scaled_and_float_values() {
        let mut packet = FixedBuffer::<8>::new();
        packet.push_scaled_i16(-12.39, 10.0);
        packet.push_float32_auto(1.4e-38);
        packet.push_i16(0x1234);

        assert_eq!(packet.into_bytes(), [0xff, 0x85, 0, 0, 0, 0, 0x12, 0x34]);
    }

    #[test]
    fn fixed_buffer_encodes_vesc_float16_values() {
        for (value, expected) in [
            (0.0, 0x0000),
            (-0.0, 0x8000),
            (1.0, 0x3c00),
            (-1.0, 0xbc00),
            (5.960_464_5e-8, 0x0001),
            (0.000_061_035_156, 0x0400),
            (131_008.0, 0x7fff),
            (f32::INFINITY, 0x7fff),
            (f32::NEG_INFINITY, 0xffff),
        ] {
            assert_eq!(float16_auto_bits(value), expected);
        }

        let mut packet = FixedBuffer::<6>::new();
        packet.push_float16_auto(1.0);
        packet.push_float16_auto(-1.0);
        packet.push_float16_auto(f32::INFINITY);

        assert_eq!(packet.into_bytes(), [0x3c, 0, 0xbc, 0, 0x7f, 0xff]);
    }

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

    #[test]
    fn float_to_integer_helpers_cover_nan_infinity_fraction_and_bounds() {
        assert_eq!(saturating_trunc_f32_to_u32(f32::NAN), 0);
        assert_eq!(saturating_trunc_f32_to_u32(-1.0), 0);
        assert_eq!(saturating_trunc_f32_to_u32(42.9), 42);
        assert_eq!(saturating_trunc_f32_to_u32(f32::INFINITY), u32::MAX);
        assert_eq!(saturating_trunc_f32_to_u8(255.9), u8::MAX);
        assert_eq!(saturating_trunc_f32_to_i16(-42.9), -42);
        assert_eq!(saturating_trunc_f32_to_i16(f32::NEG_INFINITY), i16::MIN);
    }
}
