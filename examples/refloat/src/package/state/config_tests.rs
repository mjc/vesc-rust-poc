use super::{RefloatPackageState, config_storage::REFLOAT_EEPROM_LEN};
use crate::beeper::RefloatBeeperLevel;
use crate::config::RefloatConfigImage;
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataPayloads, RefloatAppDataCommand, RefloatMode,
    RefloatRunState,
};
use crate::package::test_support::{
    RefloatConfigTestBytes, default_refloat_config_bytes, editable_config_from_bytes,
    editable_config_from_state, sample_all_data_payloads_with_ride_state,
};
use std::{vec, vec::Vec};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{Current, MahonyPitchGain, MahonyRollGain, MotorCurrent, TimestampTicks};

fn handle_config_command(
    firmware: &FirmwareTest,
    state: &mut RefloatPackageState,
    command: RefloatAppDataCommand,
    payload: &[u8],
) -> bool {
    let mut packet = vec![REFLOAT_APP_DATA_PACKAGE_ID.get(), command.id()];
    packet.extend_from_slice(payload);
    state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &packet,
    )
}

fn drain_one_short_beep(state: &mut RefloatPackageState) -> Vec<(u32, RefloatBeeperLevel)> {
    (1..=240)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect()
}

#[test]
fn configured_loop_time_uses_refloat_hertz_config() {
    let _firmware = FirmwareTest::new();
    let mut incoming = default_refloat_config_bytes();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert_eq!(state.configured_loop_time_us(), 1201);

    incoming.edit_refloat_config(|config| {
        assert!(config.set_hertz(vescpkg_rs::SampleRate::from_hertz(500.0)));
    });
    assert!(state.store_serialized_config(&incoming));

    // Upstream generated serialization places `hertz` after the first
    // seven float16 config fields; `configure(d)` then uses it as
    // `1e6 / d->float_conf.hertz` at `third_party/refloat/src/main.c:190-191`.
    assert_eq!(state.configured_loop_time_us(), 2000);
}

#[test]
fn config_save_restore_and_startup_round_trip_custom_eeprom() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
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
        RefloatAppDataCommand::ConfigSave,
        &[],
    ));
    assert_eq!(
        drain_one_short_beep(&mut state),
        [
            (80, RefloatBeeperLevel::Low),
            (160, RefloatBeeperLevel::High),
            (240, RefloatBeeperLevel::Low),
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
        RefloatRunState::Startup,
    );
    assert!(handle_config_command(
        &firmware,
        &mut state,
        RefloatAppDataCommand::ConfigRestore,
        &[],
    ));
    assert_eq!(state.serialized_config, saved);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Disabled,
    );
    assert_eq!(state.tick_beeper(), None);

    let restarted =
        RefloatPackageState::from_persisted_config(RefloatAllDataPayloads::source_startup());
    assert_eq!(restarted.serialized_config, saved);
    assert_eq!(
        restarted
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Disabled,
    );
}

#[test]
fn config_save_failure_has_no_write_acknowledgement() {
    let firmware = FirmwareTest::new();
    let address = vescpkg_rs::CustomEepromAddress::from_index(0).expect("zero fits");
    firmware.fail_eeprom_write(address);
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(handle_config_command(
        &firmware,
        &mut state,
        RefloatAppDataCommand::ConfigSave,
        &[],
    ));
    assert_eq!(state.tick_beeper(), None);
    assert_eq!(firmware.eeprom().read(address), None);
}

#[test]
fn tune_defaults_resets_only_the_fields_named_by_refloat() {
    let firmware = FirmwareTest::new();
    let defaults = default_refloat_config_bytes();
    let mut changed = defaults;
    for range in [4..18, 67..75, 77..79, 91..101, 102..118, 130..175] {
        changed[range].fill(0xAA);
    }
    changed[242] = 0;
    changed[48] = 0x55;
    changed[75] = 0x66;
    changed[118] = 0x77;
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    state.replace_serialized_config_for_test(
        RefloatConfigImage::from_serialized(&changed).expect("valid test image"),
    );

    assert!(handle_config_command(
        &firmware,
        &mut state,
        RefloatAppDataCommand::TuneDefaults,
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
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
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
        RefloatAppDataCommand::ConfigSave,
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
        RefloatAppDataCommand::Lock,
        &[1],
    ));
    assert_eq!(
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
        RefloatRunState::Disabled
    ));
    assert_eq!(
        drain_one_short_beep(&mut state),
        [
            (80, RefloatBeeperLevel::Low),
            (160, RefloatBeeperLevel::High),
            (240, RefloatBeeperLevel::Low),
        ],
    );

    let restarted =
        RefloatPackageState::from_persisted_config(RefloatAllDataPayloads::source_startup());
    assert!(restarted.serialized_config.metadata().disabled());
}

#[test]
fn lock_is_ignored_while_running_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Normal,
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
        RefloatAppDataCommand::Lock,
        &[1],
    ));

    assert_eq!(state.serialized_config, before);
    assert_eq!(state.tick_beeper(), None);
    assert_eq!(
        RefloatPackageState::new(RefloatAllDataPayloads::source_startup()).serialized_config,
        RefloatConfigImage::defaults(),
    );
}

#[test]
fn lock_rejects_a_missing_disabled_flag() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    let before = state.serialized_config;

    assert!(!handle_config_command(
        &firmware,
        &mut state,
        RefloatAppDataCommand::Lock,
        &[],
    ));

    assert_eq!(state.serialized_config, before);
}

#[test]
fn default_config_decodes_pid_scales_like_refloat_settings() {
    let state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    // Refloat generated settings serialize `kp` with scale 10 at
    // `third_party/refloat/src/conf/settings.xml:28-54`, `kp2` with scale
    // 100 at `third_party/refloat/src/conf/settings.xml:55-84`, and
    // `kp2_brake` with scale 100 at
    // `third_party/refloat/src/conf/settings.xml:199-222`.
    let balance = state.balance_config_for_test();
    assert_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_eq!(balance.kp2_brake().value(), 1.0);
}

#[test]
fn ki_limit_accepts_zero_sentinel_and_rejects_invalid_values() {
    let mut bytes = default_refloat_config_bytes();
    // Refloat's VESC Tool metadata defines zero as the disabled-limit sentinel
    // at `third_party/refloat/src/conf/settings.xml:1679-1707`.
    bytes.edit_refloat_config(|config| {
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
    let config = editable_config_from_bytes(&default_refloat_config_bytes());
    let balance = config.balance();
    let startup = config.startup();

    // C map: generated float16 fields use the offsets and scales in
    // `third_party/refloat/src/conf/settings.xml:28-222,3916-3984` and are
    // decoded by `third_party/refloat/src/conf/buffer.c:208-210`.
    assert_eq!(config.filter().mahony_kp(), MahonyPitchGain::new(2.0));
    assert_eq!(config.filter().mahony_kp_roll(), MahonyRollGain::new(1.4));
    assert_eq!(
        config.motor_control().brake_current().current().as_amps(),
        6.0
    );
    assert_eq!(startup.pitch_tolerance().as_degrees(), 4.0);
    assert_eq!(startup.roll_tolerance().as_degrees(), 45.0);
    assert_eq!(startup.startup_speed().as_degrees_per_second(), 30.0);
    assert_eq!(config.low_voltage_pushback_angle().as_degrees(), 10.0);
    assert_eq!(config.low_voltage_threshold().as_volts(), 3.0);
    assert_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.005);
    assert_eq!(balance.kp_brake().value(), 1.0);
    assert_eq!(balance.kp2_brake().value(), 1.0);
    assert_eq!(balance.ki_limit().current().as_amps(), 30.0);
    assert_eq!(balance.booster_angle().as_degrees(), 8.0);
    assert_eq!(balance.booster_ramp().as_degrees(), 4.0);
    assert_eq!(balance.booster_current().current().as_amps(), 0.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 8.0);
    assert_eq!(balance.brake_booster_ramp().as_degrees(), 4.0);
    assert_eq!(balance.brake_booster_current().current().as_amps(), 0.0);
    assert_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        0.0
    );
    assert_eq!(config.remote_throttle().grace_period().as_seconds(), 10.0);
}

#[test]
fn semantic_config_writes_round_trip_through_generated_storage() {
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
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
    assert_eq!(config.startup().pitch_tolerance().as_degrees(), 3.5);
    assert_eq!(config.startup().roll_tolerance().as_degrees(), 42.0);
    assert_eq!(
        config.startup().startup_speed().as_degrees_per_second(),
        25.0
    );
    assert_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        12.0
    );
    assert_eq!(config.remote_throttle().grace_period().as_seconds(), 1.5);
    assert_eq!(balance.kp().as_amps_per_degree(), 15.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.75);
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.004);
    assert_eq!(balance.kp_brake().value(), 0.8);
    assert_eq!(balance.booster_angle().as_degrees(), 7.0);
    assert_eq!(balance.booster_ramp().as_degrees(), 2.5);
    assert_eq!(balance.booster_current().current().as_amps(), 4.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 6.0);
    assert_eq!(balance.brake_booster_ramp().as_degrees(), 2.0);
    assert_eq!(balance.brake_booster_current().current().as_amps(), 3.0);
}

#[test]
fn parking_brake_mode_field_decodes_known_and_unknown_values() {
    let mut bytes = default_refloat_config_bytes();
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::RefloatParkingBrakeMode::Idle
    );

    bytes[101] = 0xff;
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::RefloatParkingBrakeMode::Unknown(0xff)
    );
}

#[test]
fn handtest_safety_overrides_encode_named_semantic_values() {
    let mut config = RefloatConfigImage::defaults();
    assert!(config.editor().apply_handtest_safety_overrides());

    let balance = config.balance();
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.0);
    assert_eq!(balance.kp_brake().value(), 1.0);
    assert_eq!(balance.kp2_brake().value(), 1.0);
    assert_eq!(balance.booster_angle().as_degrees(), 100.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 100.0);
    assert_eq!(config.faults().pitch_delay().as_seconds(), 0.05);
    assert_eq!(config.faults().roll_delay().as_seconds(), 0.05);

    // These currently unwired tune categories have no domain readers yet;
    // verify their generated float16 storage at the serializer boundary.
    for offset in [67, 71, 126, 128, 130, 145, 147] {
        assert_eq!(&config.as_bytes()[offset..offset + 2], &[0, 0]);
    }
}

#[test]
fn refloat_config_image_rejects_short_payload_like_confparser() {
    let bytes = default_refloat_config_bytes();

    // C map: `third_party/refloat/src/conf/confparser.h:11-12` fixes the serialized config length,
    // so shorter payloads must fail before any typed parsing or state mutation.
    assert!(RefloatConfigImage::from_serialized(&bytes[..275]).is_none());
}

#[test]
fn refloat_config_image_rejects_bad_signature_like_confparser() {
    let mut bytes = default_refloat_config_bytes();
    bytes[0] ^= 0xff;

    // C map: `third_party/refloat/src/conf/confparser.c:187-190` rejects the signature before field reads.
    assert!(RefloatConfigImage::from_serialized(&bytes).is_none());
}

#[test]
fn store_serialized_config_rejects_short_payload_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let bytes = default_refloat_config_bytes();

    assert!(!state.store_serialized_config(&bytes[..275]));

    // C map: upstream rejects truncated custom-config writes before storing them at
    // `third_party/refloat/src/main.c:2360-2368`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_bad_signature_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let mut bytes = default_refloat_config_bytes();
    bytes[0] ^= 0xff;

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects bad config signatures before deserializing any fields at
    // `third_party/refloat/src/conf/confparser.c:187-190`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_special_modes_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::HandTest,
    ));
    let bytes = default_refloat_config_bytes();

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects config writes outside `MODE_NORMAL` at
    // `third_party/refloat/src/main.c:2362-2365`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_clears_default_and_keeps_enabled_while_running_like_refloat() {
    let _firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Normal,
    ));
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
        assert!(config.set_disabled(true));
        assert!(config.set_meta_is_default(true));
    });

    assert!(state.store_serialized_config(&bytes));

    // C map: running writes clear `disabled` at `third_party/refloat/src/main.c:2369-2372`
    // and always clear `meta.is_default` at `third_party/refloat/src/main.c:2375-2377`.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().disabled());
    assert!(!current.metadata().is_default());
}

#[test]
fn successful_config_write_reconfigures_and_acknowledges_like_refloat() {
    let _firmware = FirmwareTest::new();
    for (disabled, expected_run_state, expected_changes, expected_last) in [
        (
            false,
            RefloatRunState::Ready,
            3,
            (240, RefloatBeeperLevel::Low),
        ),
        (
            true,
            RefloatRunState::Disabled,
            7,
            (560, RefloatBeeperLevel::Low),
        ),
    ] {
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));
        let mut bytes = default_refloat_config_bytes();
        bytes.edit_refloat_config(|config| {
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
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let original = state.serialized_config;
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
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
        RefloatRunState::Ready,
    );
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn store_serialized_config_persists_for_restart_like_refloat_set_cfg() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
    });

    assert!(state.store_serialized_config(&bytes));
    let mut persisted = [0; REFLOAT_EEPROM_LEN];
    assert!(firmware.eeprom().read_bytes(&mut persisted));
    assert_eq!(
        &persisted[..state.serialized_config.as_bytes().len()],
        state.serialized_config.as_bytes(),
    );
    assert!(
        persisted[state.serialized_config.as_bytes().len()..]
            .iter()
            .all(|byte| *byte == 0)
    );

    let restarted =
        RefloatPackageState::from_persisted_config(RefloatAllDataPayloads::source_startup());
    assert_eq!(
        restarted
            .serialized_config
            .balance()
            .kp()
            .as_amps_per_degree(),
        15.0,
    );
}
