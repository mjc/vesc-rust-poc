use super::*;
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand,
    FloatOutBoyMode, FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterTorque, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRideState, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition,
};
use crate::package::test_support::{
    balance_filter_with_pitch, editable_config_from_bytes, editable_config_from_state,
    sample_all_data_payloads_with_ride_state, tick_float_out_boy_state_and_handle_packet,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn handtest_request_selects_mode_only_while_ready_like_float_out_boy() {
    let _firmware = FirmwareTest::new();
    for (run_state, mode, expected) in [
        (
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
            FloatOutBoyMode::HandTest,
        ),
        (
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
            FloatOutBoyMode::Normal,
        ),
        (
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Flywheel,
            FloatOutBoyMode::Flywheel,
        ),
    ] {
        let mut state =
            FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(run_state, mode));
        FloatOutBoyHandtestRequest::Enable.apply_to(&mut state);
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            expected
        );
    }
}

#[test]
fn handtest_packet_toggles_ready_mode_and_safety_config_like_float_out_boy_qml() {
    let _firmware = FirmwareTest::new();
    // QML sends COMMAND_HANDTEST at `float-out-boy/ui.qml.in:764-768`; C toggles
    // mode and temporary safety config at `third_party/float-out-boy/src/main.c:1421-1450`.
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let original_config = *state.serialized_config();

    assert!(state.handle_handtest_packet(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::HandTest.id(),
        1,
    ]));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::HandTest
    );
    let mut expected_handtest_config = editable_config_from_bytes(&original_config);
    assert!(
        expected_handtest_config
            .editor()
            .apply_handtest_safety_overrides()
    );
    assert_eq!(
        state.serialized_config(),
        expected_handtest_config.as_bytes()
    );

    assert!(state.handle_handtest_packet(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::HandTest.id(),
        0,
    ]));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Normal
    );
    assert_eq!(state.serialized_config(), &original_config);
}

#[test]
fn handtest_disable_only_refreshes_idle_epoch_like_refloat_configure() {
    let firmware = FirmwareTest::new();
    firmware.set_clock_ticks(42);
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    state.idle_ticks.restart(TimestampTicks::from_ticks(7));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(99),
        &mut |_| true,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::HandTest.id(),
            1,
        ],
    ));
    assert_eq!(state.idle_ticks.started(), TimestampTicks::from_ticks(7));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(99),
        &mut |_| true,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::HandTest.id(),
            0,
        ],
    ));
    assert_eq!(state.idle_ticks.started(), TimestampTicks::from_ticks(42));
}

#[test]
fn handtest_disable_restores_eeprom_not_the_enable_time_image_like_float_out_boy() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let mut persisted = editable_config_from_state(&state);
    assert!(persisted.editor().set_kp(AngleCurrentGain::new(1.2)));
    assert!(persisted.editor().set_beeper_enabled(true));
    let persisted = *persisted.as_bytes();
    assert!(state.store_serialized_config(&persisted));
    for _ in 0..240 {
        let _ = state.tick_beeper();
    }

    let mut volatile = editable_config_from_state(&state);
    assert!(volatile.editor().set_kp(AngleCurrentGain::new(-9.0)));
    state.replace_serialized_config_for_test(&volatile);

    assert!(state.handle_handtest_packet(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::HandTest.id(),
        1,
    ]));
    assert!(state.handle_handtest_packet(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::HandTest.id(),
        0,
    ]));

    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        1.2
    );
    let changes: Vec<_> = (1..=18)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (6, FloatOutBoyBeeperLevel::Low),
            (12, FloatOutBoyBeeperLevel::High),
            (18, FloatOutBoyBeeperLevel::Low),
        ],
    );
}

#[test]
fn app_data_handtest_running_recenters_start_setpoint_like_float_out_boy_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::HandTest,
    );
    let base = payloads.base();
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::HandTest,
        FloatOutBoySetpointAdjustment::Centering,
        FloatOutBoyStopCondition::None,
    );
    let board = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0));
    let zero = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(board, zero, zero, zero, zero, zero);
    let base = FloatOutBoyAllDataBasePayload::new(
        FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        setpoints,
        FloatOutBoyRealtimeBoosterTorque::new(MotorTorque::ZERO),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::from_groups(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(0.0)));
    let mut config = editable_config_from_state(&state);
    assert!(
        config
            .editor()
            .set_startup_speed(AngularVelocity::from_degrees_per_second(50.0))
    );
    state.replace_serialized_config_for_test(&config);

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    // Float Out Boy RUNNING `SAT_CENTERING` uses startup speed and the fixed
    // 500 Hz main loop via
    // `get_setpoint_adjustment_step_size` at
    // `third_party/float-out-boy/src/main.c:304-310`; `rate_limitf` applies that
    // step toward target zero at `third_party/float-out-boy/src/utils.c:25-33`,
    // and the main loop publishes the new setpoint at
    // `third_party/float-out-boy/src/main.c:869-875`.
    assert_f32_eq!(base.setpoints().board().angle().as_degrees(), 1.9);
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
}
