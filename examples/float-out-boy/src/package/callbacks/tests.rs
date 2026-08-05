use super::{FloatOutBoyAppData, handle_float_out_boy_app_data_packet};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAppDataCommand,
    FloatOutBoyMode, FloatOutBoyRealtimeRemoteInput, FloatOutBoyRunState,
};
use crate::package::FloatOutBoyPackageState;
use crate::package::test_support::{
    default_float_out_boy_config_bytes, editable_config_from_state, sample_all_data_payloads,
    sample_all_data_payloads_with_ride_state,
};
use std::vec::Vec;
use vescpkg_rs::AppDataPacket;
use vescpkg_rs::test_support::{FirmwareTest, invoke_stateful_app_data_handler};
use vescpkg_rs::{SignedRatio, TimestampTicks};

fn handle_packet(
    state: &mut FloatOutBoyPackageState,
    now: TimestampTicks,
    sent: &mut Vec<Vec<u8>>,
    telemetry: &impl vescpkg_rs::MotorTelemetry,
    imu: &impl vescpkg_rs::Imu,
    packet: AppDataPacket<'_>,
) -> bool {
    let mut now = || now;
    let mut record_packet = |bytes: &[u8]| {
        sent.push(Vec::from(bytes));
        true
    };
    handle_float_out_boy_app_data_packet(
        state,
        telemetry,
        imu,
        &mut now,
        &mut record_packet,
        packet,
    )
}

fn last_log(firmware: &FirmwareTest) -> Vec<u8> {
    let mut bytes = [0; 128];
    let len = firmware.copy_last_log(&mut bytes);
    bytes[..len].to_vec()
}

#[test]
fn app_data_callback_logs_exact_truncated_header_lengths_before_dispatch() {
    let _state_lock = super::super::custom_config::lock_test_float_out_boy_config_state();
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    for (packet, expected) in [
        (
            [].as_slice(),
            b"Received command data too short: 0 bytes.".as_slice(),
        ),
        (
            [FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID].as_slice(),
            b"Received command data too short: 1 bytes.".as_slice(),
        ),
        (
            [FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID - 1, 0].as_slice(),
            b"Invalid Package ID: 100".as_slice(),
        ),
    ] {
        assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(
            packet
        ));
        assert_eq!(last_log(&firmware), expected);
    }
}

#[test]
fn handler_rejects_empty_and_sends_valid_packets() {
    let app_data = TimestampTicks::from_ticks(0);
    let mut sent = Vec::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let telemetry = FirmwareTest::new();
    let imu = telemetry.imu();
    let empty_packet = AppDataPacket::from_bytes(&[]);
    assert!(!handle_packet(
        &mut state,
        app_data,
        &mut sent,
        telemetry.telemetry(),
        imu,
        empty_packet,
    ));
    assert!(sent.is_empty());

    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetAllData.id(),
        FloatOutBoyAllDataMode::from_source_id(1).source_id(),
    ];
    let packet = AppDataPacket::from_bytes(&request);
    assert!(handle_packet(
        &mut state,
        app_data,
        &mut sent,
        telemetry.telemetry(),
        imu,
        packet,
    ));
    assert_eq!(sent.len(), 1);
    assert_eq!(&sent[0][..3], &request);
}

#[test]
fn app_data_callback_dispatches_legacy_realtime_data_like_float_out_boy() {
    let app_data = TimestampTicks::from_ticks(0);
    let mut sent = Vec::new();
    let telemetry = FirmwareTest::new();
    let imu = telemetry.imu();
    let payloads = sample_all_data_payloads();
    let expected = crate::protocol::encode_float_out_boy_get_realtime_data_response_with_remote(
        &payloads,
        FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0)),
        0.0,
    );
    let mut state = FloatOutBoyPackageState::new(payloads);
    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetRealtimeData.id(),
    ];

    assert!(handle_packet(
        &mut state,
        app_data,
        &mut sent,
        telemetry.telemetry(),
        imu,
        AppDataPacket::from_bytes(&request),
    ));
    assert_eq!(sent.as_slice(), [expected.as_slice()]);
}

#[test]
fn app_data_callback_rejects_malformed_legacy_realtime_data_requests() {
    let app_data = TimestampTicks::from_ticks(0);
    let mut sent = Vec::new();
    let telemetry = FirmwareTest::new();
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    for request in [
        &[][..],
        &[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID][..],
        &[100, 1][..],
    ] {
        assert!(!handle_packet(
            &mut state,
            app_data,
            &mut sent,
            telemetry.telemetry(),
            imu,
            AppDataPacket::from_bytes(request),
        ));
    }
    assert!(sent.is_empty());
}

#[test]
fn app_data_callback_routes_unified_remote_without_reply_and_rejects_removed_rc_move() {
    let mut sent = Vec::new();
    let telemetry = FirmwareTest::new();
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let now = TimestampTicks::from_ticks(30_001);
    let command = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::Remote.id(),
        127,
    ];

    assert!(handle_packet(
        &mut state,
        now,
        &mut sent,
        telemetry.telemetry(),
        imu,
        AppDataPacket::from_bytes(&command),
    ));
    assert_eq!(
        state.remote_input_for_test().ratio(),
        SignedRatio::from_ratio_const(1.0)
    );
    assert!(sent.is_empty());

    assert!(!handle_packet(
        &mut state,
        now,
        &mut sent,
        telemetry.telemetry(),
        imu,
        AppDataPacket::from_bytes(&[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, 7, 1, 40, 2, 42]),
    ));
    assert!(sent.is_empty());
}

#[test]
fn app_data_callback_dispatches_without_main_loop_refresh_like_float_out_boy() {
    let app_data = TimestampTicks::from_ticks(0);
    let mut sent = Vec::new();
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));

    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::RealtimeData.id(),
        0,
    ];
    let packet = AppDataPacket::from_bytes(&request);
    assert!(handle_packet(
        &mut state,
        app_data,
        &mut sent,
        telemetry.telemetry(),
        imu,
        packet,
    ));

    // Upstream `on_command_received` only dispatches app commands at
    // `third_party/float-out-boy/src/main.c:2143-2225`; READY engage and
    // IMU/motor refresh stay in `float_out_boy_thd` at `third_party/float-out-boy/src/main.c:772-1080`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready
    );
}

fn assert_real_config_restore_context() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));
    let persisted = editable_config_from_state(&state);
    state.replace_serialized_config_for_test(&crate::config::FloatOutBoyConfigImage::defaults());
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::ConfigRestore.id(),
    ]));
    drop(installed);
    assert_eq!(state.serialized_config(), persisted.as_bytes());
}

fn assert_real_config_save_context(firmware: &FirmwareTest) {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    state.initialize_time_epochs(TimestampTicks::from_ticks(0));
    let expected = *state.serialized_config();
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::ConfigSave.id(),
    ]));
    drop(installed);

    let persisted = firmware
        .with_effects(|effects| firmware.eeprom().read_image::<320>(effects))
        .expect("config save must write the complete EEPROM image");
    assert_eq!(&persisted[..expected.len()], &expected);
}

fn assert_real_lock_context() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::Lock.id(),
        1,
    ]));
    drop(installed);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Disabled
    );
}

fn assert_real_handtest_restore_context() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));
    assert_eq!(
        state.prepare_handtest_packet(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::HandTest.id(),
            1,
        ]),
        Some(false)
    );
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::HandTest.id(),
        0,
    ]));
    drop(installed);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Normal
    );
}

fn assert_real_flywheel_restore_context() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));
    assert_eq!(
        state.prepare_flywheel_packet(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Flywheel.id(),
            0x81,
            90,
            50,
            30,
            20,
            1,
        ]),
        Some(false)
    );
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::Flywheel.id(),
        0x80,
        0,
        0,
        0,
        0,
        0,
    ]));
    drop(installed);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Normal
    );
}

#[test]
fn effectful_app_data_commands_use_the_real_phased_callback_context() {
    let _state_lock = super::super::custom_config::lock_test_float_out_boy_config_state();
    let firmware = FirmwareTest::new();

    assert_real_config_save_context(&firmware);
    assert_real_config_restore_context();
    assert_real_lock_context();
    assert_real_handtest_restore_context();
    assert_real_flywheel_restore_context();
}

#[test]
fn tune_preparation_releases_the_control_state_lock() {
    let _state_lock = super::super::custom_config::lock_test_float_out_boy_config_state();
    let firmware = FirmwareTest::new();
    let packets = [
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneDefaults.id(),
        ][..],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RuntimeTune.id(),
            0xA3,
            0x21,
            0xA3,
            0x54,
            0xB9,
            0x20,
            0x71,
            0xD4,
            0xA5,
            0x43,
            0x21,
            0xFF,
            0x86,
            0xA5,
            0x47,
            0x63,
            0x82,
        ],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneTilt.id(),
            1,
            15,
            85,
            25,
            30,
        ],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneOther.id(),
            0xFE,
            25,
            20,
            15,
            25,
            7,
            110,
            30,
            20,
            25,
            35,
            40,
            50,
            8,
        ],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Booster.id(),
            0xA3,
            0x04,
            0x21,
            0xF2,
        ],
    ];
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    ));
    let mut expected = state;
    let mut sent = Vec::new();
    for packet in packets {
        assert!(handle_packet(
            &mut expected,
            TimestampTicks::from_ticks(0),
            &mut sent,
            firmware.telemetry(),
            firmware.imu(),
            AppDataPacket::from_bytes(packet),
        ));
    }
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    for packet in packets {
        assert_eq!(
            vescpkg_rs::test_support::invoke_stateful_app_data_handler_with_phase_count::<
                FloatOutBoyAppData,
            >(packet),
            Some(2)
        );
    }
    drop(installed);
    assert_eq!(state, expected);
}
