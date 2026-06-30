//! Compact LispBM integer encoding for device extensions.

const LBM_INT_TAG: u32 = 0x8;
const LBM_VALUE_SHIFT: u32 = 4;

/// Encode an `i32` as a raw LispBM integer value without calling firmware `lbm_enc_i`.
pub fn encode_lbm_i32_raw(value: i32) -> u32 {
    value.wrapping_shl(LBM_VALUE_SHIFT) as u32 | LBM_INT_TAG
}

/// Decodes a raw LispBM integer value for unit tests.
#[cfg(test)]
pub fn decode_lbm_i32_raw(value: u32) -> i32 {
    (value as i32) >> LBM_VALUE_SHIFT
}

#[cfg(test)]
mod tests {
    use super::{decode_lbm_i32_raw, encode_lbm_i32_raw};

    #[test]
    fn encodes_lispbm_integers_with_the_device_tag() {
        assert_eq!(encode_lbm_i32_raw(42), 0x2a8);
    }

    #[test]
    fn decode_lbm_i32_raw_reverses_encode() {
        assert_eq!(decode_lbm_i32_raw(encode_lbm_i32_raw(-3)), -3);
        assert_eq!(decode_lbm_i32_raw(encode_lbm_i32_raw(42)), 42);
    }
}
