use super::packet::FloatOutBoyPacket;
use super::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use vescpkg_rs::prelude::SYSTEM_TICK_RATE_HZ;

// Float Out Boy v1.2.1 `cmd_info` writes this version-2 response shape at
// `third_party/float-out-boy/src/main.c:2070-2139`.
/// Length of the FOB version-2 package-info response.
pub const FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN: usize = 60;
// The pinned cutoff writes this counted realtime-data ID-list packet.
/// Length of the FOB realtime-data ID-list response.
pub const FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN: usize = 370;

const FLOAT_OUT_BOY_PACKAGE_NAME: &[u8] = b"Float Out Boy";
const FLOAT_OUT_BOY_VERSION_SUFFIX: &[u8] = b"";
const FLOAT_OUT_BOY_MAJOR_VERSION: u8 = 0;
const FLOAT_OUT_BOY_MINOR_VERSION: u8 = 1;
const FLOAT_OUT_BOY_PATCH_VERSION: u8 = 0;
const FLOAT_OUT_BOY_BUILD_NUMBER: u8 = 0;
const FLOAT_OUT_BOY_GIT_HASH: u32 = 0x0ef6_e99d;
const FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ: u32 =
    super::packet::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ);

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
            \x12\x0acontrol.dt\
            \x0ccontrol.freq\
            \x05speed\
            \x04erpm\
            \x07current\
            \x0bdir_current\
            \x0cfilt_current\
            \x0aduty_cycle\
            \x0cbatt_voltage\
            \x0cbatt_current\
            \x0bmosfet_temp\
            \x0amotor_temp\
            \x05pitch\
            \x0dbalance_pitch\
            \x04roll\
            \x08adc_left\
            \x09adc_right\
            \x0cremote.input\
            \x0b\x08setpoint\
            \x0catr.setpoint\
            \x13brake_tilt.setpoint\
            \x14torque_tilt.setpoint\
            \x12turn_tilt.setpoint\
            \x0fremote.setpoint\
            \x0fbalance_current\
            \x0eatr.accel_diff\
            \x0fatr.speed_boost\
            \x14atr.transition_boost\
            \x0ebooster.torque"
);

/// Fixed-capacity FOB package-info response.
pub type FloatOutBoyInfoResponse = FloatOutBoyPacket<FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN>;

/// Encode FOB's legacy or version-2 package-info response.
#[must_use]
pub fn encode_float_out_boy_info_response(
    request_payload: &[u8],
    hardware_led_mode: u8,
    internal_leds_operational: bool,
    data_recorder_capable: bool,
) -> FloatOutBoyInfoResponse {
    let version = request_payload.first().copied().unwrap_or(1);
    let mut packet = FloatOutBoyPacket::new();
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
    packet.push(FloatOutBoyAppDataCommand::Info.id());
    if version == 1 {
        packet.push(FLOAT_OUT_BOY_MAJOR_VERSION * 10 + FLOAT_OUT_BOY_MINOR_VERSION);
        packet.push(FLOAT_OUT_BOY_BUILD_NUMBER);
        let legacy_led_type = if hardware_led_mode & 0x2 == 0 {
            hardware_led_mode
        } else {
            3
        };
        packet.push(legacy_led_type);
        return packet;
    }

    // Unknown versions use the highest known response with flags cleared,
    // matching upstream's `default` arm.
    let flags = match request_payload {
        [2, flags, ..] => *flags,
        _ => 0,
    };
    packet.push(2);
    packet.push(flags);
    packet.extend_fixed::<20>(FLOAT_OUT_BOY_PACKAGE_NAME);
    packet.push(FLOAT_OUT_BOY_MAJOR_VERSION);
    packet.push(FLOAT_OUT_BOY_MINOR_VERSION);
    packet.push(FLOAT_OUT_BOY_PATCH_VERSION);
    packet.extend_fixed::<20>(FLOAT_OUT_BOY_VERSION_SUFFIX);
    packet.extend(&FLOAT_OUT_BOY_GIT_HASH.to_be_bytes());
    packet.extend(&FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ.to_be_bytes());
    // Upstream derives capabilities from data-recorder and LED config at
    // `third_party/float-out-boy/src/main.c:2121-2132`. Bit 0 is the LED
    // surface and bit 1 is the external LCM surface.
    let external_leds_operational = hardware_led_mode & 0x2 != 0;
    let mut capabilities = u32::from(internal_leds_operational || external_leds_operational)
        | (u32::from(external_leds_operational) << 1);
    capabilities |= u32::from(data_recorder_capable) << 31;
    packet.extend(&capabilities.to_be_bytes());
    // Upstream currently sends zero `extra_flags` at `third_party/float-out-boy/src/main.c:2134-2135`.
    packet.push(0);
    packet
}

#[inline(never)]
/// Return FOB's counted realtime-data identifier lists.
#[must_use]
pub fn encode_float_out_boy_realtime_data_ids_response()
-> [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN] {
    // C map: `cmd_realtime_data_ids` builds a local `uint8_t buffer[bufsize]`
    // and sends it with `SEND_APP_DATA` at `third_party/float-out-boy/src/main.c:1876-1901`.
    // Return owned bytes so the firmware copy reads callback-stack storage, not
    // a package static through an extra firmware boundary.
    FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_BYTES
}

#[cfg(test)]
mod tests;
