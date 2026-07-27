use super::{
    FloatOutBoyPackageState,
    config_storage::{
        FLOAT_OUT_BOY_EEPROM_LEN, FirmwareImuMigration, FloatOutBoyConfigLoadOutcome,
        FloatOutBoyEepromImage, FloatOutBoyEepromImageError,
    },
};
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyMode, FloatOutBoyRunState,
};
use crate::package::test_support::{
    FloatOutBoyConfigTestBytes, default_float_out_boy_config_bytes, editable_config_from_bytes,
    editable_config_from_state, sample_all_data_payloads_with_ride_state,
};
use std::{vec, vec::Vec};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{
    Current, FirmwareFloatSetting, ImuMahonyIntegralGain, ImuMahonyProportionalGain,
    MahonyPitchGain, MahonyRollGain, MotorCurrent, Ratio, TimestampTicks,
};

fn handle_config_command(
    firmware: &FirmwareTest,
    state: &mut FloatOutBoyPackageState,
    command: FloatOutBoyAppDataCommand,
    payload: &[u8],
) -> bool {
    let mut packet = vec![FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(), command.id()];
    packet.extend_from_slice(payload);
    state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &packet,
    )
}

fn drain_one_short_beep(state: &mut FloatOutBoyPackageState) -> Vec<(u32, FloatOutBoyBeeperLevel)> {
    (1..=240)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect()
}

fn set_firmware_imu_settings(
    firmware: &FirmwareTest,
    proportional_gain: f32,
    integral_gain: f32,
    acceleration_confidence_decay: f32,
) {
    let settings = firmware.settings();
    settings
        .set_imu_mahony_proportional_gain(
            ImuMahonyProportionalGain::try_new(proportional_gain).unwrap(),
        )
        .unwrap();
    settings
        .set_imu_mahony_integral_gain(ImuMahonyIntegralGain::try_new(integral_gain).unwrap())
        .unwrap();
    settings
        .set_imu_acceleration_confidence_decay(
            Ratio::from_ratio(acceleration_confidence_decay).unwrap(),
        )
        .unwrap();
}

fn assert_firmware_imu_settings(
    firmware: &FirmwareTest,
    proportional_gain: f32,
    integral_gain: f32,
    acceleration_confidence_decay: f32,
) {
    let settings = firmware.settings();
    assert_f32_eq!(
        settings.imu_mahony_proportional_gain().unwrap().value(),
        proportional_gain
    );
    assert_f32_eq!(
        settings.imu_mahony_integral_gain().unwrap().value(),
        integral_gain
    );
    assert_f32_eq!(
        settings.imu_acceleration_confidence_decay().as_ratio(),
        acceleration_confidence_decay
    );
}

#[test]
fn configured_loop_time_uses_float_out_boy_hertz_config() {
    let _firmware = FirmwareTest::new();
    let mut incoming = default_float_out_boy_config_bytes();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert_eq!(state.configured_loop_time_us(), 1201);

    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_hertz(vescpkg_rs::SampleRate::from_hertz(500.0)));
    });
    assert!(state.store_serialized_config(&incoming));

    // Upstream generated serialization places `hertz` after the first
    // seven float16 config fields; `configure(d)` then uses it as
    // `1e6 / d->float_conf.hertz` at `third_party/float-out-boy/src/main.c:190-191`.
    assert_eq!(state.configured_loop_time_us(), 2000);
}

#[test]
fn main_thread_config_load_defers_configure_side_effects_like_refloat() {
    let firmware = FirmwareTest::new();
    set_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert!(!state.startup_configured());
    state.load_persisted_config_on_main_thread();

    assert_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    assert!(!state.startup_configured());

    state.configure_loaded_config_on_main_thread();

    assert!(state.startup_configured());
}

#[test]
fn main_thread_configure_alerts_the_persisted_disabled_state_like_refloat() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut persisted = state.serialized_config;
    assert!(persisted.editor().set_beeper_enabled(true));
    assert!(persisted.editor().set_disabled(true));
    assert!(state.store_serialized_config(persisted.as_bytes()));
    for _ in 0..560 {
        let _ = state.tick_beeper();
    }

    let mut restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );

    let changes: Vec<_> = (1..=560)
        .filter_map(|tick| restarted.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(changes.len(), 7);
    assert_eq!(changes.last(), Some(&(560, FloatOutBoyBeeperLevel::Low)),);
}

#[test]
fn accepted_config_replacement_migrates_legacy_firmware_imu_settings_like_refloat() {
    let firmware = FirmwareTest::new();
    set_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));

    assert_firmware_imu_settings(&firmware, 0.4, 0.0, 0.1);
    assert_eq!(
        state.firmware_imu_migration_for_test(),
        FirmwareImuMigration::Applied
    );
}

#[test]
fn accepted_config_replacement_keeps_current_firmware_imu_settings() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    for proportional_gain in [0.5, 1.0] {
        set_firmware_imu_settings(&firmware, proportional_gain, 0.25, 0.8);

        assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));

        assert_firmware_imu_settings(&firmware, proportional_gain, 0.25, 0.8);
        assert_eq!(
            state.firmware_imu_migration_for_test(),
            FirmwareImuMigration::NotRequired
        );
    }
}

#[test]
fn rejected_legacy_firmware_imu_writes_leave_live_settings_unchanged() {
    let firmware = FirmwareTest::new();
    set_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    firmware.fail_settings_writes();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));

    assert_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    assert_eq!(
        state.firmware_imu_migration_for_test(),
        FirmwareImuMigration::Rejected {
            proportional_gain: true,
            integral_gain: true,
            acceleration_confidence_decay: true,
        }
    );
}

#[test]
fn each_rejected_legacy_firmware_imu_write_has_an_explicit_partial_outcome() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    for (write_results, expected_settings, expected_migration) in [
        (
            [false, true, true],
            [2.0, 0.0, 0.1],
            FirmwareImuMigration::Rejected {
                proportional_gain: true,
                integral_gain: false,
                acceleration_confidence_decay: false,
            },
        ),
        (
            [true, false, true],
            [0.4, 0.25, 0.1],
            FirmwareImuMigration::Rejected {
                proportional_gain: false,
                integral_gain: true,
                acceleration_confidence_decay: false,
            },
        ),
        (
            [true, true, false],
            [0.4, 0.0, 0.8],
            FirmwareImuMigration::Rejected {
                proportional_gain: false,
                integral_gain: false,
                acceleration_confidence_decay: true,
            },
        ),
    ] {
        set_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
        firmware.set_float_setting_write_results(&write_results);

        assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));

        assert_firmware_imu_settings(
            &firmware,
            expected_settings[0],
            expected_settings[1],
            expected_settings[2],
        );
        assert_eq!(state.firmware_imu_migration_for_test(), expected_migration);
    }
}

#[test]
fn invalid_legacy_firmware_imu_read_is_diagnosed_without_writes() {
    let firmware = FirmwareTest::new();
    set_firmware_imu_settings(&firmware, 2.0, 0.25, 0.8);
    firmware.set_raw_float_setting(FirmwareFloatSetting::ImuMahonyKp, f32::NAN);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert!(state.store_serialized_config(&default_float_out_boy_config_bytes()));

    assert!(
        firmware
            .settings()
            .get_float(FirmwareFloatSetting::ImuMahonyKp)
            .is_nan()
    );
    assert_f32_eq!(
        firmware
            .settings()
            .imu_mahony_integral_gain()
            .unwrap()
            .value(),
        0.25
    );
    assert_f32_eq!(
        firmware
            .settings()
            .imu_acceleration_confidence_decay()
            .as_ratio(),
        0.8
    );
    assert_eq!(
        state.firmware_imu_migration_for_test(),
        FirmwareImuMigration::InvalidRead
    );
}

#[test]
fn eeprom_image_conversion_keeps_the_fixed_tail_deterministic() {
    let config = FloatOutBoyConfigImage::defaults();
    let image = FloatOutBoyEepromImage::from(config);
    let bytes = image.into_bytes();

    assert_eq!(&bytes[..config.as_bytes().len()], config.as_bytes());
    assert!(
        bytes[config.as_bytes().len()..]
            .iter()
            .all(|byte| *byte == 0)
    );
    assert_eq!(
        FloatOutBoyConfigImage::try_from(FloatOutBoyEepromImage::from_bytes(&bytes)),
        Ok(config)
    );
}

#[test]
fn eeprom_image_conversion_rejects_a_bad_signature() {
    let config = FloatOutBoyConfigImage::defaults();
    let mut bytes = FloatOutBoyEepromImage::from(config).into_bytes();
    bytes[0] ^= 0xff;

    assert_eq!(
        FloatOutBoyConfigImage::try_from(FloatOutBoyEepromImage::from_bytes(&bytes)),
        Err(FloatOutBoyEepromImageError)
    );
}

#[test]
fn config_save_restore_and_startup_round_trip_custom_eeprom() {
    let firmware = FirmwareTest::new();
    firmware.set_clock_ticks(42);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.idle_ticks = TimestampTicks::from_ticks(7);
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    assert!(state.serialized_config.editor().set_disabled(true));
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(15.0))
    );
    let saved = state.serialized_config;

    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::ConfigSave,
        &[],
    ));
    let mut log = [0; 32];
    let len = firmware.copy_last_log(&mut log);
    assert_eq!(&log[..len], b"Config written: 276B");
    assert_eq!(
        drain_one_short_beep(&mut state),
        [
            (80, FloatOutBoyBeeperLevel::Low),
            (160, FloatOutBoyBeeperLevel::High),
            (240, FloatOutBoyBeeperLevel::Low),
        ],
    );

    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(5.0))
    );
    assert!(state.serialized_config.editor().set_disabled(false));
    state.refresh_config_runtime_state();
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Startup,
    );
    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::ConfigRestore,
        &[],
    ));
    assert_eq!(state.serialized_config, saved);
    assert_eq!(state.idle_ticks, TimestampTicks::from_ticks(7));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Disabled,
    );
    assert_eq!(state.tick_beeper(), None);

    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_eq!(restarted.idle_ticks, TimestampTicks::from_ticks(42));
    assert_eq!(restarted.serialized_config, saved);
    assert_eq!(
        restarted
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Disabled,
    );
}

#[test]
fn startup_distinguishes_eeprom_read_failure_from_an_invalid_image() {
    let firmware = FirmwareTest::new();
    let failed_address = vescpkg_rs::CustomEepromAddress::from_index(2).expect("test address fits");
    firmware.fail_eeprom_read(failed_address);

    let read_failed = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_eq!(
        read_failed.config_load_outcome_for_test(),
        FloatOutBoyConfigLoadOutcome::DefaultAfterReadFailure,
    );
    assert_eq!(
        read_failed.serialized_config,
        FloatOutBoyConfigImage::defaults(),
    );
    let mut log = [0; 64];
    let len = firmware.copy_last_log(&mut log);
    assert_eq!(&log[..len], b"Failed to read config, using defaults.");
    drop(firmware);

    let firmware = FirmwareTest::new();
    assert!(
        firmware
            .eeprom()
            .write_bytes(&[0; FLOAT_OUT_BOY_EEPROM_LEN])
            .is_ok()
    );
    let invalid = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_eq!(
        invalid.config_load_outcome_for_test(),
        FloatOutBoyConfigLoadOutcome::DefaultAfterInvalidImage,
    );
    assert_eq!(
        invalid.serialized_config,
        FloatOutBoyConfigImage::defaults(),
    );
    let mut log = [0; 64];
    let len = firmware.copy_last_log(&mut log);
    assert_eq!(
        &log[..len],
        b"Failed to deserialize config, using defaults.",
    );
}

#[test]
fn config_save_failure_has_no_write_acknowledgement() {
    let firmware = FirmwareTest::new();
    let address = vescpkg_rs::CustomEepromAddress::from_index(0).expect("zero fits");
    firmware.fail_eeprom_write(address);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::ConfigSave,
        &[],
    ));
    assert_eq!(state.tick_beeper(), None);
    assert_eq!(firmware.eeprom().read(address), None);
    let mut log = [0; 32];
    let len = firmware.copy_last_log(&mut log);
    assert_eq!(&log[..len], b"Failed to write config.");
}

#[test]
fn successful_config_save_starts_led_confirmation_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    assert!(state.store_serialized_config(&bytes));
    let packet = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        FloatOutBoyAppDataCommand::ConfigSave.id(),
    ];

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(1_500),
        &mut |_bytes| true,
        &packet,
    ));
    assert_eq!(state.internal_led_confirmation_start_for_test(), Some(0.15));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(2_000),
        &mut |_bytes| true,
        &packet,
    ));
    assert_eq!(state.internal_led_confirmation_start_for_test(), Some(0.15));
}

#[test]
fn tune_defaults_resets_only_the_fields_named_by_float_out_boy() {
    let firmware = FirmwareTest::new();
    let defaults = default_float_out_boy_config_bytes();
    let mut changed = defaults;
    for range in [4..18, 67..75, 77..79, 91..101, 102..118, 130..175] {
        changed[range].fill(0xAA);
    }
    changed[242] = 0;
    changed[48] = 0x55;
    changed[75] = 0x66;
    changed[118] = 0x77;
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.replace_serialized_config_for_test(
        &FloatOutBoyConfigImage::from_serialized(&changed).expect("valid test image"),
    );

    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::TuneDefaults,
        &[],
    ));
    let actual = state.serialized_config.as_bytes();
    for range in [4..18, 67..75, 77..79, 91..101, 102..118, 130..175] {
        assert_eq!(&actual[range.clone()], &defaults[range]);
    }
    assert_eq!(actual[242], defaults[242]);
    assert_eq!(actual[48], 0x55);
    assert_eq!(actual[75], 0x66);
    assert_eq!(actual[118], 0x77);
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn lock_restores_persisted_config_then_disables_and_saves() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(15.0))
    );
    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::ConfigSave,
        &[],
    ));
    let _ = drain_one_short_beep(&mut state);
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(5.0))
    );

    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::Lock,
        &[1],
    ));
    assert_f32_eq!(
        state.balance_config_for_test().kp().as_amps_per_degree(),
        15.0
    );
    assert!(state.serialized_config.metadata().disabled());
    assert!(matches!(
        state
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Disabled
    ));
    assert_eq!(
        drain_one_short_beep(&mut state),
        [
            (80, FloatOutBoyBeeperLevel::Low),
            (160, FloatOutBoyBeeperLevel::High),
            (240, FloatOutBoyBeeperLevel::Low),
        ],
    );

    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert!(restarted.serialized_config.metadata().disabled());
}

#[test]
fn successful_lock_starts_led_confirmation_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    assert!(state.store_serialized_config(&bytes));
    let packet = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        FloatOutBoyAppDataCommand::Lock.id(),
        1,
    ];

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(1_500),
        &mut |_bytes| true,
        &packet,
    ));
    assert_eq!(state.internal_led_confirmation_start_for_test(), Some(0.15));
}

#[test]
fn lock_is_ignored_while_running_like_float_out_boy() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    ));
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(15.0))
    );
    let before = state.serialized_config;

    assert!(handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::Lock,
        &[1],
    ));

    assert_eq!(state.serialized_config, before);
    assert_eq!(state.tick_beeper(), None);
    assert_eq!(
        FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup())
            .serialized_config,
        FloatOutBoyConfigImage::defaults(),
    );
}

#[test]
fn lock_rejects_a_missing_disabled_flag() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let before = state.serialized_config;

    assert!(!handle_config_command(
        &firmware,
        &mut state,
        FloatOutBoyAppDataCommand::Lock,
        &[],
    ));

    assert_eq!(state.serialized_config, before);
}

#[test]
fn default_config_decodes_pid_scales_like_float_out_boy_settings() {
    let state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    // Float Out Boy generated settings serialize `kp` with scale 10 at
    // `third_party/float-out-boy/src/conf/settings.xml:28-54`, `kp2` with scale
    // 100 at `third_party/float-out-boy/src/conf/settings.xml:55-84`, and
    // `kp2_brake` with scale 100 at
    // `third_party/float-out-boy/src/conf/settings.xml:199-222`.
    let balance = state.balance_config_for_test();
    assert_f32_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_f32_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_f32_eq!(balance.kp2_brake().value(), 1.0);
}

#[test]
fn ki_limit_accepts_zero_sentinel_and_rejects_invalid_values() {
    let mut bytes = default_float_out_boy_config_bytes();
    // Float Out Boy's VESC Tool metadata defines zero as the disabled-limit sentinel
    // at `third_party/float-out-boy/src/conf/settings.xml:1679-1707`.
    bytes.edit_float_out_boy_config(|config| {
        assert!(config.set_ki_limit(MotorCurrent::new(Current::ZERO)));
        assert!(!config.set_ki_limit(MotorCurrent::new(Current::from_amps(-1.0))));
        assert!(!config.set_ki_limit(MotorCurrent::new(Current::from_amps(f32::NAN))));
        assert!(!config.set_ki_limit(MotorCurrent::new(Current::from_amps(f32::INFINITY))));
    });

    assert_eq!(
        editable_config_from_bytes(&bytes)
            .balance()
            .ki_limit()
            .current(),
        Current::ZERO
    );
}

#[test]
fn default_scaled_config_fields_decode_to_semantic_values() {
    let config = editable_config_from_bytes(&default_float_out_boy_config_bytes());
    let balance = config.balance();
    let startup = config.startup();

    // C map: generated float16 fields use the offsets and scales in
    // `third_party/float-out-boy/src/conf/settings.xml:28-222,3916-3984` and are
    // decoded by `third_party/float-out-boy/src/conf/buffer.c:208-210`.
    assert_eq!(config.filter().mahony_kp(), MahonyPitchGain::new(2.0));
    assert_eq!(config.filter().mahony_kp_roll(), MahonyRollGain::new(1.4));
    assert_f32_eq!(
        config.motor_control().brake_current().current().as_amps(),
        6.0
    );
    assert_f32_eq!(startup.pitch_tolerance().as_degrees(), 4.0);
    assert_f32_eq!(startup.roll_tolerance().as_degrees(), 45.0);
    assert_f32_eq!(startup.startup_speed().as_degrees_per_second(), 30.0);
    assert_f32_eq!(config.low_voltage_pushback_angle().as_degrees(), 10.0);
    assert_f32_eq!(config.low_voltage_threshold().as_volts(), 3.0);
    assert_f32_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_f32_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_f32_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.005);
    assert_f32_eq!(balance.kp_brake().value(), 1.0);
    assert_f32_eq!(balance.kp2_brake().value(), 1.0);
    assert_f32_eq!(balance.ki_limit().current().as_amps(), 30.0);
    assert_f32_eq!(balance.booster_angle().as_degrees(), 8.0);
    assert_f32_eq!(balance.booster_ramp().as_degrees(), 4.0);
    assert_f32_eq!(balance.booster_current().current().as_amps(), 0.0);
    assert_f32_eq!(balance.brake_booster_angle().as_degrees(), 8.0);
    assert_f32_eq!(balance.brake_booster_ramp().as_degrees(), 4.0);
    assert_f32_eq!(balance.brake_booster_current().current().as_amps(), 0.0);
    assert_f32_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        0.0
    );
    assert_f32_eq!(config.remote_throttle().grace_period().as_seconds(), 10.0);
}

#[test]
fn default_led_runtime_flags_follow_generated_config_image() {
    let config = FloatOutBoyConfigImage::defaults();

    assert!(config.leds_enabled());
    assert!(config.headlights_enabled());
    assert!(config.lights_off_when_lifted());
    assert_eq!(config.as_bytes()[175], 1);
    assert_eq!(config.as_bytes()[176], 1);
    assert_eq!(config.as_bytes()[179], 1);
}

#[test]
fn semantic_config_writes_round_trip_through_generated_storage() {
    let mut bytes = default_float_out_boy_config_bytes();
    bytes.edit_float_out_boy_config(|config| {
        assert!(config.set_startup_pitch_tolerance(vescpkg_rs::AngleDegrees::from_degrees(3.5)));
        assert!(config.set_startup_roll_tolerance(vescpkg_rs::AngleDegrees::from_degrees(42.0)));
        assert!(
            config.set_startup_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(25.0,))
        );
        assert!(
            config.set_remote_throttle_current_max(vescpkg_rs::MotorCurrent::new(
                vescpkg_rs::Current::from_amps(12.0),
            ))
        );
        assert!(
            config.set_remote_throttle_grace_period(vescpkg_rs::VescSeconds::from_seconds(1.5,))
        );
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
        assert!(config.set_kp2(vescpkg_rs::RateCurrentGain::new(0.75)));
        assert!(config.set_ki(vescpkg_rs::IntegralCurrentGain::new(0.004)));
        assert!(config.set_kp_brake(vescpkg_rs::PidScale::new(0.8)));
        assert!(config.set_booster_angle(vescpkg_rs::AngleDegrees::from_degrees(7.0)));
        assert!(config.set_booster_ramp(vescpkg_rs::AngleDegrees::from_degrees(2.5)));
        assert!(config.set_booster_current(vescpkg_rs::MotorCurrent::new(
            vescpkg_rs::Current::from_amps(4.0),
        )));
        assert!(config.set_brake_booster_angle(vescpkg_rs::AngleDegrees::from_degrees(6.0)));
        assert!(config.set_brake_booster_ramp(vescpkg_rs::AngleDegrees::from_degrees(2.0)));
        assert!(
            config.set_brake_booster_current(vescpkg_rs::MotorCurrent::new(
                vescpkg_rs::Current::from_amps(3.0),
            ))
        );
    });

    let config = editable_config_from_bytes(&bytes);
    let balance = config.balance();
    assert_f32_eq!(config.startup().pitch_tolerance().as_degrees(), 3.5);
    assert_f32_eq!(config.startup().roll_tolerance().as_degrees(), 42.0);
    assert_f32_eq!(
        config.startup().startup_speed().as_degrees_per_second(),
        25.0
    );
    assert_f32_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        12.0
    );
    assert_f32_eq!(config.remote_throttle().grace_period().as_seconds(), 1.5);
    assert_f32_eq!(balance.kp().as_amps_per_degree(), 15.0);
    assert_f32_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.75);
    assert_f32_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.004);
    assert_f32_eq!(balance.kp_brake().value(), 0.8);
    assert_f32_eq!(balance.booster_angle().as_degrees(), 7.0);
    assert_f32_eq!(balance.booster_ramp().as_degrees(), 2.5);
    assert_f32_eq!(balance.booster_current().current().as_amps(), 4.0);
    assert_f32_eq!(balance.brake_booster_angle().as_degrees(), 6.0);
    assert_f32_eq!(balance.brake_booster_ramp().as_degrees(), 2.0);
    assert_f32_eq!(balance.brake_booster_current().current().as_amps(), 3.0);
}

#[test]
fn parking_brake_mode_field_decodes_known_and_unknown_values() {
    let mut bytes = default_float_out_boy_config_bytes();
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::FloatOutBoyParkingBrakeMode::Idle
    );

    bytes[101] = 0xff;
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::FloatOutBoyParkingBrakeMode::Unknown(0xff)
    );
}

#[test]
fn handtest_safety_overrides_encode_named_semantic_values() {
    let mut config = FloatOutBoyConfigImage::defaults();
    assert!(config.editor().apply_handtest_safety_overrides());

    let balance = config.balance();
    assert_f32_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.0);
    assert_f32_eq!(balance.kp_brake().value(), 1.0);
    assert_f32_eq!(balance.kp2_brake().value(), 1.0);
    assert_f32_eq!(balance.booster_angle().as_degrees(), 100.0);
    assert_f32_eq!(balance.brake_booster_angle().as_degrees(), 100.0);
    assert_f32_eq!(config.faults().pitch_delay().as_seconds(), 0.05);
    assert_f32_eq!(config.faults().roll_delay().as_seconds(), 0.05);

    // These currently unwired tune categories have no domain readers yet;
    // verify their generated float16 storage at the serializer boundary.
    for offset in [67, 71, 126, 128, 130, 145, 147] {
        assert_eq!(&config.as_bytes()[offset..offset + 2], &[0, 0]);
    }
}

#[test]
fn float_out_boy_config_image_rejects_short_payload_like_confparser() {
    let bytes = default_float_out_boy_config_bytes();

    // C map: `third_party/float-out-boy/src/conf/confparser.h:11-12` fixes the serialized config length,
    // so shorter payloads must fail before any typed parsing or state mutation.
    assert!(FloatOutBoyConfigImage::from_serialized(&bytes[..275]).is_none());
}

#[test]
fn float_out_boy_config_image_rejects_bad_signature_like_confparser() {
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[0] ^= 0xff;

    // C map: `third_party/float-out-boy/src/conf/confparser.c:187-190` rejects the signature before field reads.
    assert!(FloatOutBoyConfigImage::from_serialized(&bytes).is_none());
}

#[test]
fn store_serialized_config_rejects_short_payload_like_float_out_boy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let bytes = default_float_out_boy_config_bytes();

    assert!(!state.store_serialized_config(&bytes[..275]));

    // C map: upstream rejects truncated custom-config writes before storing them at
    // `third_party/float-out-boy/src/main.c:2360-2368`.
    assert_eq!(
        state.serialized_config(),
        default_float_out_boy_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_bad_signature_like_float_out_boy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[0] ^= 0xff;

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects bad config signatures before deserializing any fields at
    // `third_party/float-out-boy/src/conf/confparser.c:187-190`.
    assert_eq!(
        state.serialized_config(),
        default_float_out_boy_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_special_modes_like_float_out_boy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::HandTest,
    ));
    let bytes = default_float_out_boy_config_bytes();

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects config writes outside `MODE_NORMAL` at
    // `third_party/float-out-boy/src/main.c:2362-2365`.
    assert_eq!(
        state.serialized_config(),
        default_float_out_boy_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_clears_default_and_keeps_enabled_while_running_like_float_out_boy() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    ));
    let mut bytes = default_float_out_boy_config_bytes();
    bytes.edit_float_out_boy_config(|config| {
        assert!(config.set_disabled(true));
        assert!(config.set_meta_is_default(true));
    });

    assert!(state.store_serialized_config(&bytes));

    // C map: running writes clear `disabled` at `third_party/float-out-boy/src/main.c:2369-2372`
    // and always clear `meta.is_default` at `third_party/float-out-boy/src/main.c:2375-2377`.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().disabled());
    assert!(!current.metadata().is_default());
}

#[test]
fn config_write_acknowledgement_wins_over_the_following_configure_alert_like_refloat() {
    let _firmware = FirmwareTest::new();
    for (old_beeper_enabled, disabled, expected_run_state, expected_changes, expected_last) in [
        (
            false,
            false,
            FloatOutBoyRunState::Ready,
            3,
            (240, FloatOutBoyBeeperLevel::Low),
        ),
        (
            false,
            true,
            FloatOutBoyRunState::Disabled,
            7,
            (560, FloatOutBoyBeeperLevel::Low),
        ),
        (
            true,
            true,
            FloatOutBoyRunState::Disabled,
            3,
            (240, FloatOutBoyBeeperLevel::Low),
        ),
    ] {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ));
        let mut active_config = state.serialized_config;
        assert!(
            active_config
                .editor()
                .set_beeper_enabled(old_beeper_enabled),
        );
        state.replace_active_config(&active_config);
        let mut bytes = default_float_out_boy_config_bytes();
        bytes.edit_float_out_boy_config(|config| {
            assert!(config.set_beeper_enabled(true));
            assert!(config.set_disabled(disabled));
        });

        assert!(state.store_serialized_config(&bytes));

        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            expected_run_state,
        );
        let changes: Vec<_> = (1..=560)
            .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
            .collect();
        assert_eq!(changes.len(), expected_changes);
        assert_eq!(changes.last(), Some(&expected_last));
    }
}

#[test]
fn failed_config_write_does_not_reconfigure_or_acknowledge() {
    let firmware = FirmwareTest::new();
    let address = vescpkg_rs::CustomEepromAddress::from_index(0).expect("zero fits");
    firmware.fail_eeprom_write(address);
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let original = state.serialized_config;
    let mut bytes = default_float_out_boy_config_bytes();
    bytes.edit_float_out_boy_config(|config| {
        assert!(config.set_beeper_enabled(true));
        assert!(config.set_disabled(true));
    });

    assert!(!state.store_serialized_config(&bytes));

    assert_eq!(state.serialized_config, original);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready,
    );
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn interrupted_config_write_cannot_boot_a_mixed_image() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut old = default_float_out_boy_config_bytes();
    old.edit_float_out_boy_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
    });
    assert!(state.store_serialized_config(&old));

    let interrupted_address =
        vescpkg_rs::CustomEepromAddress::from_index(2).expect("test address fits");
    firmware.fail_eeprom_write(interrupted_address);
    let mut new = default_float_out_boy_config_bytes();
    new.edit_float_out_boy_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(5.0)));
    });

    assert!(!state.store_serialized_config(&new));

    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_eq!(
        restarted.serialized_config,
        FloatOutBoyConfigImage::defaults(),
    );
}

fn assert_interrupted_config_write_fails_safe(successful_writes: usize) {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut old = default_float_out_boy_config_bytes();
    old.edit_float_out_boy_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
    });
    assert!(state.store_serialized_config(&old));
    let old = state.serialized_config;

    firmware.fail_eeprom_write_after(successful_writes);
    let mut new = default_float_out_boy_config_bytes();
    new.edit_float_out_boy_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(5.0)));
    });
    assert!(!state.store_serialized_config(&new));

    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    let expected = if successful_writes == 0 {
        old
    } else {
        FloatOutBoyConfigImage::defaults()
    };
    assert_eq!(restarted.serialized_config, expected);
    assert_eq!(state.serialized_config, old);
}

#[test]
fn interrupted_config_write_fails_safe_at_every_commit_phase() {
    for successful_writes in [0, 1, 80] {
        assert_interrupted_config_write_fails_safe(successful_writes);
    }
}

#[test]
fn store_serialized_config_persists_for_restart_like_float_out_boy_set_cfg() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();
    bytes.edit_float_out_boy_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
    });

    assert!(state.store_serialized_config(&bytes));
    let mut persisted = [0; FLOAT_OUT_BOY_EEPROM_LEN];
    assert!(firmware.eeprom().read_bytes(&mut persisted).is_ok());
    assert_eq!(
        &persisted[..state.serialized_config.as_bytes().len()],
        state.serialized_config.as_bytes(),
    );
    assert!(
        persisted[state.serialized_config.as_bytes().len()..]
            .iter()
            .all(|byte| *byte == 0)
    );

    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_f32_eq!(
        restarted
            .serialized_config
            .balance()
            .kp()
            .as_amps_per_degree(),
        15.0,
    );
}

#[test]
fn storing_led_config_replaces_internal_renderer_immediately() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();

    assert!(state.internal_leds.is_none());
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    assert!(state.store_serialized_config(&bytes));
    assert!(state.internal_leds.is_some());

    bytes[227] = crate::lcm::FloatOutBoyLedMode::Off.id();
    assert!(state.store_serialized_config(&bytes));
    assert!(state.internal_leds.is_none());
}

#[test]
fn failed_internal_led_teardown_retains_runtime_for_retry() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    assert!(state.store_serialized_config(&bytes));

    assert!(!state.destroy_internal_leds_with(|_| false));
    assert!(state.internal_leds.is_some());
    assert!(!state.internal_leds_operational());

    assert!(state.destroy_internal_leds_with(|_| true));
    assert!(state.internal_leds.is_none());
}

#[test]
fn storing_internal_led_config_while_both_footpads_are_pressed_skips_setup() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();

    assert!(state.store_serialized_config(&bytes));
    assert!(state.internal_leds.is_none());
    assert!(!state.internal_leds_operational());
}

#[test]
fn startup_defers_internal_led_setup_until_after_the_physical_footpad_sample() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut bytes = default_float_out_boy_config_bytes();
    bytes[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    state.serialized_config = editable_config_from_bytes(&bytes);

    state.configure_loaded_config_on_main_thread();

    assert!(state.internal_leds.is_none());

    state.setup_loaded_led_hardware_after_threads(
        vescpkg_rs::AdcVoltage::new(vescpkg_rs::Voltage::from_volts(3.0)),
        vescpkg_rs::AdcVoltage::new(vescpkg_rs::Voltage::from_volts(3.0)),
    );

    assert_eq!(
        state.all_data_payloads().base().footpad().state(),
        crate::FloatOutBoyFootpadState::Both,
    );
    assert!(state.internal_leds.is_none());
}
