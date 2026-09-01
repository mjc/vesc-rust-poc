use super::*;

#[test]
fn info_v2_response_matches_float_out_boy_qml_metadata() {
    let response = encode_float_out_boy_info_response(&[2, 0], 0, false, false);
    let bytes = response.as_bytes();

    // QML sends COMMAND_INFO version 2 at `ui.qml.in:693-697`; upstream
    // `cmd_info` replies with the v2 metadata layout at `third_party/float-out-boy/src/main.c:2108-2135`.
    assert_eq!(bytes.len(), 60);
    assert_eq!(&bytes[..4], &[101, 0, 2, 0]);
    assert_eq!(&bytes[4..17], b"Float Out Boy");
    assert_eq!(&bytes[24..27], &[0, 1, 0]);
    assert_eq!(
        u32::from_be_bytes([bytes[47], bytes[48], bytes[49], bytes[50]]),
        0x0ef6_e99d
    );
    assert_eq!(
        u32::from_be_bytes([bytes[51], bytes[52], bytes[53], bytes[54]]),
        FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ
    );
    assert_eq!(
        u32::from_be_bytes([bytes[55], bytes[56], bytes[57], bytes[58]]),
        0
    );
    assert_eq!(bytes[59], 0);
    let external_response = encode_float_out_boy_info_response(&[2, 0], 2, false, false);
    let external = external_response.as_bytes();
    assert_eq!(
        u32::from_be_bytes([external[55], external[56], external[57], external[58]]),
        3
    );
    let internal_response = encode_float_out_boy_info_response(&[2, 0], 1, false, false);
    let internal = internal_response.as_bytes();
    assert_eq!(
        u32::from_be_bytes([internal[55], internal[56], internal[57], internal[58]]),
        0
    );
    let operational_internal = encode_float_out_boy_info_response(&[2, 0], 1, true, false);
    assert_eq!(
        u32::from_be_bytes([
            operational_internal.as_bytes()[55],
            operational_internal.as_bytes()[56],
            operational_internal.as_bytes()[57],
            operational_internal.as_bytes()[58],
        ]),
        1
    );
    assert_eq!(
        &encode_float_out_boy_info_response(&[2, 0xa5], 0, false, false).as_bytes()[..4],
        &[101, 0, 2, 0xa5]
    );
}

#[test]
fn info_v1_response_matches_float_out_boy_legacy_shape_and_led_mapping() {
    assert_eq!(
        encode_float_out_boy_info_response(&[], 1, false, false).as_bytes(),
        &[101, 0, 1, 0, 1]
    );
    assert_eq!(
        encode_float_out_boy_info_response(&[1], 2, false, false).as_bytes(),
        &[101, 0, 1, 0, 3]
    );
    assert_eq!(
        encode_float_out_boy_info_response(&[1], 3, false, false).as_bytes(),
        &[101, 0, 1, 0, 3]
    );
}

#[test]
fn unknown_info_version_uses_v2_without_echoing_flags() {
    let response = encode_float_out_boy_info_response(&[99, 0xff], 0, false, false);

    assert_eq!(&response.as_bytes()[..4], &[101, 0, 2, 0]);
    assert_eq!(
        response.as_bytes().len(),
        FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN
    );
}

#[test]
fn info_v2_advertises_only_an_operational_recorder() {
    let response = encode_float_out_boy_info_response(&[2, 0], 0, false, true);
    let bytes = response.as_bytes();

    assert_eq!(
        u32::from_be_bytes([bytes[55], bytes[56], bytes[57], bytes[58]]),
        1 << 31
    );
}

#[test]
fn oversized_info_length_preserves_the_complete_buffer() {
    let mut response = FloatOutBoyInfoResponse::new();
    response.extend(&[0x5a; FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN]);
    response.push(0xff);

    assert_eq!(
        response.as_bytes(),
        &[0x5a; FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN]
    );
}

#[test]
fn realtime_data_ids_response_matches_float_out_boy_qml_metadata() {
    fn take_id_list<'a>(bytes: &'a [u8], index: &mut usize) -> std::vec::Vec<&'a str> {
        let count = bytes
            .get(*index)
            .copied()
            .map(usize::from)
            .expect("ID count byte");
        *index = index.saturating_add(1);

        (0..count)
            .map(|_| {
                let len = bytes
                    .get(*index)
                    .copied()
                    .map(usize::from)
                    .expect("ID length byte");
                *index = index.saturating_add(1);
                let end = index.saturating_add(len);
                let id = bytes.get(*index..end).expect("ID bytes");
                *index = end;
                core::str::from_utf8(id).expect("ID UTF-8")
            })
            .collect()
    }

    let bytes = encode_float_out_boy_realtime_data_ids_response();

    // QML asks for IDs at `ui.qml.in:704-705`;
    // upstream `cmd_realtime_data_ids` writes the counted string sets at
    // `third_party/float-out-boy/src/main.c:1876-1901`, using IDs from `third_party/float-out-boy/src/rt_data.h:38-66`.
    assert_eq!(bytes.len(), 414);
    assert_eq!(bytes.get(..2), Some(&[101, 32][..]));
    let mut index = 2;
    assert_eq!(
        take_id_list(&bytes, &mut index).as_slice(),
        &[
            "motor.speed",
            "motor.erpm",
            "motor.current",
            "motor.dir_current",
            "motor.filt_current",
            "motor.duty_cycle",
            "motor.batt_voltage",
            "motor.batt_current",
            "motor.mosfet_temp",
            "motor.motor_temp",
            "imu.pitch",
            "imu.balance_pitch",
            "imu.roll",
            "footpad.adc_left",
            "footpad.adc_right",
            "remote.input",
        ]
    );
    assert_eq!(
        take_id_list(&bytes, &mut index).as_slice(),
        &[
            "setpoint",
            "atr.setpoint",
            "brake_tilt.setpoint",
            "torque_tilt.setpoint",
            "turn_tilt.setpoint",
            "remote.setpoint",
            "balance_current",
            "atr.accel_diff",
            "atr.speed_boost",
            "booster.current",
        ]
    );
    assert_eq!(index, bytes.len());
}
