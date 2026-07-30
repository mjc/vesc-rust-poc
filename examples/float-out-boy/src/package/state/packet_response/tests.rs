use super::*;
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use crate::package::test_support::sample_all_data_payloads;
use std::vec::Vec;
use vescpkg_rs::prelude::{FirmwareFault, FirmwareFaultId};
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn realtime_packet_response_uses_system_ticks_like_float_out_boy() {
    let app_data = TimestampTicks::from_ticks(0x0102_0304);
    let telemetry = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = Vec::new();
    let mut now = || app_data;
    let mut reply = |bytes: &[u8]| {
        packet.extend_from_slice(bytes);
        true
    };

    assert!(state.reply_to_realtime_data_packet(
        telemetry.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Float Out Boy v1.2.1 writes `d->time.now` into realtime packets at
    // `third_party/float-out-boy/src/main.c:1931`; VESC system ticks are 100 us ticks.
    assert_eq!(&packet[4..8], &[1, 2, 3, 4]);
}

#[test]
fn all_data_response_retains_last_beep_reason_like_upstream_fix() {
    let firmware = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetAllData.id(),
        0,
    ];
    let mut first = Vec::new();
    assert!(state.reply_to_all_data_packet(
        firmware.telemetry(),
        &mut |bytes| {
            first.extend_from_slice(bytes);
            true
        },
        &request,
    ));
    let mut second = Vec::new();
    assert!(state.reply_to_all_data_packet(
        firmware.telemetry(),
        &mut |bytes| {
            second.extend_from_slice(bytes);
            true
        },
        &request,
    ));

    assert_ne!(first[10] >> 4, 0);
    assert_eq!(second[10] >> 4, first[10] >> 4);
}

#[test]
fn all_data_fault_response_preserves_pending_beep_reason_like_refloat() {
    let faulted = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetAllData.id(),
        0,
    ];

    assert!(state.reply_to_all_data_packet(faulted.telemetry(), &mut |_| true, &request,));
    drop(faulted);

    let healthy = FirmwareTest::new();
    let mut response = Vec::new();
    assert!(state.reply_to_all_data_packet(
        healthy.telemetry(),
        &mut |bytes| {
            response.extend_from_slice(bytes);
            true
        },
        &request,
    ));
    assert_ne!(response[10] >> 4, 0);
}

#[test]
fn rejected_all_data_send_retains_last_beep_reason_like_upstream_fix() {
    let firmware = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetAllData.id(),
        0,
    ];

    assert!(!state.reply_to_all_data_packet(firmware.telemetry(), &mut |_| false, &request,));

    let mut response = Vec::new();
    assert!(state.reply_to_all_data_packet(
        firmware.telemetry(),
        &mut |bytes| {
            response.extend_from_slice(bytes);
            true
        },
        &request,
    ));
    assert_ne!(response[10] >> 4, 0);
}

#[test]
fn malformed_all_data_request_preserves_pending_beep_reason_like_refloat() {
    let firmware = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    assert!(!state.reply_to_all_data_packet(
        firmware.telemetry(),
        &mut |_| true,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::GetAllData.id(),
        ],
    ));

    let mut response = Vec::new();
    assert!(state.reply_to_all_data_packet(
        firmware.telemetry(),
        &mut |bytes| {
            response.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::GetAllData.id(),
            0,
        ],
    ));
    assert_ne!(response[10] >> 4, 0);
}

#[test]
fn realtime_packet_reports_live_firmware_fault_alert_like_float_out_boy() {
    let now = TimestampTicks::from_ticks(42);
    let firmware = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);
    let mut packet = Vec::new();

    assert!(state.reply_to_realtime_data_packet(
        firmware.telemetry(),
        &mut || now,
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    assert_ne!(
        u32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]) & (1 << 20),
        0
    );
    assert_eq!(&packet[packet.len() - 9..packet.len() - 5], &[0, 0, 0, 1]);
    assert_eq!(packet.last(), Some(&5));
}

#[test]
fn alerts_list_command_returns_source_header_when_empty() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = Vec::new();

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::AlertsList.id(),
        ],
    ));

    assert_eq!(packet, [101, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn alerts_list_command_returns_firmware_fault_name_and_record() {
    let now = TimestampTicks::from_ticks(42);
    let firmware = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);
    let mut packet = Vec::new();

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || now,
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::AlertsList.id(),
        ],
    ));

    let name = b"OVER_TEMP_FET";
    assert_eq!(&packet[..11], &[101, 35, 0, 0, 0, 1, 0, 0, 0, 0, 5]);
    assert_eq!(packet[11], u8::try_from(name.len()).unwrap_or(u8::MAX));
    assert_eq!(&packet[12..25], name);
    assert_eq!(&packet[25..34], &[1, 0, 0, 0, 42, 1, 1, 5, 13]);
    assert_eq!(&packet[34..], name);
}

#[test]
fn alerts_list_uses_each_historical_fault_code_for_its_name() {
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let firmware = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverVoltage));
    state.refresh_runtime_state(
        firmware.telemetry(),
        firmware.imu(),
        TimestampTicks::from_ticks(1),
    );
    let firmware =
        firmware.with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    state.refresh_runtime_state(
        firmware.telemetry(),
        firmware.imu(),
        TimestampTicks::from_ticks(2),
    );
    let mut packet = Vec::new();

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(2),
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::AlertsList.id(),
        ],
    ));

    assert!(
        packet
            .windows(b"OVER_VOLTAGE".len())
            .any(|name| name == b"OVER_VOLTAGE")
    );
    assert!(
        packet
            .windows(b"OVER_TEMP_FET".len())
            .any(|name| name == b"OVER_TEMP_FET")
    );
}

#[test]
fn alerts_control_clears_the_persistent_fatal_without_hiding_the_live_fault() {
    let now = TimestampTicks::from_ticks(42);
    let firmware = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || now,
        &mut |_| true,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::AlertsControl.id(),
            1,
        ],
    ));

    let mut packet = Vec::new();
    assert!(state.reply_to_realtime_data_packet(
        firmware.telemetry(),
        &mut || now,
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    assert_eq!(packet[3] & 0x08, 0);
    assert_eq!(&packet[packet.len() - 9..packet.len() - 5], &[0, 0, 0, 1]);
    assert_eq!(packet.last(), Some(&5));
}

#[test]
fn metadata_packet_response_defaults_to_legacy_info_like_float_out_boy() {
    let state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = Vec::new();

    assert!(state.reply_to_metadata_packet(
        &mut |bytes| {
            packet.extend_from_slice(bytes);
            true
        },
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Info.id(),
        ],
    ));

    assert_eq!(packet, [101, 0, 1, 0, 0]);
}

#[test]
fn metadata_packet_response_sends_realtime_ids_directly() {
    let state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = Vec::new();
    let mut reply = |bytes: &[u8]| {
        packet.extend_from_slice(bytes);
        true
    };

    assert!(state.reply_to_metadata_packet(
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeDataIds.id(),
        ],
    ));

    // C map: QML asks for this packet at `ui.qml.in:704-705`; Float Out Boy C replies
    // from `third_party/float-out-boy/src/main.c:1876-1901`.
    assert_eq!(packet.len(), 370);
    assert_eq!(
        &packet[..3],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeDataIds.id(),
            18,
        ]
    );
}
