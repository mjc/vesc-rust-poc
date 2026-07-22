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
const FLOAT_OUT_BOY_MAJOR_VERSION: u8 = 1;
const FLOAT_OUT_BOY_MINOR_VERSION: u8 = 2;
const FLOAT_OUT_BOY_PATCH_VERSION: u8 = 1;
const FLOAT_OUT_BOY_BUILD_NUMBER: u8 = 1;
const FLOAT_OUT_BOY_GIT_HASH: u32 = 0x0ef6_e99d;
const FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ: u32 = SYSTEM_TICK_RATE_HZ as u32;

// Float Out Boy C builds this exact packet in `third_party/float-out-boy/src/main.c:1876-1901`, using the ID
// order from `third_party/float-out-boy/src/rt_data.h:38-66` and counted-string framing from
// `third_party/float-out-boy/src/conf/buffer.c:147-155`. QML reads the same two string lists in
// `ui.qml.in:926-934`.
// Keep the materialized bytes in the loaded extension image so hardware never
// has to dereference string-literal storage.
vescpkg_rs::firmware_section_static!(
    ".text.float_out_boy_realtime_data_ids",
    static FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_BYTES: [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN] =
        build_float_out_boy_realtime_data_ids_response()
);

pub(in crate::package) struct FloatOutBoyInfoResponse {
    bytes: [u8; FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN],
    len: usize,
}

impl FloatOutBoyInfoResponse {
    pub(in crate::package) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

pub(in crate::package) fn encode_float_out_boy_info_response(
    request_payload: &[u8],
    hardware_led_mode: u8,
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
    // `third_party/float-out-boy/src/main.c:2121-2132`; this Rust runtime has not ported either
    // capability yet, so the honest advertised capability mask is zero.
    float_out_boy_response_push_bytes(&mut bytes, &mut index, &0u32.to_be_bytes());
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

// Same packet as `cmd_realtime_data_ids` in `third_party/float-out-boy/src/main.c:1876-1901`, built as
// bytes so the ARM image does not rely on target string-literal addresses.
const fn build_float_out_boy_realtime_data_ids_response()
-> [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN] {
    let mut bytes = [0; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN];
    let mut index = 0;

    index = float_out_boy_realtime_ids_push_u8(&mut bytes, index, 101);
    index = float_out_boy_realtime_ids_push_u8(&mut bytes, index, 32);

    index = float_out_boy_realtime_ids_push_u8(&mut bytes, index, 16);
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.speed");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.erpm");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.current");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.dir_current");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.filt_current");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.duty_cycle");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.batt_voltage");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.batt_current");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.mosfet_temp");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"motor.motor_temp");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"imu.pitch");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"imu.balance_pitch");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"imu.roll");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"footpad.adc1");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"footpad.adc2");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"remote.input");

    index = float_out_boy_realtime_ids_push_u8(&mut bytes, index, 10);
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"atr.setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"brake_tilt.setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"torque_tilt.setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"turn_tilt.setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"remote.setpoint");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"balance_current");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"atr.accel_diff");
    index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"atr.speed_boost");
    let _index = float_out_boy_realtime_ids_push_id(&mut bytes, index, b"booster.current");

    bytes
}

const fn float_out_boy_realtime_ids_push_id<const N: usize>(
    bytes: &mut [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: &[u8; N],
) -> usize {
    let mut next = float_out_boy_realtime_ids_push_u8(bytes, index, N as u8);
    let mut offset = 0;
    while offset < N {
        next = float_out_boy_realtime_ids_push_u8(bytes, next, value[offset]);
        offset += 1;
    }
    next
}

const fn float_out_boy_realtime_ids_push_u8(
    bytes: &mut [u8; FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: u8,
) -> usize {
    bytes[index] = value;
    index + 1
}

fn float_out_boy_response_push_bytes(bytes: &mut [u8], index: &mut usize, values: &[u8]) {
    values
        .iter()
        .copied()
        .for_each(|byte| float_out_boy_response_push_u8(bytes, index, byte));
}

fn float_out_boy_response_push_u8(bytes: &mut [u8], index: &mut usize, value: u8) {
    if let Some(slot) = bytes.get_mut(*index) {
        *slot = value;
    }
    *index = index.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_v2_response_matches_float_out_boy_qml_metadata() {
        let response = encode_float_out_boy_info_response(&[2, 0], 0);
        let bytes = response.as_bytes();

        // QML sends COMMAND_INFO version 2 at `ui.qml.in:693-697`; upstream
        // `cmd_info` replies with the v2 metadata layout at `third_party/float-out-boy/src/main.c:2108-2135`.
        assert_eq!(bytes.len(), 60);
        assert_eq!(&bytes[..4], &[101, 0, 2, 0]);
        assert_eq!(&bytes[4..17], b"Float Out Boy");
        assert_eq!(&bytes[24..27], &[1, 2, 1]);
        assert_eq!(
            u32::from_be_bytes([bytes[47], bytes[48], bytes[49], bytes[50]]),
            0x0ef6_e99d
        );
        assert_eq!(
            u32::from_be_bytes([bytes[51], bytes[52], bytes[53], bytes[54]]),
            FLOAT_OUT_BOY_SYSTEM_TICK_RATE_HZ
        );
        assert_eq!(
            u32::from_be_bytes([bytes[55], bytes[56], bytes[57], bytes[58]]),
            0
        );
        assert_eq!(bytes[59], 0);
        assert_eq!(
            &encode_float_out_boy_info_response(&[2, 0xa5], 0).as_bytes()[..4],
            &[101, 0, 2, 0xa5]
        );
    }

    #[test]
    fn info_v1_response_matches_float_out_boy_legacy_shape_and_led_mapping() {
        assert_eq!(
            encode_float_out_boy_info_response(&[], 1).as_bytes(),
            &[101, 0, 12, 1, 1]
        );
        assert_eq!(
            encode_float_out_boy_info_response(&[1], 2).as_bytes(),
            &[101, 0, 12, 1, 3]
        );
        assert_eq!(
            encode_float_out_boy_info_response(&[1], 3).as_bytes(),
            &[101, 0, 12, 1, 3]
        );
    }

    #[test]
    fn unknown_info_version_uses_v2_without_echoing_flags() {
        let response = encode_float_out_boy_info_response(&[99, 0xff], 0);

        assert_eq!(&response.as_bytes()[..4], &[101, 0, 2, 0]);
        assert_eq!(
            response.as_bytes().len(),
            FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN
        );
    }

    #[test]
    fn realtime_data_ids_response_matches_float_out_boy_qml_metadata() {
        fn take_id_list<'a>(bytes: &'a [u8], index: &mut usize) -> std::vec::Vec<&'a str> {
            let count = bytes
                .get(*index)
                .copied()
                .map(usize::from)
                .expect("ID count byte");
            *index = index.saturating_add(1);

            (0..count)
                .map(|_| {
                    let len = bytes
                        .get(*index)
                        .copied()
                        .map(usize::from)
                        .expect("ID length byte");
                    *index = index.saturating_add(1);
                    let end = index.saturating_add(len);
                    let id = bytes.get(*index..end).expect("ID bytes");
                    *index = end;
                    core::str::from_utf8(id).expect("ID UTF-8")
                })
                .collect()
        }

        let bytes = encode_float_out_boy_realtime_data_ids_response();

        // QML asks for IDs at `ui.qml.in:704-705`;
        // upstream `cmd_realtime_data_ids` writes the counted string sets at
        // `third_party/float-out-boy/src/main.c:1876-1901`, using IDs from `third_party/float-out-boy/src/rt_data.h:38-66`.
        assert_eq!(bytes.len(), 405);
        assert_eq!(bytes.get(..2), Some(&[101, 32][..]));
        let mut index = 2;
        assert_eq!(
            take_id_list(&bytes, &mut index).as_slice(),
            &[
                "motor.speed",
                "motor.erpm",
                "motor.current",
                "motor.dir_current",
                "motor.filt_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "motor.batt_current",
                "motor.mosfet_temp",
                "motor.motor_temp",
                "imu.pitch",
                "imu.balance_pitch",
                "imu.roll",
                "footpad.adc1",
                "footpad.adc2",
                "remote.input",
            ]
        );
        assert_eq!(
            take_id_list(&bytes, &mut index).as_slice(),
            &[
                "setpoint",
                "atr.setpoint",
                "brake_tilt.setpoint",
                "torque_tilt.setpoint",
                "turn_tilt.setpoint",
                "remote.setpoint",
                "balance_current",
                "atr.accel_diff",
                "atr.speed_boost",
                "booster.current",
            ]
        );
        assert_eq!(index, bytes.len());
    }
}
