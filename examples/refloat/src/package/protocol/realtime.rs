use super::wire::refloat_realtime_push_float32_auto;
use super::wire::{
    push_refloat_float16, refloat_degrees, refloat_realtime_push_u8, refloat_realtime_push_u32,
};
use crate::domain::RefloatMode;
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RUNTIME_ITEMS,
    RefloatAllDataPayloads, RefloatAppDataCommand, RefloatChargingState, RefloatDarkRideState,
    RefloatRealtimeDataItem, RefloatRunState, RefloatWheelSlipState,
};
use vescpkg_rs::prelude::TimestampTicks;

// Refloat v1.2.1 `send_realtime_data` declares its fixed buffer at
// `third_party/refloat/src/main.c:1267-1269`.
const REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// Refloat v1.2.1 `cmd_realtime_data` declares its runtime-sized packet at
// `third_party/refloat/src/main.c:1904-1906`.
const REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY: usize = 77;

/// Variable-length Refloat `COMMAND_REALTIME_DATA` response bytes from
/// `third_party/refloat/src/main.c:1904-1960`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) struct RefloatRealtimeDataResponse {
    bytes: [u8; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY],
    len: usize,
}

impl RefloatRealtimeDataResponse {
    /// Return the encoded response bytes actually sent on the app-data wire.
    pub(in crate::package) fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }
}

#[inline(never)]
#[cfg(test)]
pub(in crate::package) fn encode_refloat_get_realtime_data_response(
    payloads: &RefloatAllDataPayloads,
) -> [u8; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN] {
    encode_refloat_get_realtime_data_response_with_remote(
        payloads,
        crate::domain::RefloatRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
    )
}

#[inline(never)]
pub(in crate::package) fn encode_refloat_get_realtime_data_response_with_remote(
    payloads: &RefloatAllDataPayloads,
    remote_input: crate::domain::RefloatRealtimeRemoteInput,
) -> [u8; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN] {
    let mut bytes = [0; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let footpad = base.footpad();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let motor = base.motor();

    // Upstream `on_command_received` dispatches `COMMAND_GET_RTDATA` to
    // `send_realtime_data` at `third_party/refloat/src/main.c:2162-2164`; `send_realtime_data`
    // writes this legacy 72-byte payload at `third_party/refloat/src/main.c:1267-1310`.
    // Its IMU fields are degree-valued because `imu_update` converts them at
    // `third_party/refloat/src/imu.c:35-41`.
    refloat_realtime_push_u8(&mut bytes, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        RefloatAppDataCommand::GetRealtimeData.id(),
    );

    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        base.balance_current().current().current().as_amps(),
    );
    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        refloat_degrees(attitude.balance_pitch().angle()),
    );
    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        refloat_degrees(attitude.roll().angle()),
    );

    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (ride_state.float_state_compat() & 0x0f) + (ride_state.setpoint_adjustment_compat() << 4),
    );
    let switch_state = footpad.state().switch_compat()
        | u8::from(matches!(ride_state.mode(), RefloatMode::HandTest)) << 3;
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (switch_state & 0x0f) + (base.status().beep_reason().id() << 4),
    );
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc1_volts());
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc2_volts());

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
    .for_each(|value| refloat_realtime_push_float32_auto(&mut bytes, &mut ind, value));

    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        refloat_degrees(attitude.pitch().angle()),
    );
    // Upstream reads `d->motor.filt_current`, `d->atr.accel_diff`, and
    // `d->motor.dir_current` at `third_party/refloat/src/main.c:1298-1306`.
    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        motor.filtered_motor_current().current().current().as_amps(),
    );
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, 0.0);
    if matches!(ride_state.charging(), RefloatChargingState::Charging) {
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    } else {
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            base.booster_current().current().current().as_amps(),
        );
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            motor.directional_motor_current().current().as_amps(),
        );
    }
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, remote_input.ratio().as_ratio());

    bytes
}

#[inline(never)]
#[cfg(test)]
pub(in crate::package) fn encode_refloat_realtime_data_response(
    payloads: &RefloatAllDataPayloads,
    system_timestamp: TimestampTicks,
) -> RefloatRealtimeDataResponse {
    encode_refloat_realtime_data_response_with_runtime(
        payloads,
        system_timestamp,
        crate::domain::RefloatRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        0.0,
        0.0,
    )
}

#[inline(never)]
pub(in crate::package) fn encode_refloat_realtime_data_response_with_runtime(
    payloads: &RefloatAllDataPayloads,
    system_timestamp: TimestampTicks,
    remote_input: crate::domain::RefloatRealtimeRemoteInput,
    atr_accel_diff: f32,
    atr_speed_boost: f32,
) -> RefloatRealtimeDataResponse {
    let mut bytes = [0; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = matches!(ride_state.run_state(), RefloatRunState::Running);
    let charging = matches!(ride_state.charging(), RefloatChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `third_party/refloat/src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
    refloat_realtime_push_u8(&mut bytes, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        RefloatAppDataCommand::RealtimeData.id(),
    );

    let mut mask = 0x04;
    if running {
        mask |= 0x01;
    }
    if charging {
        mask |= 0x02;
    }
    refloat_realtime_push_u8(&mut bytes, &mut ind, mask);

    // The data recorder and alert tracker are still part of the unported
    // control-loop/runtime state (`third_party/refloat/src/main.c:1927-1930`, `third_party/refloat/src/main.c:1956-1958`).
    refloat_realtime_push_u8(&mut bytes, &mut ind, 0);
    // Upstream writes `d->time.now` at `third_party/refloat/src/main.c:1931`; VESC timestamps are
    // represented as 100 us system ticks.
    refloat_realtime_push_u32(&mut bytes, &mut ind, system_timestamp.as_ticks());

    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        ride_state.mode().id() << 4 | ride_state.run_state().id(),
    );
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        base.footpad().state().id() << 6
            | u8::from(matches!(
                ride_state.charging(),
                RefloatChargingState::Charging
            )) << 5
            | u8::from(matches!(
                ride_state.darkride(),
                RefloatDarkRideState::Active
            )) << 1
            | u8::from(matches!(
                ride_state.wheelslip(),
                RefloatWheelSlipState::Detected
            )),
    );
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        ride_state.setpoint_adjustment().id() << 4 | ride_state.stop_condition().id(),
    );
    refloat_realtime_push_u8(&mut bytes, &mut ind, base.status().beep_reason().id());

    REFLOAT_REALTIME_DATA_ITEMS.into_iter().for_each(|item| {
        push_refloat_float16(
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
        REFLOAT_REALTIME_RUNTIME_ITEMS.into_iter().for_each(|item| {
            push_refloat_float16(
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
        push_refloat_float16(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        push_refloat_float16(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    }

    refloat_realtime_push_u32(&mut bytes, &mut ind, 0);
    refloat_realtime_push_u32(&mut bytes, &mut ind, 0);
    refloat_realtime_push_u8(&mut bytes, &mut ind, 0);

    RefloatRealtimeDataResponse { bytes, len: ind }
}

fn realtime_value(
    payloads: &RefloatAllDataPayloads,
    item: RefloatRealtimeDataItem,
    remote_input: crate::domain::RefloatRealtimeRemoteInput,
    atr_accel_diff: f32,
    atr_speed_boost: f32,
) -> f32 {
    // C map: `cmd_realtime_data` expands `RT_DATA_ITEMS` and
    // `RT_DATA_RUNTIME_ITEMS` through `buffer_append_float16_auto` at
    // `third_party/refloat/src/main.c:1943-1948`; the ID order is the string
    // list emitted at `third_party/refloat/src/main.c:1876-1901`.
    let base = payloads.base();
    let motor = base.motor();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let temperatures = payloads.mode2().temperatures();

    match item {
        // Refloat converts its internal m/s speed for the VESC Tool km/h
        // consumer at `third_party/refloat/src/motor_data.c:119` and
        // `ui.qml.in:853-925`.
        RefloatRealtimeDataItem::MotorSpeed => {
            motor.vehicle_speed().speed().as_kilometers_per_hour()
        }
        RefloatRealtimeDataItem::MotorErpm => {
            motor.electrical_speed().rpm().as_revolutions_per_minute()
        }
        RefloatRealtimeDataItem::MotorCurrent => motor.motor_current().current().as_amps(),
        RefloatRealtimeDataItem::MotorDirectionalCurrent => {
            motor.directional_motor_current().current().as_amps()
        }
        RefloatRealtimeDataItem::MotorFilteredCurrent => {
            motor.filtered_motor_current().current().current().as_amps()
        }
        RefloatRealtimeDataItem::MotorDutyCycle => motor.duty_cycle().ratio().as_ratio(),
        RefloatRealtimeDataItem::MotorBatteryVoltage => {
            motor.battery_voltage().voltage().as_volts()
        }
        RefloatRealtimeDataItem::MotorBatteryCurrent => motor.battery_current().current().as_amps(),
        RefloatRealtimeDataItem::MotorMosfetTemperature => {
            temperatures.mosfet().temperature().as_degrees_celsius()
        }
        RefloatRealtimeDataItem::MotorTemperature => {
            temperatures.motor().temperature().as_degrees_celsius()
        }
        RefloatRealtimeDataItem::ImuPitch => refloat_degrees(attitude.pitch().angle()),
        RefloatRealtimeDataItem::ImuBalancePitch => {
            refloat_degrees(attitude.balance_pitch().angle())
        }
        RefloatRealtimeDataItem::ImuRoll => refloat_degrees(attitude.roll().angle()),
        RefloatRealtimeDataItem::FootpadAdc1 => base.footpad().adc1_volts(),
        RefloatRealtimeDataItem::FootpadAdc2 => base.footpad().adc2_volts(),
        // C map: `RT_DATA_ITEMS` includes `remote.input` at
        // `third_party/refloat/src/rt_data.h:38-54`.
        RefloatRealtimeDataItem::RemoteInput => remote_input.ratio().as_ratio(),
        RefloatRealtimeDataItem::Setpoint => setpoints.board().angle().as_degrees(),
        RefloatRealtimeDataItem::AtrSetpoint => setpoints.atr().angle().as_degrees(),
        RefloatRealtimeDataItem::BrakeTiltSetpoint => setpoints.brake_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::TorqueTiltSetpoint => setpoints.torque_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::TurnTiltSetpoint => setpoints.turn_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::RemoteSetpoint => setpoints.remote().angle().as_degrees(),
        RefloatRealtimeDataItem::BalanceCurrent => {
            base.balance_current().current().current().as_amps()
        }
        // C map: runtime-only ATR fields are appended at
        // `third_party/refloat/src/main.c:1946-1948`; keep these explicit
        // placeholders until ATR runtime state is ported.
        RefloatRealtimeDataItem::AtrAccelDiff => atr_accel_diff,
        RefloatRealtimeDataItem::AtrSpeedBoost => atr_speed_boost,
        RefloatRealtimeDataItem::BoosterCurrent => {
            base.booster_current().current().current().as_amps()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::test_support::sample_all_data_payloads;
    use super::*;
    use crate::domain::{RefloatAllDataBasePayload, RefloatAllDataMotorPayload};
    use vescpkg_rs::prelude::{AngleDegrees, AngleRadians, Speed, TimestampTicks, VehicleSpeed};

    fn sample_payloads_with_speed(meters_per_second: f32) -> RefloatAllDataPayloads {
        let payloads = sample_all_data_payloads();
        let base = payloads.base();
        let motor = base.motor();
        let motor = RefloatAllDataMotorPayload::new(
            motor.battery_voltage(),
            motor.electrical_speed(),
            VehicleSpeed::new(Speed::from_meters_per_second(meters_per_second)),
            motor.currents(),
            motor.duty_cycle(),
            motor.foc_id_current(),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            motor,
        );
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
    }

    #[test]
    fn app_data_processes_legacy_get_rtdata_like_refloat() {
        let bytes = encode_refloat_get_realtime_data_response(&sample_all_data_payloads());

        // Upstream dispatches `COMMAND_GET_RTDATA` at `third_party/refloat/src/main.c:2162-2164`;
        // `send_realtime_data` writes this 72-byte response at
        // `third_party/refloat/src/main.c:1267-1310`.
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
    fn realtime_encoders_use_live_remote_input_like_refloat() {
        let payloads = sample_all_data_payloads();
        let input = crate::domain::RefloatRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.5),
        );
        let legacy = encode_refloat_get_realtime_data_response_with_remote(&payloads, input);

        assert_f32_be(&legacy, 68, 0.5);
        assert_eq!(
            realtime_value(
                &payloads,
                RefloatRealtimeDataItem::RemoteInput,
                input,
                0.0,
                0.0,
            ),
            0.5,
        );
    }

    #[test]
    fn float32_auto_zeros_small_normal_like_refloat() {
        let value = 1.25e-38_f32;
        let mut bytes = [0xff; 4];
        let mut index = 0;

        refloat_realtime_push_float32_auto(&mut bytes, &mut index, value);

        assert_eq!((value.is_normal(), index, bytes), (true, 4, [0; 4]));
    }

    #[test]
    fn app_data_processes_non_running_realtime_data_like_refloat_qml() {
        let response = encode_refloat_realtime_data_response(
            &RefloatAllDataPayloads::source_startup(),
            TimestampTicks::from_ticks(0),
        );
        let bytes = response.as_bytes();

        // QML reads `c_REALTIME_DATA` at `ui.qml.in:853-925`; upstream
        // `cmd_realtime_data` writes this non-running packet shape at
        // `third_party/refloat/src/main.c:1904-1960`.
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
    fn command_31_motor_speed_encodes_kilometres_per_hour_like_refloat() {
        let baseline = encode_refloat_realtime_data_response(
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
            let response = encode_refloat_realtime_data_response(
                &sample_payloads_with_speed(meters_per_second),
                TimestampTicks::from_ticks(0),
            );
            let bytes = response.as_bytes();

            // C map: Refloat converts m/s to km/h at
            // `third_party/refloat/src/motor_data.c:119`; VESC Tool reads the
            // first command-31 data item at `ui.qml.in:853-925` as speed.
            assert_eq!(bytes.len(), baseline.as_bytes().len());
            assert_eq!(&bytes[..12], &baseline.as_bytes()[..12]);
            assert_eq!(&bytes[12..14], &expected);
            assert_eq!(&bytes[14..], &baseline.as_bytes()[14..]);
        }
    }

    #[test]
    fn command_31_qml_visible_motor_speed_is_kilometres_per_hour() {
        let response = encode_refloat_realtime_data_response(
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
