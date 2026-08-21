use super::{FloatOutBoyAppData, handle_float_out_boy_app_data_packet};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAppDataCommand,
    FloatOutBoyMode, FloatOutBoyRunState,
};
use crate::package::FloatOutBoyPackageState;
use crate::package::protocol::encode_float_out_boy_get_realtime_data_response;
use crate::package::test_support::{
    default_float_out_boy_config_bytes, editable_config_from_state, sample_all_data_payloads,
    sample_all_data_payloads_with_ride_state,
};
use std::vec::Vec;
use vescpkg_rs::AppDataPacket;
use vescpkg_rs::TimestampTicks;
use vescpkg_rs::test_support::{FirmwareTest, invoke_stateful_app_data_handler};

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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
    let expected = encode_float_out_boy_get_realtime_data_response(&payloads);
    let mut state = FloatOutBoyPackageState::new(payloads);
    let request = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
        &[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get()][..],
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
    let expected = *state.serialized_config();
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::HandTest.id(),
            1,
        ]),
        Some(false)
    );
    let installed =
        super::super::custom_config::install_test_float_out_boy_runtime_state(&mut state);
    assert!(installed.is_some());

    assert!(invoke_stateful_app_data_handler::<FloatOutBoyAppData>(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
