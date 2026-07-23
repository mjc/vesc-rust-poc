use super::wire::float_out_boy_realtime_push_float32_auto;
use super::wire::{
    float_out_boy_degrees, float_out_boy_realtime_push_u8, float_out_boy_realtime_push_u32,
    push_float_out_boy_float16,
};
use crate::domain::FloatOutBoyMode;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_DATA_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyChargingState, FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeTail, FloatOutBoyRunState,
};
#[cfg(test)]
use crate::domain::{FloatOutBoyRealtimeAlertMask, FloatOutBoyRealtimeReservedFlags};
#[cfg(test)]
use vescpkg_rs::prelude::{FirmwareFaultWireCode, TimestampTicks};

// Float Out Boy v1.2.1 `send_realtime_data` declares its fixed buffer at
// `third_party/float-out-boy/src/main.c:1267-1269`.
const FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// Float Out Boy v1.2.1 `cmd_realtime_data` declares its runtime-sized packet at
// `third_party/float-out-boy/src/main.c:1904-1906`.
const FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY: usize = 77;

/// Variable-length Float Out Boy `COMMAND_REALTIME_DATA` response bytes from
/// `third_party/float-out-boy/src/main.c:1904-1960`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) struct FloatOutBoyRealtimeDataResponse {
    bytes: [u8; FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY],
    len: usize,
}

impl FloatOutBoyRealtimeDataResponse {
    /// Return the encoded response bytes actually sent on the app-data wire.
    pub(in crate::package) fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }
}

#[inline(never)]
#[cfg(test)]
pub(in crate::package) fn encode_float_out_boy_get_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
) -> [u8; FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN] {
    encode_float_out_boy_get_realtime_data_response_with_remote(
        payloads,
        crate::domain::FloatOutBoyRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        0.0,
    )
}

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_get_realtime_data_response_with_remote(
    payloads: &FloatOutBoyAllDataPayloads,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: f32,
) -> [u8; FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN] {
    let mut bytes = [0; FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let footpad = base.footpad();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let motor = base.motor();

    // Upstream `on_command_received` dispatches `COMMAND_GET_RTDATA` to
    // `send_realtime_data` at `third_party/float-out-boy/src/main.c:2162-2164`; `send_realtime_data`
    // writes this legacy 72-byte payload at `third_party/float-out-boy/src/main.c:1267-1310`.
    // Its IMU fields are degree-valued because `imu_update` converts them at
    // `third_party/float-out-boy/src/imu.c:35-41`.
    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
    );
    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        FloatOutBoyAppDataCommand::GetRealtimeData.id(),
    );

    float_out_boy_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        base.balance_current().current().current().as_amps(),
    );
    float_out_boy_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        float_out_boy_degrees(attitude.balance_pitch().angle()),
    );
    float_out_boy_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        float_out_boy_degrees(attitude.roll().angle()),
    );

    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (ride_state.float_state_compat() & 0x0f) + (ride_state.setpoint_adjustment_compat() << 4),
    );
    let switch_state = footpad.state().switch_compat()
        | u8::from(matches!(ride_state.mode(), FloatOutBoyMode::HandTest)) << 3;
    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (switch_state & 0x0f) + (base.status().beep_reason().id() << 4),
    );
    float_out_boy_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc1_volts());
    float_out_boy_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc2_volts());

    [
        setpoints.board(),
        setpoints.atr(),
        setpoints.brake_tilt(),
        setpoints.torque_tilt(),
        setpoints.turn_tilt(),
        setpoints.remote(),
    ]
    .into_iter()
    .map(|setpoint| setpoint.angle().as_degrees())
    .for_each(|value| float_out_boy_realtime_push_float32_auto(&mut bytes, &mut ind, value));

    float_out_boy_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        float_out_boy_degrees(attitude.pitch().angle()),
    );
    // Upstream reads `d->motor.filt_current`, `d->atr.accel_diff`, and
    // `d->motor.dir_current` at `third_party/float-out-boy/src/main.c:1298-1306`.
    float_out_boy_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        motor.filtered_motor_current().current().current().as_amps(),
    );
    float_out_boy_realtime_push_float32_auto(&mut bytes, &mut ind, atr_accel_diff);
    if matches!(ride_state.charging(), FloatOutBoyChargingState::Charging) {
        float_out_boy_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        float_out_boy_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    } else {
        float_out_boy_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            base.booster_current().current().current().as_amps(),
        );
        float_out_boy_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            motor.directional_motor_current().current().as_amps(),
        );
    }
    float_out_boy_realtime_push_float32_auto(&mut bytes, &mut ind, remote_input.ratio().as_ratio());

    bytes
}

#[inline(never)]
#[cfg(test)]
pub(in crate::package) fn encode_float_out_boy_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
    system_timestamp: TimestampTicks,
) -> FloatOutBoyRealtimeDataResponse {
    encode_float_out_boy_realtime_data_response_with_runtime(
        payloads,
        FloatOutBoyRealtimeDataHeader::new(
            system_timestamp,
            payloads.base().status().ride_state(),
            payloads.base().footpad().state(),
            payloads.base().status().beep_reason(),
        ),
        FloatOutBoyRealtimeTail::new(
            FloatOutBoyRealtimeAlertMask::empty(),
            FloatOutBoyRealtimeReservedFlags::none(),
            FirmwareFaultWireCode::from_wire_code(0),
        ),
        crate::domain::FloatOutBoyRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        0.0,
        0.0,
    )
}

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_realtime_data_response_with_runtime(
    payloads: &FloatOutBoyAllDataPayloads,
    header: FloatOutBoyRealtimeDataHeader,
    tail: FloatOutBoyRealtimeTail,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: f32,
    atr_speed_boost: f32,
) -> FloatOutBoyRealtimeDataResponse {
    let mut bytes = [0; FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = matches!(ride_state.run_state(), FloatOutBoyRunState::Running);
    let charging = matches!(ride_state.charging(), FloatOutBoyChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `third_party/float-out-boy/src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
    );
    float_out_boy_realtime_push_u8(
        &mut bytes,
        &mut ind,
        FloatOutBoyAppDataCommand::RealtimeData.id(),
    );

    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.data_mask_compat());
    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.extra_flags_compat());
    // Upstream writes `d->time.now` at `third_party/float-out-boy/src/main.c:1931`; VESC timestamps are
    // represented as 100 us system ticks.
    float_out_boy_realtime_push_u32(&mut bytes, &mut ind, header.timestamp().as_ticks());

    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.state_byte_compat());
    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.footpad_flags_compat());
    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.stop_setpoint_byte_compat());
    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, header.beep_reason_compat());

    FLOAT_OUT_BOY_REALTIME_DATA_ITEMS
        .into_iter()
        .for_each(|item| {
            push_float_out_boy_float16(
                &mut bytes,
                &mut ind,
                realtime_value(
                    payloads,
                    item,
                    remote_input,
                    atr_accel_diff,
                    atr_speed_boost,
                ),
            )
        });
    if running {
        FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS
            .into_iter()
            .for_each(|item| {
                push_float_out_boy_float16(
                    &mut bytes,
                    &mut ind,
                    realtime_value(
                        payloads,
                        item,
                        remote_input,
                        atr_accel_diff,
                        atr_speed_boost,
                    ),
                );
            });
    }
    if charging {
        push_float_out_boy_float16(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        push_float_out_boy_float16(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    }

    float_out_boy_realtime_push_u32(
        &mut bytes,
        &mut ind,
        tail.active_alerts().active_alert_mask_compat(),
    );
    float_out_boy_realtime_push_u32(
        &mut bytes,
        &mut ind,
        tail.reserved_flags().extra_flags_compat(),
    );
    float_out_boy_realtime_push_u8(&mut bytes, &mut ind, tail.firmware_fault_code().wire_code());

    FloatOutBoyRealtimeDataResponse { bytes, len: ind }
}

fn realtime_value(
    payloads: &FloatOutBoyAllDataPayloads,
    item: FloatOutBoyRealtimeDataItem,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: f32,
    atr_speed_boost: f32,
) -> f32 {
    // C map: `cmd_realtime_data` expands `RT_DATA_ITEMS` and
    // `RT_DATA_RUNTIME_ITEMS` through `buffer_append_float16_auto` at
    // `third_party/float-out-boy/src/main.c:1943-1948`; the ID order is the string
    // list emitted at `third_party/float-out-boy/src/main.c:1876-1901`.
    let base = payloads.base();
    let motor = base.motor();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let temperatures = payloads.mode2().temperatures();

    match item {
        // Float Out Boy converts its internal m/s speed for the VESC Tool km/h
        // consumer at `third_party/float-out-boy/src/motor_data.c:119` and
        // `ui.qml.in:853-925`.
        FloatOutBoyRealtimeDataItem::MotorSpeed => {
            motor.vehicle_speed().speed().as_kilometers_per_hour()
        }
        FloatOutBoyRealtimeDataItem::MotorErpm => {
            motor.electrical_speed().rpm().as_revolutions_per_minute()
        }
        FloatOutBoyRealtimeDataItem::MotorCurrent => motor.motor_current().current().as_amps(),
        FloatOutBoyRealtimeDataItem::MotorDirectionalCurrent => {
            motor.directional_motor_current().current().as_amps()
        }
        FloatOutBoyRealtimeDataItem::MotorFilteredCurrent => {
            motor.filtered_motor_current().current().current().as_amps()
        }
        FloatOutBoyRealtimeDataItem::MotorDutyCycle => motor.duty_cycle().ratio().as_ratio(),
        FloatOutBoyRealtimeDataItem::MotorBatteryVoltage => {
            motor.battery_voltage().voltage().as_volts()
        }
        FloatOutBoyRealtimeDataItem::MotorBatteryCurrent => {
            motor.battery_current().current().as_amps()
        }
        FloatOutBoyRealtimeDataItem::MotorMosfetTemperature => {
            temperatures.mosfet().temperature().as_degrees_celsius()
        }
        FloatOutBoyRealtimeDataItem::MotorTemperature => {
            temperatures.motor().temperature().as_degrees_celsius()
        }
        FloatOutBoyRealtimeDataItem::ImuPitch => float_out_boy_degrees(attitude.pitch().angle()),
        FloatOutBoyRealtimeDataItem::ImuBalancePitch => {
            float_out_boy_degrees(attitude.balance_pitch().angle())
        }
        FloatOutBoyRealtimeDataItem::ImuRoll => float_out_boy_degrees(attitude.roll().angle()),
        FloatOutBoyRealtimeDataItem::FootpadAdc1 => base.footpad().adc1_volts(),
        FloatOutBoyRealtimeDataItem::FootpadAdc2 => base.footpad().adc2_volts(),
        // C map: `RT_DATA_ITEMS` includes `remote.input` at
        // `third_party/float-out-boy/src/rt_data.h:38-54`.
        FloatOutBoyRealtimeDataItem::RemoteInput => remote_input.ratio().as_ratio(),
        FloatOutBoyRealtimeDataItem::Setpoint => setpoints.board().angle().as_degrees(),
        FloatOutBoyRealtimeDataItem::AtrSetpoint => setpoints.atr().angle().as_degrees(),
        FloatOutBoyRealtimeDataItem::BrakeTiltSetpoint => {
            setpoints.brake_tilt().angle().as_degrees()
        }
        FloatOutBoyRealtimeDataItem::TorqueTiltSetpoint => {
            setpoints.torque_tilt().angle().as_degrees()
        }
        FloatOutBoyRealtimeDataItem::TurnTiltSetpoint => setpoints.turn_tilt().angle().as_degrees(),
        FloatOutBoyRealtimeDataItem::RemoteSetpoint => setpoints.remote().angle().as_degrees(),
        FloatOutBoyRealtimeDataItem::BalanceCurrent => {
            base.balance_current().current().current().as_amps()
        }
        // C map: runtime-only ATR fields are appended at
        // `third_party/float-out-boy/src/main.c:1946-1948`; the live values come
        // from the source-shaped `RideModifierState` refresh.
        FloatOutBoyRealtimeDataItem::AtrAccelDiff => atr_accel_diff,
        FloatOutBoyRealtimeDataItem::AtrSpeedBoost => atr_speed_boost,
        FloatOutBoyRealtimeDataItem::BoosterCurrent => {
            base.booster_current().current().current().as_amps()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::test_support::sample_all_data_payloads;
    use super::*;
    use crate::domain::{FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMotorPayload};
    use vescpkg_rs::prelude::{AngleDegrees, AngleRadians, Speed, TimestampTicks, VehicleSpeed};

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
        let legacy =
            encode_float_out_boy_get_realtime_data_response_with_remote(&payloads, input, 0.25);

        assert_f32_be(&legacy, 56, 0.25);
        assert_f32_be(&legacy, 68, 0.5);
        assert_eq!(
            realtime_value(
                &payloads,
                FloatOutBoyRealtimeDataItem::RemoteInput,
                input,
                0.0,
                0.0,
            ),
            0.5,
        );
    }

    #[test]
    fn float32_auto_zeros_small_normal_like_float_out_boy() {
        let value = 1.25e-38_f32;
        let mut bytes = [0xff; 4];
        let mut index = 0;

        float_out_boy_realtime_push_float32_auto(&mut bytes, &mut index, value);

        assert_eq!((value.is_normal(), index, bytes), (true, 4, [0; 4]));
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
}
