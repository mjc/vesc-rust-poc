use crate::domain::{REFLOAT_APP_DATA_PACKAGE_ID, RefloatAppDataCommand};

// Refloat v1.2.1 `cmd_info` writes this version-2 response shape at
// `third_party/refloat/src/main.c:2070-2139`.
pub(super) const REFLOAT_INFO_RESPONSE_V2_LEN: usize = 60;
// Refloat v1.2.1 `cmd_realtime_data_ids` writes the counted ID-list packet at
// `third_party/refloat/src/main.c:1876-1901`.
pub(super) const REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN: usize = 405;

const REFLOAT_PACKAGE_NAME: &[u8] = b"Refloat";
const REFLOAT_VERSION_SUFFIX: &[u8] = b"";
const REFLOAT_GIT_HASH: u32 = 0x0ef6_e99d;
const REFLOAT_SYSTEM_TICK_RATE_HZ: u32 = 10_000;

// Refloat C builds this exact packet in `third_party/refloat/src/main.c:1876-1901`, using the ID
// order from `third_party/refloat/src/rt_data.h:38-66` and counted-string framing from
// `third_party/refloat/src/conf/buffer.c:147-155`. QML reads the same two string lists in
// `ui.qml.in:926-934`.
// Keep the materialized bytes in the loaded extension image so hardware never
// has to dereference string-literal storage.
vescpkg_rs::firmware_section_static!(
    ".text.refloat_realtime_data_ids",
    static REFLOAT_REALTIME_DATA_IDS_RESPONSE_BYTES: [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN] =
        build_refloat_realtime_data_ids_response()
);

pub(in crate::package) fn encode_refloat_info_response_v2(
    request_payload: &[u8],
) -> [u8; REFLOAT_INFO_RESPONSE_V2_LEN] {
    // Upstream `cmd_info` responds to QML's version-2 request at
    // `third_party/refloat/src/main.c:2070-2139`; QML allocates the four-byte request and sets
    // version 2 at `ui.qml.in:693-697`.
    let flags = match request_payload {
        [2, flags, ..] => *flags,
        _ => 0,
    };
    let mut bytes = [0; REFLOAT_INFO_RESPONSE_V2_LEN];
    let mut index = 0;
    refloat_response_push_u8(&mut bytes, &mut index, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_response_push_u8(&mut bytes, &mut index, RefloatAppDataCommand::Info.id());
    refloat_response_push_u8(&mut bytes, &mut index, 2);
    refloat_response_push_u8(&mut bytes, &mut index, flags);
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_PACKAGE_NAME);
    refloat_response_push_u8(&mut bytes, &mut index, 1);
    refloat_response_push_u8(&mut bytes, &mut index, 2);
    refloat_response_push_u8(&mut bytes, &mut index, 1);
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_VERSION_SUFFIX);
    refloat_response_push_bytes(&mut bytes, &mut index, &REFLOAT_GIT_HASH.to_be_bytes());
    refloat_response_push_bytes(
        &mut bytes,
        &mut index,
        &REFLOAT_SYSTEM_TICK_RATE_HZ.to_be_bytes(),
    );
    // Upstream derives capabilities from data-recorder and LED config at
    // `third_party/refloat/src/main.c:2121-2132`; this Rust runtime has not ported either
    // capability yet, so the honest advertised capability mask is zero.
    refloat_response_push_bytes(&mut bytes, &mut index, &0u32.to_be_bytes());
    // Upstream currently sends zero `extra_flags` at `third_party/refloat/src/main.c:2134-2135`.
    refloat_response_push_u8(&mut bytes, &mut index, 0);
    bytes
}

fn append_fixed_ascii<const LEN: usize>(bytes: &mut [u8], index: &mut usize, value: &[u8]) {
    // C map: `buffer_append_string_fixed` copies up to the fixed width, then
    // zero-pads at `third_party/refloat/src/conf/buffer.c:169-181`.
    let start = *index;
    for (offset, byte) in value.iter().copied().take(LEN).enumerate() {
        if let Some(slot) = bytes.get_mut(start.saturating_add(offset)) {
            *slot = byte;
        }
    }
    *index = start.saturating_add(LEN);
}

#[inline(never)]
pub(in crate::package) fn encode_refloat_realtime_data_ids_response()
-> [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN] {
    // C map: `cmd_realtime_data_ids` builds a local `uint8_t buffer[bufsize]`
    // and sends it with `SEND_APP_DATA` at `third_party/refloat/src/main.c:1876-1901`.
    // Return owned bytes so the firmware copy reads callback-stack storage, not
    // a package static through an extra firmware boundary.
    REFLOAT_REALTIME_DATA_IDS_RESPONSE_BYTES
}

// Same packet as `cmd_realtime_data_ids` in `third_party/refloat/src/main.c:1876-1901`, built as
// bytes so the ARM image does not rely on target string-literal addresses.
const fn build_refloat_realtime_data_ids_response() -> [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN]
{
    let mut bytes = [0; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN];
    let mut index = 0;

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 101);
    index = refloat_realtime_ids_push_u8(&mut bytes, index, 32);

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 16);
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.speed");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.erpm");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.dir_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.filt_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.duty_cycle");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.batt_voltage");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.batt_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.mosfet_temp");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.motor_temp");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.pitch");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.balance_pitch");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.roll");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"footpad.adc1");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"footpad.adc2");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"remote.input");

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 10);
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"brake_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"torque_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"turn_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"remote.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"balance_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.accel_diff");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.speed_boost");
    let _index = refloat_realtime_ids_push_id(&mut bytes, index, b"booster.current");

    bytes
}

const fn refloat_realtime_ids_push_id<const N: usize>(
    bytes: &mut [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: &[u8; N],
) -> usize {
    let mut next = refloat_realtime_ids_push_u8(bytes, index, N as u8);
    let mut offset = 0;
    while offset < N {
        next = refloat_realtime_ids_push_u8(bytes, next, value[offset]);
        offset += 1;
    }
    next
}

const fn refloat_realtime_ids_push_u8(
    bytes: &mut [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: u8,
) -> usize {
    bytes[index] = value;
    index + 1
}

fn refloat_response_push_bytes(bytes: &mut [u8], index: &mut usize, values: &[u8]) {
    values
        .iter()
        .copied()
        .for_each(|byte| refloat_response_push_u8(bytes, index, byte));
}

fn refloat_response_push_u8(bytes: &mut [u8], index: &mut usize, value: u8) {
    if let Some(slot) = bytes.get_mut(*index) {
        *slot = value;
    }
    *index = index.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_v2_response_matches_refloat_qml_metadata() {
        let bytes = encode_refloat_info_response_v2(&[2, 0]);

        // QML sends COMMAND_INFO version 2 at `ui.qml.in:693-697`; upstream
        // `cmd_info` replies with the v2 metadata layout at `third_party/refloat/src/main.c:2108-2135`.
        assert_eq!(bytes.len(), 60);
        assert_eq!(&bytes[..4], &[101, 0, 2, 0]);
        assert_eq!(&bytes[4..11], b"Refloat");
        assert_eq!(&bytes[24..27], &[1, 2, 1]);
        assert_eq!(
            u32::from_be_bytes([bytes[47], bytes[48], bytes[49], bytes[50]]),
            0x0ef6_e99d
        );
        assert_eq!(
            u32::from_be_bytes([bytes[51], bytes[52], bytes[53], bytes[54]]),
            10_000
        );
        assert_eq!(
            u32::from_be_bytes([bytes[55], bytes[56], bytes[57], bytes[58]]),
            0
        );
        assert_eq!(bytes[59], 0);
    }

    #[test]
    fn realtime_data_ids_response_matches_refloat_qml_metadata() {
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

        let bytes = encode_refloat_realtime_data_ids_response();

        // QML asks for IDs at `ui.qml.in:704-705`;
        // upstream `cmd_realtime_data_ids` writes the counted string sets at
        // `third_party/refloat/src/main.c:1876-1901`, using IDs from `third_party/refloat/src/rt_data.h:38-66`.
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
