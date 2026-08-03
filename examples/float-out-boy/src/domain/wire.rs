//! Float Out Boy compact all-data wire helpers.
//!
//! C map: `cmd_all_data` writes compact all-data packets at
//! `third_party/float-out-boy/src/main.c:1313-1399`; the helpers here own the raw
//! byte/scaled-value boundary for that packet family.

use super::{
    FloatOutBoyAllDataMode2Payload, FloatOutBoyAllDataMode3Payload, FloatOutBoyAllDataMode4Payload,
};
use crate::wire::FloatOutBoyPacket;
use vescpkg_rs::prelude::AngleRadians;

pub(super) fn float_out_boy_append_all_data_mode2<const N: usize>(
    packet: &mut FloatOutBoyPacket<N>,
    mode2: FloatOutBoyAllDataMode2Payload,
) {
    // C map: mode >= 2 appends distance, motor temperatures, and the zero
    // battery-temperature placeholder at `third_party/float-out-boy/src/main.c:1373-1379`.
    packet.push_float32_auto(mode2.distance_abs().distance().as_meters());
    packet.push(float_out_boy_nonnegative_scaled_u8(
        mode2
            .temperatures()
            .mosfet()
            .temperature()
            .as_degrees_celsius(),
        2.0,
    ));
    packet.push(float_out_boy_nonnegative_scaled_u8(
        mode2
            .temperatures()
            .motor()
            .temperature()
            .as_degrees_celsius(),
        2.0,
    ));
    packet.push(mode2.battery_temperature().as_measured().map_or(0, |temp| {
        float_out_boy_nonnegative_scaled_u8(temp.as_degrees_celsius(), 2.0)
    }));
}

pub(super) fn float_out_boy_append_all_data_mode3<const N: usize>(
    packet: &mut FloatOutBoyPacket<N>,
    mode3: FloatOutBoyAllDataMode3Payload,
) {
    // C map: mode >= 3 appends odometer, Ah/Wh totals, and battery level at
    // `third_party/float-out-boy/src/main.c:1381-1389`.
    packet.push_u32(crate::wire::truncating_u64_to_u32(
        mode3.odometer().as_meters(),
    ));
    packet.push_scaled_i16(mode3.discharged_charge().charge().as_amp_hours(), 10.0);
    packet.push_scaled_i16(mode3.charged_charge().charge().as_amp_hours(), 10.0);
    packet.push_scaled_i16(mode3.discharged_energy().energy().as_watt_hours(), 1.0);
    packet.push_scaled_i16(mode3.charged_energy().energy().as_watt_hours(), 1.0);
    packet.push(float_out_boy_scaled_u8(
        mode3.battery_level().as_fraction().min(1.25),
        200.0,
    ));
}

pub(super) fn float_out_boy_append_all_data_mode4<const N: usize>(
    packet: &mut FloatOutBoyPacket<N>,
    mode4: FloatOutBoyAllDataMode4Payload,
) {
    // C map: mode >= 4 appends charging current and voltage at
    // `third_party/float-out-boy/src/main.c:1391-1395`.
    packet.push_scaled_i16(mode4.current().current().current().as_amps(), 10.0);
    packet.push_scaled_i16(mode4.voltage().voltage().voltage().as_volts(), 10.0);
}

pub(super) fn float_out_boy_degrees(angle: AngleRadians) -> f32 {
    // C map: compact realtime/all-data packets emit angles in degrees at
    // `third_party/float-out-boy/src/main.c:1328-1399` and `third_party/float-out-boy/src/main.c:1881-1930`.
    crate::wire::degrees(angle)
}

pub(super) fn float_out_boy_scaled_u8(value: f32, scale: f32) -> u8 {
    // C map: packet helpers use direct scale/cast encoding for compact
    // integer fields at `third_party/float-out-boy/src/main.c:1328-1399`.
    crate::wire::saturating_trunc_f32_to_u8(value * scale)
}

fn float_out_boy_nonnegative_scaled_u8(value: f32, scale: f32) -> u8 {
    // C map: zero-clamp the temperature and battery placeholders before the
    // compact packet cast at `third_party/float-out-boy/src/main.c:1373-1395`.
    float_out_boy_scaled_u8(value.max(0.0), scale)
}

pub(super) fn float_out_boy_offset_scaled_u8(value: f32, scale: f32, offset: f32) -> u8 {
    // C map: compact packet helpers add a fixed offset before the integer cast
    // at `third_party/float-out-boy/src/main.c:1241-1399`.
    crate::wire::saturating_trunc_f32_to_u8(value * scale + offset)
}
