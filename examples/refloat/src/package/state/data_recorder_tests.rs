use super::RefloatPackageState;
use crate::domain::{REFLOAT_APP_DATA_PACKAGE_ID, RefloatAppDataCommand};
use crate::package::test_support::sample_all_data_payloads;
use std::vec;
use std::vec::Vec;
use vescpkg_rs::TimestampTicks;
use vescpkg_rs::test_support::FirmwareTest;

fn handle(state: &mut RefloatPackageState, request: &[u8]) -> (bool, Vec<Vec<u8>>) {
    let firmware = FirmwareTest::new();
    let mut now = || TimestampTicks::from_ticks(123);
    let mut sent = Vec::new();
    let mut send = |bytes: &[u8]| {
        sent.push(bytes.to_vec());
        true
    };
    let handled =
        state.handle_packet_with_telemetry(firmware.telemetry(), &mut now, &mut send, request);
    (handled, sent)
}

fn request(command: RefloatAppDataCommand, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![REFLOAT_APP_DATA_PACKAGE_ID.get(), command.id()];
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn recorder_control_updates_live_realtime_flags() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    let (handled, sent) = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    assert!(handled);
    assert!(sent.is_empty());

    let (_, sent) = handle(
        &mut state,
        &request(RefloatAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(sent[0][3] & 0x07, 0x07);
}

#[test]
fn recorder_samples_and_streams_source_wire_packets() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    state.sample_data_recorder(TimestampTicks::from_ticks(0x0102_0304));

    let (_, header) = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&header[0][..7], &[101, 42, 0, 0, 0, 1, 10]);

    let (_, data) = handle(
        &mut state,
        &request(
            RefloatAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    assert_eq!(
        &data[0][..11],
        &[101, 43, 0, 0, 0, 0, 1, 2, 3, 4, 0b0000_1101]
    );
    assert_eq!(data[0].len(), 31);
}

#[test]
fn recorder_control_preserves_autostart_and_autostop_policy() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    let _ = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[1, 2, 0]),
    );
    let _ = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[1, 3, 0]),
    );
    let (_, sent) = handle(
        &mut state,
        &request(RefloatAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(sent[0][3] & 0x07, 0);
}

#[test]
fn recorder_triggers_and_overwrites_the_oldest_sample_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    state.trigger_data_recorder(true);
    for timestamp in 1..=5 {
        state.sample_data_recorder(TimestampTicks::from_ticks(timestamp));
    }
    state.trigger_data_recorder(false);
    state.sample_data_recorder(TimestampTicks::from_ticks(6));

    let (_, header) = handle(
        &mut state,
        &request(RefloatAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(
        u32::from_be_bytes([header[0][2], header[0][3], header[0][4], header[0][5]]),
        4
    );

    let (_, data) = handle(
        &mut state,
        &request(
            RefloatAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    assert_eq!(
        u32::from_be_bytes([data[0][6], data[0][7], data[0][8], data[0][9]]),
        2
    );
}

#[test]
fn experiment_command_is_recognized_as_the_source_noop() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let (handled, sent) = handle(&mut state, &request(RefloatAppDataCommand::Experiment, &[]));

    assert!(handled);
    assert!(sent.is_empty());
}
