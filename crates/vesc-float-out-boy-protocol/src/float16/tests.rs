use super::encode_float_out_boy_float16;

#[test]
fn float16_matches_float_out_boy_encoding() {
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
        assert_eq!(encode_float_out_boy_float16(value), expected);
    }
}
