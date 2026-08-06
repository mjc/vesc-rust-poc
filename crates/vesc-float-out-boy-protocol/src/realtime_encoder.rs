use crate::packet::FloatOutBoyPacket;
use crate::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_DATA_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyChargingState, FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeLiveValues,
    FloatOutBoyRealtimeTail, FloatOutBoyRunState, realtime_value,
};
use crate::{FloatOutBoyMode, degrees as float_out_boy_degrees};

// Float Out Boy v1.2.1 `send_realtime_data` declares its fixed buffer at
// `third_party/float-out-boy/src/main.c:1267-1269`.
const FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// The cutoff internal realtime packet is largest while both running and charging.
const FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY: usize = 83;

/// Variable-length Float Out Boy `COMMAND_REALTIME_DATA` response bytes from
/// `third_party/float-out-boy/src/main.c:1904-1960`.
/// Fixed-capacity FOB command-31 realtime response.
pub type FloatOutBoyRealtimeDataResponse =
    FloatOutBoyPacket<FLOAT_OUT_BOY_REALTIME_DATA_RESPONSE_CAPACITY>;

#[inline(never)]
/// Encode FOB's legacy realtime response with live remote and ATR input.
#[must_use]
pub fn encode_float_out_boy_get_realtime_data_response_with_remote(
    payloads: &FloatOutBoyAllDataPayloads,
    remote_input: crate::FloatOutBoyRealtimeRemoteInput,
    atr_accel_diff: f32,
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
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
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
    packet.push_float32_auto(atr_accel_diff);
    if matches!(ride_state.charging(), FloatOutBoyChargingState::Charging) {
        packet.push_float32_auto(payloads.mode4().current().current().as_amps());
        packet.push_float32_auto(payloads.mode4().voltage().voltage().as_volts());
    } else {
        packet.push_float32_auto(base.booster_torque().torque().as_newton_meters());
        packet.push_float32_auto(motor.directional_motor_current().current().as_amps());
    }
    packet.push_float32_auto(remote_input.ratio().as_ratio());

    packet.into_bytes()
}

#[inline(never)]
/// Encode FOB's command-31 realtime response from typed runtime values.
#[must_use]
pub fn encode_float_out_boy_realtime_data_response_with_runtime(
    payloads: &FloatOutBoyAllDataPayloads,
    header: FloatOutBoyRealtimeDataHeader,
    tail: FloatOutBoyRealtimeTail,
    live: FloatOutBoyRealtimeLiveValues,
) -> FloatOutBoyRealtimeDataResponse {
    let mut packet = FloatOutBoyPacket::new();
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = ride_state.run_state() == FloatOutBoyRunState::Running;
    let charging = matches!(ride_state.charging(), FloatOutBoyChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `third_party/float-out-boy/src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
    packet.push(FloatOutBoyAppDataCommand::RealtimeData.id());
    packet.push(header.data_mask_compat());
    packet.push(header.extra_flags_compat());
    // Upstream writes `d->time.now` at `third_party/float-out-boy/src/main.c:1931`; VESC timestamps are
    // represented as 100 us system ticks.
    packet.push_u32(header.timestamp().as_ticks());
    packet.push_u32(header.state_flags_compat());

    for item in FLOAT_OUT_BOY_REALTIME_DATA_ITEMS {
        packet.push_float16_auto(realtime_value(payloads, item, live));
    }
    if running {
        for item in FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS {
            packet.push_float16_auto(realtime_value(payloads, item, live));
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
