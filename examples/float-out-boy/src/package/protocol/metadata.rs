use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use vescpkg_rs::prelude::SYSTEM_TICK_RATE_HZ;

// Float Out Boy v1.2.1 `cmd_info` writes this version-2 response shape at
// `third_party/float-out-boy/src/main.c:2070-2139`.
pub(super) const FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN: usize = 60;
// Float Out Boy v1.2.1 `cmd_realtime_data_ids` writes the counted ID-list packet at
// `third_party/float-out-boy/src/main.c:1876-1901`.
pub(super) const FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN: usize = 405;

const FLOAT_OUT_BOY_PACKAGE_NAME: &[u8] = b"Float Out Boy";
const FLOAT_OUT_BOY_VERSION_SUFFIX: &[u8] = b"";
const FLOAT_OUT_BOY_MAJOR_VERSION: u8 = 0;
const FLOAT_OUT_BOY_MINOR_VERSION: u8 = 1;
const FLOAT_OUT_BOY_PATCH_VERSION: u8 = 0;
const FLOAT_OUT_BOY_BUILD_NUMBER: u8 = 0;
const FLOAT_OUT_BOY_GIT_HASH: u32 = 0x0ef6_e99d;
const FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ: u32 =
    crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ);

// Float Out Boy C builds this exact packet in `third_party/float-out-boy/src/main.c:1876-1901`, using the ID
// order from `third_party/float-out-boy/src/rt_data.h:38-66` and counted-string framing from
// `third_party/float-out-boy/src/conf/buffer.c:147-155`. QML reads the same two string lists in
// `ui.qml.in:926-934`.
// Keep the materialized bytes in the loaded extension image so hardware never
// has to dereference string-literal storage.
vescpkg_rs::firmware_section_static!(
    ".text.float_out_boy_realtime_data_ids",
    static FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_BYTES: [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN] =
        // Each leading byte is the following ASCII identifier's length. Keeping
        // the complete packet as a compile-time byte string eliminates runtime
        // construction and every potentially panicking indexed write.
        *b"\x65\x20\
            \x10\x0bmotor.speed\
            \x0amotor.erpm\
            \x0dmotor.current\
            \x11motor.dir_current\
            \x12motor.filt_current\
            \x10motor.duty_cycle\
            \x12motor.batt_voltage\
            \x12motor.batt_current\
            \x11motor.mosfet_temp\
            \x10motor.motor_temp\
            \x09imu.pitch\
            \x11imu.balance_pitch\
            \x08imu.roll\
            \x0cfootpad.adc1\
            \x0cfootpad.adc2\
            \x0cremote.input\
            \x0a\x08setpoint\
            \x0catr.setpoint\
            \x13brake_tilt.setpoint\
            \x14torque_tilt.setpoint\
            \x12turn_tilt.setpoint\
            \x0fremote.setpoint\
            \x0fbalance_current\
            \x0eatr.accel_diff\
            \x0fatr.speed_boost\
            \x0fbooster.current"
);

pub(in crate::package) struct FloatOutBoyInfoResponse {
    bytes: [u8; FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN],
    len: usize,
}

impl FloatOutBoyInfoResponse {
    pub(in crate::package) fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }
}

pub(in crate::package) fn encode_float_out_boy_info_response(
    request_payload: &[u8],
    hardware_led_mode: u8,
    internal_leds_operational: bool,
    data_recorder_capable: bool,
) -> FloatOutBoyInfoResponse {
    let version = request_payload.first().copied().unwrap_or(1);
    let mut bytes = [0; FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN];
    let mut index = 0;
    float_out_boy_response_push_u8(
        &mut bytes,
        &mut index,
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
    );
    float_out_boy_response_push_u8(&mut bytes, &mut index, FloatOutBoyAppDataCommand::Info.id());
    if version == 1 {
        float_out_boy_response_push_u8(
            &mut bytes,
            &mut index,
            FLOAT_OUT_BOY_MAJOR_VERSION * 10 + FLOAT_OUT_BOY_MINOR_VERSION,
        );
        float_out_boy_response_push_u8(&mut bytes, &mut index, FLOAT_OUT_BOY_BUILD_NUMBER);
        let legacy_led_type = if hardware_led_mode & 0x2 == 0 {
            hardware_led_mode
        } else {
            3
        };
        float_out_boy_response_push_u8(&mut bytes, &mut index, legacy_led_type);
        return FloatOutBoyInfoResponse { bytes, len: index };
    }

    // Unknown versions use the highest known response with flags cleared,
    // matching upstream's `default` arm.
    let flags = match request_payload {
        [2, flags, ..] => *flags,
        _ => 0,
    };
    float_out_boy_response_push_u8(&mut bytes, &mut index, 2);
    float_out_boy_response_push_u8(&mut bytes, &mut index, flags);
    append_fixed_ascii::<20>(&mut bytes, &mut index, FLOAT_OUT_BOY_PACKAGE_NAME);
    float_out_boy_response_push_u8(&mut bytes, &mut index, FLOAT_OUT_BOY_MAJOR_VERSION);
    float_out_boy_response_push_u8(&mut bytes, &mut index, FLOAT_OUT_BOY_MINOR_VERSION);
    float_out_boy_response_push_u8(&mut bytes, &mut index, FLOAT_OUT_BOY_PATCH_VERSION);
    append_fixed_ascii::<20>(&mut bytes, &mut index, FLOAT_OUT_BOY_VERSION_SUFFIX);
    float_out_boy_response_push_bytes(
        &mut bytes,
        &mut index,
        &FLOAT_OUT_BOY_GIT_HASH.to_be_bytes(),
    );
    float_out_boy_response_push_bytes(
        &mut bytes,
        &mut index,
        &FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ.to_be_bytes(),
    );
    // Upstream derives capabilities from data-recorder and LED config at
    // `third_party/float-out-boy/src/main.c:2121-2132`. Bit 0 is the LED
    // surface and bit 1 is the external LCM surface.
    let external_leds_operational = hardware_led_mode & 0x2 != 0;
    let mut capabilities = u32::from(internal_leds_operational || external_leds_operational)
        | (u32::from(external_leds_operational) << 1);
    capabilities |= u32::from(data_recorder_capable) << 31;
    float_out_boy_response_push_bytes(&mut bytes, &mut index, &capabilities.to_be_bytes());
    // Upstream currently sends zero `extra_flags` at `third_party/float-out-boy/src/main.c:2134-2135`.
    float_out_boy_response_push_u8(&mut bytes, &mut index, 0);
    FloatOutBoyInfoResponse { bytes, len: index }
}

fn append_fixed_ascii<const LEN: usize>(bytes: &mut [u8], index: &mut usize, value: &[u8]) {
    // C map: `buffer_append_string_fixed` copies up to the fixed width, then
    // zero-pads at `third_party/float-out-boy/src/conf/buffer.c:169-181`.
    let start = *index;
    for (offset, byte) in value.iter().copied().take(LEN).enumerate() {
        if let Some(slot) = bytes.get_mut(start.saturating_add(offset)) {
            *slot = byte;
        }
    }
    *index = start.saturating_add(LEN);
}

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_realtime_data_ids_response()
-> [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN] {
    // C map: `cmd_realtime_data_ids` builds a local `uint8_t buffer[bufsize]`
    // and sends it with `SEND_APP_DATA` at `third_party/float-out-boy/src/main.c:1876-1901`.
    // Return owned bytes so the firmware copy reads callback-stack storage, not
    // a package static through an extra firmware boundary.
    FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_BYTES
}

fn float_out_boy_response_push_bytes(bytes: &mut [u8], index: &mut usize, values: &[u8]) {
    crate::wire::push_bytes(bytes, index, values);
}

fn float_out_boy_response_push_u8(bytes: &mut [u8], index: &mut usize, value: u8) {
    crate::wire::push_u8(bytes, index, value);
}

#[cfg(test)]
mod tests;
