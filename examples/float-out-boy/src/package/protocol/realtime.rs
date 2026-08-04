use super::wire::float_out_boy_degrees;
use crate::domain::FloatOutBoyMode;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_DATA_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyChargingState, FloatOutBoyRealtimeAtrAccelerationDiff,
    FloatOutBoyRealtimeAtrSpeedBoost, FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeTail, FloatOutBoyRunState,
};
use crate::wire::FloatOutBoyPacket;
#[cfg(test)]
pub(in crate::package) use test_support::{
    encode_float_out_boy_get_realtime_data_response, encode_float_out_boy_realtime_data_response,
};

// Float Out Boy v1.2.1 `send_realtime_data` declares its fixed buffer at
// `third_party/float-out-boy/src/main.c:1267-1269`.
const FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// Float Out Boy v1.2.1 `cmd_realtime_data` declares its runtime-sized packet at
// `third_party/float-out-boy/src/main.c:1904-1906`.
const FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY: usize = 77;

/// Variable-length Float Out Boy `COMMAND_REALTIME_DATA` response bytes from
/// `third_party/float-out-boy/src/main.c:1904-1960`.
pub(in crate::package) type FloatOutBoyRealtimeDataResponse =
    FloatOutBoyPacket<FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY>;

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_get_realtime_data_response_with_remote(
    payloads: &FloatOutBoyAllDataPayloads,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff,
) -> [u8; FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN] {
    let mut packet = FloatOutBoyPacket::new();
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
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
    packet.push(FloatOutBoyAppDataCommand::GetRealtimeData.id());
    packet.push_float32_auto(base.balance_current().current().current().as_amps());
    packet.push_float32_auto(float_out_boy_degrees(attitude.balance_pitch().angle()));
    packet.push_float32_auto(float_out_boy_degrees(attitude.roll().angle()));

    packet.push(
        (ride_state.float_state_compat() & 0x0f) | (ride_state.setpoint_adjustment_compat() << 4),
    );
    let switch_state = footpad.state().switch_compat()
        | u8::from(matches!(ride_state.mode(), FloatOutBoyMode::HandTest)) << 3;
    packet.push((switch_state & 0x0f) | (base.status().beep_reason().id() << 4));
    packet.push_float32_auto(footpad.adc1_volts());
    packet.push_float32_auto(footpad.adc2_volts());

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
    .for_each(|value| packet.push_float32_auto(value));

    packet.push_float32_auto(float_out_boy_degrees(attitude.pitch().angle()));
    // Upstream reads `d->motor.filt_current`, `d->atr.accel_diff`, and
    // `d->motor.dir_current` at `third_party/float-out-boy/src/main.c:1298-1306`.
    packet.push_float32_auto(motor.filtered_motor_current().current().current().as_amps());
    packet.push_float32_auto(atr_accel_diff.as_erpm_delta());
    if matches!(ride_state.charging(), FloatOutBoyChargingState::Charging) {
        packet.push_float32_auto(payloads.mode4().current().current().as_amps());
        packet.push_float32_auto(payloads.mode4().voltage().voltage().as_volts());
    } else {
        packet.push_float32_auto(base.booster_current().current().current().as_amps());
        packet.push_float32_auto(motor.directional_motor_current().current().as_amps());
    }
    packet.push_float32_auto(remote_input.ratio().as_ratio());

    packet.into_bytes()
}

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_realtime_data_response_with_runtime(
    payloads: &FloatOutBoyAllDataPayloads,
    header: FloatOutBoyRealtimeDataHeader,
    tail: FloatOutBoyRealtimeTail,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff,
    atr_speed_boost: FloatOutBoyRealtimeAtrSpeedBoost,
) -> FloatOutBoyRealtimeDataResponse {
    let mut packet = FloatOutBoyPacket::new();
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = matches!(ride_state.run_state(), FloatOutBoyRunState::Running);
    let charging = matches!(ride_state.charging(), FloatOutBoyChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `third_party/float-out-boy/src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
    packet.push(FloatOutBoyAppDataCommand::RealtimeData.id());
    packet.push(header.data_mask_compat());
    packet.push(header.extra_flags_compat());
    // Upstream writes `d->time.now` at `third_party/float-out-boy/src/main.c:1931`; VESC timestamps are
    // represented as 100 us system ticks.
    packet.push_u32(header.timestamp().as_ticks());
    packet.push(header.state_byte_compat());
    packet.push(header.footpad_flags_compat());
    packet.push(header.stop_setpoint_byte_compat());
    packet.push(header.beep_reason_compat());

    for item in FLOAT_OUT_BOY_REALTIME_DATA_ITEMS {
        packet.push_float16_auto(realtime_value(
            payloads,
            item,
            remote_input,
            atr_accel_diff,
            atr_speed_boost,
        ));
    }
    if running {
        for item in FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS {
            packet.push_float16_auto(realtime_value(
                payloads,
                item,
                remote_input,
                atr_accel_diff,
                atr_speed_boost,
            ));
        }
    }
    if charging {
        packet.push_float16_auto(payloads.mode4().current().current().as_amps());
        packet.push_float16_auto(payloads.mode4().voltage().voltage().as_volts());
    }

    packet.push_u32(u32::from(tail.firmware_fault_active()));
    packet.push_u32(0);
    packet.push(tail.firmware_fault_code().wire_code());

    packet
}

pub(in crate::package) fn realtime_value(
    payloads: &FloatOutBoyAllDataPayloads,
    item: FloatOutBoyRealtimeDataItem,
    remote_input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff,
    atr_speed_boost: FloatOutBoyRealtimeAtrSpeedBoost,
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
        FloatOutBoyRealtimeDataItem::AtrAccelDiff => atr_accel_diff.as_erpm_delta(),
        FloatOutBoyRealtimeDataItem::AtrSpeedBoost => atr_speed_boost.as_units(),
        FloatOutBoyRealtimeDataItem::BoosterCurrent => {
            base.booster_current().current().current().as_amps()
        }
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
