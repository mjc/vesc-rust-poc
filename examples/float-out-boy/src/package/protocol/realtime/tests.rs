use super::super::super::test_support::sample_all_data_payloads;
use super::*;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMotorPayload, FloatOutBoyAllDataStatus,
    FloatOutBoyChargingState,
};
use vescpkg_rs::{
    FirmwareFaultWireCode,
    prelude::{AngleDegrees, AngleRadians, Speed, TimestampTicks, VehicleSpeed},
};

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
        base.booster_current(),
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
fn realtime_encoders_use_live_remote_input_like_float_out_boy() {
    let payloads = sample_all_data_payloads();
    let input = crate::domain::FloatOutBoyRealtimeRemoteInput::new(
        vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.5),
    );
    let legacy = encode_float_out_boy_get_realtime_data_response_with_remote(
        &payloads,
        input,
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.25),
    );

    assert_f32_be(&legacy, 56, 0.25);
    assert_f32_be(&legacy, 68, 0.5);
    assert_f32_eq!(
        realtime_value(
            &payloads,
            FloatOutBoyRealtimeDataItem::RemoteInput,
            input,
            FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
            FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0),
        ),
        0.5,
    );
}

#[test]
fn legacy_and_command_31_encode_every_live_modifier_with_source_signs() {
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
    assert!((decode_normal_float16([bytes[44], bytes[45]]) - 1.0).abs() < 0.001);
    assert_eq!(&bytes[46..48], &[0, 0]);
    assert!((decode_normal_float16([bytes[48], bytes[49]]) + 1.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[50], bytes[51]]) - 2.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[52], bytes[53]]) + 2.0).abs() < 0.001);
    assert!((decode_normal_float16([bytes[54], bytes[55]]) - 3.0).abs() < 0.001);
}

#[test]
fn command_31_running_charging_preserves_the_fault_tail_at_capacity() {
    let payloads = sample_all_data_payloads();
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_charging(FloatOutBoyChargingState::Charging);
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let payloads = payloads.with_base(base);
    let response = encode_float_out_boy_realtime_data_response_with_runtime(
        &payloads,
        FloatOutBoyRealtimeDataHeader::new(
            TimestampTicks::from_ticks(0),
            ride_state,
            payloads.base().footpad().state(),
            payloads.base().status().beep_reason(),
        ),
        FloatOutBoyRealtimeTail::new(true, FirmwareFaultWireCode::from_wire_code(0x2a)),
        crate::domain::FloatOutBoyRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
        FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0),
    );

    assert_eq!(response.len(), 77);
    assert_eq!(response.as_bytes().len(), response.len());
    assert_eq!(&response.as_bytes()[68..72], &[0, 0, 0, 1]);
    assert_eq!(&response.as_bytes()[72..76], &[0, 0, 0, 0]);
    assert_eq!(response.as_bytes()[76], 0x2a);
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
        &FloatOutBoyAllDataPayloads::default(),
        TimestampTicks::from_ticks(0),
    );
    let bytes = response.as_bytes();

    // QML reads `c_REALTIME_DATA` at `ui.qml.in:853-925`; upstream
    // `cmd_realtime_data` writes this non-running packet shape at
    // `third_party/float-out-boy/src/main.c:1904-1960`.
    assert_eq!(bytes.len(), 53);
    assert_eq!(&bytes[..2], &[101, 31]);
    assert_eq!(bytes[2], 0x04);
    assert_eq!(bytes[3], 0);
    assert_eq!(&bytes[4..8], &[0, 0, 0, 0]);
    assert_eq!(bytes[8], 1);
    assert_eq!(bytes[9], 0);
    assert_eq!(bytes[10], 0);
    assert_eq!(bytes[11], 0);
    assert!(bytes[12..44].iter().all(|byte| *byte == 0));
    assert_eq!(&bytes[44..48], &[0, 0, 0, 0]);
    assert_eq!(&bytes[48..52], &[0, 0, 0, 0]);
    assert_eq!(bytes[52], 0);
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
        assert_eq!(&bytes[12..14], &expected);
        assert_eq!(&bytes[14..], &baseline.as_bytes()[14..]);
    }
}

#[test]
fn command_31_qml_visible_motor_speed_is_kilometres_per_hour() {
    let response = encode_float_out_boy_realtime_data_response(
        &sample_payloads_with_speed(1.0),
        TimestampTicks::from_ticks(0),
    );
    let bytes = response.as_bytes();
    let qml_value = decode_normal_float16([bytes[12], bytes[13]]);

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
