//! Refloat compact all-data wire helpers.
//!
//! C map: `cmd_all_data` writes compact all-data packets at
//! `third_party/refloat/src/main.c:1313-1399`; the helpers here own the raw
//! byte/scaled-value boundary for that packet family.

use super::{RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload};
use vescpkg_rs::prelude::AngleRadians;

pub(super) fn refloat_append_all_data_mode2(
    buffer: &mut [u8],
    ind: &mut usize,
    mode2: RefloatAllDataMode2Payload,
) {
    // C map: mode >= 2 appends distance, motor temperatures, and the zero
    // battery-temperature placeholder at `third_party/refloat/src/main.c:1373-1379`.
    refloat_push_float32_auto(buffer, ind, mode2.distance_abs().distance().as_meters());
    refloat_push_u8(
        buffer,
        ind,
        refloat_nonnegative_scaled_u8(
            mode2
                .temperatures()
                .mosfet()
                .temperature()
                .as_degrees_celsius(),
            2.0,
        ),
    );
    refloat_push_u8(
        buffer,
        ind,
        refloat_nonnegative_scaled_u8(
            mode2
                .temperatures()
                .motor()
                .temperature()
                .as_degrees_celsius(),
            2.0,
        ),
    );
    refloat_push_u8(
        buffer,
        ind,
        mode2.battery_temperature().as_measured().map_or(0, |temp| {
            refloat_nonnegative_scaled_u8(temp.as_degrees_celsius(), 2.0)
        }),
    );
}

pub(super) fn refloat_append_all_data_mode3(
    buffer: &mut [u8],
    ind: &mut usize,
    mode3: RefloatAllDataMode3Payload,
) {
    // C map: mode >= 3 appends odometer, Ah/Wh totals, and battery level at
    // `third_party/refloat/src/main.c:1381-1389`.
    refloat_push_u32(buffer, ind, mode3.odometer().as_meters() as u32);
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.discharged_charge().charge().as_amp_hours(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.charged_charge().charge().as_amp_hours(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.discharged_energy().energy().as_watt_hours(),
        1.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode3.charged_energy().energy().as_watt_hours(),
        1.0,
    );
    refloat_push_u8(
        buffer,
        ind,
        refloat_scaled_u8(mode3.battery_level().as_fraction().min(1.25), 200.0),
    );
}

pub(super) fn refloat_append_all_data_mode4(
    buffer: &mut [u8],
    ind: &mut usize,
    mode4: RefloatAllDataMode4Payload,
) {
    // C map: mode >= 4 appends charging current and voltage at
    // `third_party/refloat/src/main.c:1391-1395`.
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode4.current().current().current().as_amps(),
        10.0,
    );
    refloat_push_scaled_i16(
        buffer,
        ind,
        mode4.voltage().voltage().voltage().as_volts(),
        10.0,
    );
}

pub(super) fn refloat_push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    crate::wire::push_u8(buffer, ind, value);
}

pub(super) fn refloat_push_i16(buffer: &mut [u8], ind: &mut usize, value: i16) {
    // C map: `buffer_append_i16` writes big-endian signed integers via the
    // same bounded byte-by-byte helper path.
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| refloat_push_u8(buffer, ind, byte));
}

fn refloat_push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    crate::wire::push_u32(buffer, ind, value);
}

fn refloat_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    crate::wire::push_float32_auto(buffer, ind, value);
}

pub(super) fn refloat_degrees(angle: AngleRadians) -> f32 {
    // C map: compact realtime/all-data packets emit angles in degrees at
    // `third_party/refloat/src/main.c:1328-1399` and `third_party/refloat/src/main.c:1881-1930`.
    crate::wire::degrees(angle)
}

pub(super) fn refloat_push_scaled_i16(buffer: &mut [u8], ind: &mut usize, value: f32, scale: f32) {
    // C map: compact all-data uses direct scale/cast wire encodings at
    // `third_party/refloat/src/main.c:1328-1395`; callers keep unit conversion
    // at this packet boundary.
    refloat_push_i16(buffer, ind, (value * scale) as i16);
}

pub(super) fn refloat_scaled_u8(value: f32, scale: f32) -> u8 {
    // C map: packet helpers use direct scale/cast encoding for compact
    // integer fields at `third_party/refloat/src/main.c:1328-1399`.
    (value * scale) as u8
}

fn refloat_nonnegative_scaled_u8(value: f32, scale: f32) -> u8 {
    // C map: zero-clamp the temperature and battery placeholders before the
    // compact packet cast at `third_party/refloat/src/main.c:1373-1395`.
    refloat_scaled_u8(value.max(0.0), scale)
}

pub(super) fn refloat_offset_scaled_u8(value: f32, scale: f32, offset: f32) -> u8 {
    // C map: compact packet helpers add a fixed offset before the integer cast
    // at `third_party/refloat/src/main.c:1241-1399`.
    (value * scale + offset) as u8
}
