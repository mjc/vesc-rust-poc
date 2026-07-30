use super::FloatOutBoyPackageState;
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use crate::package::test_support::sample_all_data_payloads;
use std::vec::Vec;
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{SampleRate, TimestampTicks};

fn handle(state: &mut FloatOutBoyPackageState, request: &[u8]) -> (bool, Vec<Vec<u8>>) {
    let firmware = FirmwareTest::new();
    let mut now = || TimestampTicks::from_ticks(123);
    let mut replies = Vec::new();
    let mut reply = |bytes: &[u8]| {
        replies.push(bytes.to_vec());
        true
    };
    let handled =
        state.handle_packet_with_telemetry(firmware.telemetry(), &mut now, &mut reply, request);
    (handled, replies)
}

fn request(command: FloatOutBoyAppDataCommand, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, command.id()];
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn recorder_control_updates_live_realtime_flags() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let (handled, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    assert!(handled);
    assert_eq!(sent, [vec![101, 41, 1, 0b111, 1, 0, 3]]);

    let (_, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(sent[0][3], 1);
}

#[test]
fn package_stop_clears_live_recorder_activity() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );

    crate::package::stop(&mut state);

    let (_, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(sent[0][3] & 0x07, 0);

    let (_, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::Info, &[2, 0]),
    );
    assert_eq!(
        u32::from_be_bytes([sent[0][55], sent[0][56], sent[0][57], sent[0][58]]) & (1 << 31),
        0
    );
}

#[test]
fn recorder_samples_and_streams_source_wire_packets() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    state.sample_data_recorder(TimestampTicks::from_ticks(0x0102_0304));

    let (_, header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&header[0][..7], &[101, 42, 0, 0, 0, 1, 13]);

    let (_, data) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    assert_eq!(
        &data[0][..11],
        &[101, 43, 0, 0, 0, 0, 1, 2, 3, 4, 0b0000_1101]
    );
    assert_eq!(data[0].len(), 37);
}

#[test]
fn recorder_control_preserves_autostart_and_autostop_policy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 2, 0]),
    );
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 3, 0]),
    );
    let (_, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(sent[0][3] & 0x07, 0);
}

#[test]
fn recorder_status_and_each_control_return_the_complete_cutoff_state() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    for (payload, expected) in [
        (&[1, 0][..], [101, 41, 1, 0b110, 1, 0, 3]),
        (&[1, 2, 0][..], [101, 41, 1, 0b100, 1, 0, 3]),
        (&[1, 3, 0][..], [101, 41, 1, 0, 1, 0, 3]),
        (&[1, 4, 0][..], [101, 41, 1, 0, 1, 0, 3]),
        (&[1, 4, 3][..], [101, 41, 1, 0, 3, 0, 3]),
        (&[1, 1, 1][..], [101, 41, 1, 1, 3, 0, 3]),
    ] {
        let (_, sent) = handle(
            &mut state,
            &request(FloatOutBoyAppDataCommand::DataRecordRequest, payload),
        );
        assert_eq!(sent, [expected.to_vec()], "payload {payload:?}");
    }
}

#[test]
fn recorder_initial_rate_calculates_decimation_and_live_rate_only_changes_duration() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    state.initialize_data_recorder_sample_rate(SampleRate::from_hertz(48.0));

    let (_, initial) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 0]),
    );
    assert_eq!(initial, [vec![101, 41, 1, 0b110, 20, 0, 50]]);

    state.refresh_data_recorder_sample_rate(SampleRate::from_hertz(24.0));
    let (_, refreshed) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 0]),
    );
    assert_eq!(refreshed, [vec![101, 41, 1, 0b110, 20, 0, 100]]);
}

#[test]
fn recorder_decimation_records_only_every_nth_sample_and_start_resets_the_counter() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 4, 3]),
    );
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    for timestamp in 1..=3 {
        state.sample_data_recorder(TimestampTicks::from_ticks(timestamp));
    }

    let (_, header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&header[0][2..6], &[0, 0, 0, 1]);

    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    state.sample_data_recorder(TimestampTicks::from_ticks(4));
    state.sample_data_recorder(TimestampTicks::from_ticks(5));
    let (_, restarted_header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&restarted_header[0][2..6], &[0, 0, 0, 0]);
}

#[test]
fn recorder_makes_duplicate_and_regressing_timestamps_strictly_increasing() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    for timestamp in [0, 0, 1, 1] {
        state.sample_data_recorder(TimestampTicks::from_ticks(timestamp));
    }

    let (_, data) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    for (offset, expected) in [(6, 1), (37, 2), (68, 3), (99, 4)] {
        assert_eq!(&data[0][offset..offset + 4], &u32::to_be_bytes(expected));
    }
}

#[test]
fn recorder_triggers_and_overwrites_the_oldest_sample_like_refloat() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    state.trigger_data_recorder(true);
    for timestamp in 1..=25 {
        state.sample_data_recorder(TimestampTicks::from_ticks(timestamp));
    }
    state.trigger_data_recorder(false);
    state.sample_data_recorder(TimestampTicks::from_ticks(6));

    let (_, header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(
        u32::from_be_bytes([header[0][2], header[0][3], header[0][4], header[0][5]]),
        24
    );

    let (_, data) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
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
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let (handled, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::Experiment, &[]),
    );

    assert!(handled);
    assert!(sent.is_empty());
}

#[test]
fn print_info_command_is_recognized_as_the_source_noop() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let (handled, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::PrintInfo, &[]),
    );

    assert!(handled);
    assert!(sent.is_empty());
}

#[test]
fn unavailable_recorder_fails_closed_across_commands_flags_and_capability() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    state.disable_data_recorder_for_test();

    let (handled, recorder_response) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    assert!(handled);
    assert_eq!(recorder_response, [vec![101, 41, 0, 0b110, 1, 0, 3]]);

    let (_, realtime_response) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
    );
    assert_eq!(realtime_response[0][3] & 0x07, 0);

    let (_, info_response) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::Info, &[2, 0]),
    );
    assert_eq!(
        u32::from_be_bytes([
            info_response[0][55],
            info_response[0][56],
            info_response[0][57],
            info_response[0][58],
        ]) & (1 << 31),
        0
    );
}

#[test]
fn malformed_and_unknown_recorder_requests_are_recognized_noops() {
    for payload in [
        &[][..],
        &[1][..],
        &[1, 1][..],
        &[2, 2][..],
        &[2, 2, 0, 0, 0][..],
        &[9, 9, 9][..],
    ] {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        let (handled, sent) = handle(
            &mut state,
            &request(FloatOutBoyAppDataCommand::DataRecordRequest, payload),
        );
        assert!(handled, "payload {payload:?}");
        assert!(sent.is_empty(), "payload {payload:?}");
    }

    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let (handled, sent) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 9, 1]),
    );
    assert!(handled);
    assert_eq!(sent, [vec![101, 41, 1, 0b110, 1, 0, 3]]);
}

#[test]
fn manual_stop_preserves_samples_and_manual_start_clears_them() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    state.sample_data_recorder(TimestampTicks::from_ticks(1));
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 0]),
    );

    let (_, stopped_header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&stopped_header[0][2..6], &[0, 0, 0, 1]);

    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    let (_, restarted_header) = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[2, 1]),
    );
    assert_eq!(&restarted_header[0][2..6], &[0, 0, 0, 0]);
}

#[test]
fn empty_and_out_of_range_data_requests_match_refloat_response_policy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let (_, empty) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    assert!(empty.is_empty());

    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    state.sample_data_recorder(TimestampTicks::from_ticks(1));
    let (_, out_of_range) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 1],
        ),
    );
    assert_eq!(out_of_range, [vec![101, 43, 0, 0, 0, 1]]);
}

#[test]
fn engage_and_disengage_cover_every_autostart_autostop_combination() {
    for (autostart, autostop, expected_after_engage, expected_after_disengage) in [
        (false, false, false, false),
        (false, true, false, false),
        (true, false, true, true),
        (true, true, true, false),
    ] {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
        for (submode, enabled) in [(2, autostart), (3, autostop)] {
            let _ = handle(
                &mut state,
                &request(
                    FloatOutBoyAppDataCommand::DataRecordRequest,
                    &[1, submode, u8::from(enabled)],
                ),
            );
        }

        state.trigger_data_recorder(true);
        let (_, engaged) = handle(
            &mut state,
            &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
        );
        assert_eq!(engaged[0][3] & 1 != 0, expected_after_engage);

        state.trigger_data_recorder(false);
        let (_, disengaged) = handle(
            &mut state,
            &request(FloatOutBoyAppDataCommand::RealtimeData, &[]),
        );
        assert_eq!(disengaged[0][3] & 1 != 0, expected_after_disengage);
    }
}

#[test]
fn data_response_paginates_at_the_refloat_send_buffer_boundary() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let _ = handle(
        &mut state,
        &request(FloatOutBoyAppDataCommand::DataRecordRequest, &[1, 1, 1]),
    );
    for timestamp in 1..=24 {
        state.sample_data_recorder(TimestampTicks::from_ticks(timestamp));
    }

    let (_, first_page) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 0],
        ),
    );
    assert_eq!(first_page[0].len(), 502);
    assert_eq!(&first_page[0][2..6], &[0, 0, 0, 0]);
    assert_eq!(&first_page[0][6..10], &[0, 0, 0, 1]);
    assert_eq!(&first_page[0][471..475], &[0, 0, 0, 16]);

    let (_, second_page) = handle(
        &mut state,
        &request(
            FloatOutBoyAppDataCommand::DataRecordRequest,
            &[2, 2, 0, 0, 0, 16],
        ),
    );
    assert_eq!(second_page[0].len(), 254);
    assert_eq!(&second_page[0][2..6], &[0, 0, 0, 16]);
    assert_eq!(&second_page[0][6..10], &[0, 0, 0, 17]);
    assert_eq!(&second_page[0][223..227], &[0, 0, 0, 24]);
}
