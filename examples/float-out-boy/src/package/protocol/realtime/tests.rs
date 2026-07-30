use super::super::super::test_support::sample_all_data_payloads;
use super::*;
use crate::domain::{
    FLOAT_OUT_BOY_REALTIME_SELECTED_RESPONSE_CAPACITY, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataMotorPayload, FloatOutBoyAllDataPayloads,
    FloatOutBoyRealtimeAtrAccelerationDiff, FloatOutBoyRealtimeAtrSpeedBoost,
    FloatOutBoyRealtimeAtrTransitionBoost, FloatOutBoyRealtimeControlFrequency,
    FloatOutBoyRealtimeControlPeriod, FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeLiveValues, FloatOutBoyRealtimeRemoteInput,
    FloatOutBoyRealtimeSelectedRequest, FloatOutBoyRealtimeTail,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, FirmwareFaultWireCode, SampleRate, SignedRatio, Speed,
    TimestampTicks, VehicleSpeed, VescSeconds,
};

fn live_values(
    remote: FloatOutBoyRealtimeRemoteInput,
    period: f32,
    frequency: f32,
    transition_boost: f32,
) -> FloatOutBoyRealtimeLiveValues {
    FloatOutBoyRealtimeLiveValues::new(
        FloatOutBoyRealtimeControlPeriod::new(VescSeconds::from_seconds(period)),
        FloatOutBoyRealtimeControlFrequency::new(SampleRate::from_hertz(frequency)),
        remote,
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
        FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0),
        FloatOutBoyRealtimeAtrTransitionBoost::from_factor(transition_boost),
    )
}

fn selected_request(flags: u8, mask1: u32, mask2: u32) -> FloatOutBoyRealtimeSelectedRequest {
    let mut payload = [0; 9];
    payload[0] = flags;
    payload[1..5].copy_from_slice(&mask1.to_be_bytes());
    payload[5..].copy_from_slice(&mask2.to_be_bytes());
    FloatOutBoyRealtimeSelectedRequest::parse(&payload).expect("selected request")
}

fn selected_header(payloads: &FloatOutBoyAllDataPayloads) -> FloatOutBoyRealtimeDataHeader {
    let base = payloads.base();
    FloatOutBoyRealtimeDataHeader::new(
        TimestampTicks::from_ticks(0x0102_0304),
        base.status().ride_state(),
        base.footpad().state(),
        base.status().beep_reason(),
    )
}

fn selected_live_values() -> FloatOutBoyRealtimeLiveValues {
    live_values(
        FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.5)),
        0.004,
        250.0,
        1.0,
    )
}

fn encode_float_out_boy_get_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
) -> [u8; 72] {
    encode_float_out_boy_get_realtime_data_response_with_remote(
        payloads,
        FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0)),
        0.0,
    )
}

fn encode_float_out_boy_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
    timestamp: TimestampTicks,
) -> vesc_float_out_boy_protocol::FloatOutBoyRealtimeDataResponse {
    encode_float_out_boy_realtime_data_response_with_runtime(
        payloads,
        FloatOutBoyRealtimeDataHeader::new(
            timestamp,
            payloads.base().status().ride_state(),
            payloads.base().footpad().state(),
            payloads.base().status().beep_reason(),
        ),
        FloatOutBoyRealtimeTail::new(false, FirmwareFaultWireCode::from_wire_code(0)),
        live_values(
            FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0)),
            0.002,
            500.0,
            1.0,
        ),
    )
}

fn sample_payloads_with_speed(meters_per_second: f32) -> FloatOutBoyAllDataPayloads {
    let payloads = sample_all_data_payloads();
    let base = payloads.base();
    let motor = base.motor();
    let motor = FloatOutBoyAllDataMotorPayload::new(
        motor.battery_voltage(),
        motor.electrical_speed(),
        VehicleSpeed::new(Speed::from_meters_per_second(meters_per_second)),
        motor.currents(),
        motor.duty_cycle(),
        motor.foc_id_current(),
    );
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_torque(),
        motor,
    );
    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

#[test]
fn app_data_processes_legacy_get_rtdata_like_float_out_boy() {
    let bytes = encode_float_out_boy_get_realtime_data_response(&sample_all_data_payloads());

    // Upstream dispatches `COMMAND_GET_RTDATA` at `third_party/float-out-boy/src/main.c:2162-2164`;
    // `send_realtime_data` writes this 72-byte response at
    // `third_party/float-out-boy/src/main.c:1267-1310`.
    assert_eq!(bytes.len(), 72);
    assert_eq!(&bytes[..2], &[101, 1]);
    assert_f32_be(&bytes, 2, 9.0);
    assert_f32_be(
        &bytes,
        6,
        AngleDegrees::from(AngleRadians::from_radians(1.2)).as_degrees(),
    );
    assert_f32_be(
        &bytes,
        10,
        AngleDegrees::from(AngleRadians::from_radians(-0.5)).as_degrees(),
    );
    assert_eq!(bytes[14], 0x21);
    assert_eq!(bytes[15], 0x12);
    assert_f32_be(&bytes, 16, 0.60);
    assert_f32_be(&bytes, 20, 0.40);
    assert_f32_be(&bytes, 24, 1.0);
    assert_f32_be(&bytes, 32, -1.0);
    assert_f32_be(&bytes, 44, 3.0);
    assert_f32_be(
        &bytes,
        48,
        AngleDegrees::from(AngleRadians::from_radians(2.3)).as_degrees(),
    );
    assert_f32_be(&bytes, 52, 5.0);
    assert_f32_be(&bytes, 56, 0.0);
    assert_f32_be(&bytes, 60, 4.0);
    assert_f32_be(&bytes, 64, 5.0);
    assert_f32_be(&bytes, 68, 0.0);
}

#[test]
fn booster_telemetry_reports_newton_meters_separately_from_motor_current() {
    let payloads = sample_all_data_payloads();
    let base = payloads.base();
    let remote = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0));

    assert_f32_eq!(base.booster_torque().torque().as_newton_meters(), 4.0);
    assert_f32_eq!(base.motor().motor_current().current().as_amps(), 5.0);
    assert_f32_eq!(
        realtime_value(
            &payloads,
            FloatOutBoyRealtimeDataItem::BoosterTorque,
            live_values(remote, 0.002, 500.0, 1.0),
        ),
        4.0,
    );
    let legacy = encode_float_out_boy_get_realtime_data_response(&payloads);
    assert_f32_be(&legacy, 60, 4.0);
}

#[test]
fn realtime_encoders_use_live_remote_input_like_float_out_boy() {
    let payloads = sample_all_data_payloads();
    let input = crate::domain::FloatOutBoyRealtimeRemoteInput::new(
        vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.5),
    );
    let legacy =
        encode_float_out_boy_get_realtime_data_response_with_remote(&payloads, input, 0.25);

    assert_f32_be(&legacy, 56, 0.25);
    assert_f32_be(&legacy, 68, 0.5);
    assert_f32_eq!(
        realtime_value(
            &payloads,
            FloatOutBoyRealtimeDataItem::RemoteInput,
            live_values(input, 0.002, 500.0, 1.0),
        ),
        0.5,
    );
}

#[test]
fn legacy_and_internal_realtime_encode_every_live_modifier_with_source_signs() {
    let payloads = sample_all_data_payloads();
    let legacy = encode_float_out_boy_get_realtime_data_response(&payloads);

    for (offset, expected) in [
        (24, 1.0),
        (28, 0.0),
        (32, -1.0),
        (36, 2.0),
        (40, -2.0),
        (44, 3.0),
    ] {
        assert_f32_be(&legacy, offset, expected);
    }

    let command_31 =
        encode_float_out_boy_realtime_data_response(&payloads, TimestampTicks::from_ticks(0));
    let bytes = command_31.as_bytes();
    assert!((decode_normal_float16([bytes[48], bytes[49]]) - 1.0).abs() < 0.001);
    assert_eq!(&bytes[50..52], &[0, 0]);
    assert!((decode_normal_float16([bytes[52], bytes[53]]) + 1.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[54], bytes[55]]) - 2.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[56], bytes[57]]) + 2.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[58], bytes[59]]) - 3.0).abs() < 0.001);
}

#[test]
fn float32_auto_zeros_small_normal_like_float_out_boy() {
    let value = 1.25e-38_f32;
    let mut packet = crate::wire::FloatOutBoyPacket::<4>::new();

    packet.push_float32_auto(value);

    assert_eq!((value.is_normal(), packet.into_bytes()), (true, [0; 4]));
}

#[test]
fn app_data_processes_non_running_realtime_data_like_float_out_boy_qml() {
    let response = encode_float_out_boy_realtime_data_response(
        &FloatOutBoyAllDataPayloads::source_startup(),
        TimestampTicks::from_ticks(0),
    );
    let bytes = response.as_bytes();

    // QML reads `c_REALTIME_DATA` at `ui.qml.in:853-925`; upstream
    // `cmd_realtime_data` writes this non-running packet shape at
    // `third_party/float-out-boy/src/main.c:1904-1960`.
    assert_eq!(bytes.len(), 57);
    assert_eq!(&bytes[..2], &[101, 31]);
    assert_eq!(bytes[2], 0x04);
    assert_eq!(bytes[3], 0);
    assert_eq!(&bytes[4..8], &[0, 0, 0, 0]);
    assert_eq!(&bytes[8..12], &[1, 0, 0, 0]);
    assert!((decode_normal_float16([bytes[12], bytes[13]]) - 0.002).abs() < 0.000_01);
    assert!((decode_normal_float16([bytes[14], bytes[15]]) - 500.0).abs() < 0.1);
    assert!(bytes[16..48].iter().all(|byte| *byte == 0));
    assert_eq!(&bytes[48..56], &[0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(bytes[56], 0);
}

#[test]
fn internal_realtime_encodes_typed_control_timing_and_atr_transition_boost() {
    let payloads = sample_all_data_payloads();
    let base = payloads.base();
    let response = encode_float_out_boy_realtime_data_response_with_runtime(
        &payloads,
        FloatOutBoyRealtimeDataHeader::new(
            TimestampTicks::from_ticks(7),
            base.status().ride_state(),
            base.footpad().state(),
            base.status().beep_reason(),
        ),
        FloatOutBoyRealtimeTail::new(false, FirmwareFaultWireCode::from_wire_code(0)),
        live_values(
            FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0)),
            0.004,
            250.0,
            1.75,
        ),
    );
    let bytes = response.as_bytes();

    assert!((decode_normal_float16([bytes[12], bytes[13]]) - 0.004).abs() < 0.000_01);
    assert!((decode_normal_float16([bytes[14], bytes[15]]) - 250.0).abs() < 0.1);
    assert!((decode_normal_float16([bytes[66], bytes[67]]) - 1.75).abs() < 0.001);
}

#[test]
fn selected_realtime_echoes_unknown_bits_with_an_exact_empty_response() {
    let payloads = sample_all_data_payloads();
    let response = encode_float_out_boy_realtime_selected_response(
        selected_request(0xfe, 1 << 2, 1 << 31),
        &payloads,
        selected_header(&payloads),
        selected_live_values(),
        None,
    );

    assert_eq!(
        response.as_bytes(),
        &[101, 33, 0xfe, 0, 0, 0, 4, 0x80, 0, 0, 0, 1, 2, 3, 4]
    );
}

#[test]
fn selected_realtime_float16_fields_follow_mask1_wire_order() {
    let payloads = sample_all_data_payloads();
    let header = selected_header(&payloads);
    let response = encode_float_out_boy_realtime_selected_response(
        selected_request(0, (1 << 0) | (1 << 1) | (1 << 6) | (1 << 14) | (1 << 30), 0),
        &payloads,
        header,
        selected_live_values(),
        None,
    );
    let bytes = response.as_bytes();

    assert_eq!(bytes.len(), 26);
    assert_eq!(bytes[15], header.extra_flags_compat());
    assert_eq!(&bytes[16..20], &header.state_flags_compat().to_be_bytes());
    assert!((decode_normal_float16([bytes[20], bytes[21]]) - 10.8).abs() < 0.01);
    assert!((decode_normal_float16([bytes[22], bytes[23]]) - 0.72).abs() < 0.001);
    assert!((decode_normal_float16([bytes[24], bytes[25]]) - 250.0).abs() < 0.1);
}

#[test]
fn selected_realtime_float32_fields_keep_mask1_width_and_order() {
    let payloads = sample_all_data_payloads();
    let response = encode_float_out_boy_realtime_selected_response(
        selected_request(1, (1 << 6) | (1 << 8) | (1 << 14), 0),
        &payloads,
        selected_header(&payloads),
        selected_live_values(),
        None,
    );
    let bytes = response.as_bytes();

    assert_eq!(bytes.len(), 27);
    assert_eq!(&bytes[15..19], &0x412c_cccc_u32.to_be_bytes());
    assert_f32_auto_be(bytes, 19, 5.0);
    assert_f32_auto_be(bytes, 23, 0.72);
}

#[test]
fn selected_realtime_mask2_keeps_odometer_integer_and_numeric_order() {
    let payloads = sample_all_data_payloads();
    let response = encode_float_out_boy_realtime_selected_response(
        selected_request(1, 0, 0x01ff),
        &payloads,
        selected_header(&payloads),
        selected_live_values(),
        None,
    );
    let bytes = response.as_bytes();

    assert_eq!(bytes.len(), 51);
    assert_eq!(&bytes[15..19], &123_456_u32.to_be_bytes());
    for (offset, expected) in [
        (19, 64.0),
        (23, 82.4),
        (27, 1.2),
        (31, 3.2),
        (35, 0.8),
        (39, 170.0),
        (43, 18.5),
        (47, 2.0),
    ] {
        assert_f32_auto_be(bytes, offset, expected);
    }
}

#[test]
fn selected_realtime_gnss_keeps_float64_coordinates_and_converts_speed() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let gnss = firmware.gnss().snapshot().expect("GNSS snapshot");
    let payloads = sample_all_data_payloads();
    let mask = 0x0000_7e00;

    for (flags, expected_len, scalar_width) in [(0, 41, 2), (1, 47, 4)] {
        let response = encode_float_out_boy_realtime_selected_response(
            selected_request(flags, 0, mask),
            &payloads,
            selected_header(&payloads),
            selected_live_values(),
            Some(gnss),
        );
        let bytes = response.as_bytes();

        assert_eq!(bytes.len(), expected_len);
        assert_eq!(&bytes[15..23], &40.0_f64.to_be_bytes());
        assert_eq!(&bytes[23..31], &(-105.0_f64).to_be_bytes());
        if scalar_width == 2 {
            assert!((decode_normal_float16([bytes[33], bytes[34]]) - 12.6).abs() < 0.01);
        } else {
            assert_eq!(&bytes[35..39], &0x4149_9999_u32.to_be_bytes());
        }
        assert_eq!(&bytes[expected_len - 4..], &42_u32.to_be_bytes());
    }
}

#[test]
fn selected_realtime_all_current_float32_fields_fit_the_exact_capacity() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let payloads = sample_all_data_payloads();
    let response = encode_float_out_boy_realtime_selected_response(
        selected_request(1, 0x7fff_ffc3, 0x0000_7fff),
        &payloads,
        selected_header(&payloads),
        selected_live_values(),
        Some(firmware.gnss().snapshot().expect("GNSS snapshot")),
    );

    assert_eq!(
        response.as_bytes().len(),
        FLOAT_OUT_BOY_REALTIME_SELECTED_RESPONSE_CAPACITY
    );
}

#[test]
fn command_31_motor_speed_encodes_kilometres_per_hour_like_float_out_boy() {
    let baseline = encode_float_out_boy_realtime_data_response(
        &sample_payloads_with_speed(0.0),
        TimestampTicks::from_ticks(0),
    );

    for (meters_per_second, expected) in [
        (0.0, [0x00, 0x00]),
        (1.0, [0x43, 0x33]),
        (-1.0, [0xc3, 0x33]),
        (0.5, [0x3f, 0x33]),
        (65_504.0 / 3.6, [0x7b, 0xff]),
    ] {
        let response = encode_float_out_boy_realtime_data_response(
            &sample_payloads_with_speed(meters_per_second),
            TimestampTicks::from_ticks(0),
        );
        let bytes = response.as_bytes();

        // C map: Float Out Boy converts m/s to km/h at
        // `third_party/float-out-boy/src/motor_data.c:119`; VESC Tool reads the
        // first command-31 data item at `ui.qml.in:853-925` as speed.
        assert_eq!(bytes.len(), baseline.as_bytes().len());
        assert_eq!(&bytes[..12], &baseline.as_bytes()[..12]);
        assert_eq!(&bytes[16..18], &expected);
        assert_eq!(&bytes[18..], &baseline.as_bytes()[18..]);
    }
}

#[test]
fn command_31_qml_visible_motor_speed_is_kilometres_per_hour() {
    let response = encode_float_out_boy_realtime_data_response(
        &sample_payloads_with_speed(1.0),
        TimestampTicks::from_ticks(0),
    );
    let bytes = response.as_bytes();
    let qml_value = decode_normal_float16([bytes[16], bytes[17]]);

    assert!((qml_value - 3.6).abs() < 0.001);
}

fn decode_normal_float16(bytes: [u8; 2]) -> f32 {
    let bits = u16::from_be_bytes(bytes);
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f) - 15;
    let significand = 1.0 + f32::from(bits & 0x03ff) / 1024.0;
    sign * significand * 2.0_f32.powi(exponent)
}

#[track_caller]
fn assert_f32_be(bytes: &[u8], offset: usize, expected: f32) {
    assert_eq!(
        u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]),
        expected.to_bits(),
    );
}

#[track_caller]
fn assert_f32_auto_be(bytes: &[u8], offset: usize, expected: f32) {
    assert_eq!(
        u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]),
        vescpkg_rs::protocol_buffer::float32_auto_bits(expected),
    );
}
