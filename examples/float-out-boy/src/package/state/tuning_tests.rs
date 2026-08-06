use super::{FloatOutBoyPackageState, config_storage::FLOAT_OUT_BOY_EEPROM_LEN};
use crate::beeper::FloatOutBoyBeeperLevel;
use crate::config::FloatOutBoyParkingBrakeMode;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
};
use std::vec::Vec;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, MahonyRollGain, MotorCurrent,
    TimestampTicks,
};
use vescpkg_rs::test_support::FirmwareTest;

const TUNE_DEFAULTS_PACKET: &[u8] = &[
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
    FloatOutBoyAppDataCommand::TuneDefaults.id(),
];
const RUNTIME_TUNE_PACKET: &[u8] = &[
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
];
const TILT_TUNE_PACKET: &[u8] = &[
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
    FloatOutBoyAppDataCommand::TuneTilt.id(),
    1,
    15,
    85,
    25,
    30,
];
const OTHER_TUNE_PACKET: &[u8] = &[
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
];
const BOOSTER_PACKET: &[u8] = &[
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
    FloatOutBoyAppDataCommand::Booster.id(),
    0xA3,
    0x04,
    0x21,
    0xF2,
];

#[test]
fn runtime_only_tunes_leave_persisted_config_unchanged_across_restart() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(AngleCurrentGain::new(15.0))
    );
    let candidate = state.serialized_config;
    assert!(state.store_serialized_config(candidate.as_bytes()));
    let persisted_config = state.serialized_config;
    let mut persisted_image = [0; FLOAT_OUT_BOY_EEPROM_LEN];
    assert!(
        firmware
            .with_effects(|effects| { firmware.eeprom().read_bytes(effects, &mut persisted_image) })
            .is_ok()
    );
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;
    let packets = [
        TUNE_DEFAULTS_PACKET,
        RUNTIME_TUNE_PACKET,
        TILT_TUNE_PACKET,
        OTHER_TUNE_PACKET,
        BOOSTER_PACKET,
    ];

    for packet in &packets {
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut now,
            &mut reply,
            packet,
        ));
        let mut current_image = [0; FLOAT_OUT_BOY_EEPROM_LEN];
        assert!(
            firmware
                .with_effects(|effects| {
                    firmware.eeprom().read_bytes(effects, &mut current_image)
                })
                .is_ok()
        );
        assert_eq!(current_image, persisted_image);
    }

    assert_ne!(state.serialized_config, persisted_config);
    let restarted = FloatOutBoyPackageState::from_persisted_config(
        FloatOutBoyAllDataPayloads::source_startup(),
    );
    assert_eq!(restarted.serialized_config, persisted_config);
}

#[test]
fn runtime_tune_refreshes_idle_epoch_like_refloat_reconfigure() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.idle_ticks.restart(TimestampTicks::from_ticks(7));
    let mut now = || TimestampTicks::from_ticks(42);

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut |_| true,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RuntimeTune.id(),
        ],
    ));

    assert_eq!(state.idle_ticks.started(), TimestampTicks::from_ticks(42));
}

#[test]
fn other_reconfigure_commands_refresh_idle_epoch_like_refloat() {
    let firmware = FirmwareTest::new();
    let packets: &[&[u8]] = &[
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneDefaults.id(),
        ],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneOther.id(),
            0,
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
        ],
    ];

    for packet in packets {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        state.idle_ticks.restart(TimestampTicks::from_ticks(7));
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(42),
            &mut |_| true,
            packet,
        ));
        assert_eq!(state.idle_ticks.started(), TimestampTicks::from_ticks(42));
    }
}

#[test]
fn runtime_tune_reconfigures_once_after_applying_all_blocks() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = RUNTIME_TUNE_PACKET.to_vec();
    packet.extend_from_slice(&[0x54, 0x63]);
    FloatOutBoyPackageState::reset_config_reconfigure_count_for_test();
    state.internal_led_refresh_pending = false;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(42),
        &mut |_| true,
        &packet,
    ));
    assert_eq!(
        FloatOutBoyPackageState::config_reconfigure_count_for_test(),
        1
    );
    assert!(state.internal_led_refresh_pending);
}

#[test]
fn non_reconfigure_tune_commands_preserve_idle_epoch_like_refloat() {
    let firmware = FirmwareTest::new();
    let packets: &[&[u8]] = &[
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Booster.id(),
            0,
            0,
            0,
            0,
        ],
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneTilt.id(),
            0,
            0,
            0,
            0,
            0,
        ],
    ];

    for packet in packets {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        state.idle_ticks.restart(TimestampTicks::from_ticks(7));
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(42),
            &mut |_| true,
            packet,
        ));
        assert_eq!(state.idle_ticks.started(), TimestampTicks::from_ticks(7));
    }
}

#[test]
fn booster_command_decodes_nibbles_and_acknowledges_like_float_out_boy() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        BOOSTER_PACKET,
    ));

    let balance = state.serialized_config.balance();
    assert_eq!(balance.booster_angle(), AngleDegrees::from_degrees(8.0));
    assert_eq!(balance.booster_ramp(), AngleDegrees::from_degrees(12.0));
    assert_eq!(
        balance.booster_current(),
        MotorCurrent::new(Current::from_amps(16.0)),
    );
    assert_eq!(
        balance.brake_booster_angle(),
        AngleDegrees::from_degrees(6.0),
    );
    assert_eq!(
        balance.brake_booster_ramp(),
        AngleDegrees::from_degrees(4.0),
    );
    assert_eq!(
        balance.brake_booster_current(),
        MotorCurrent::new(Current::from_amps(12.0)),
    );

    let changes: Vec<_> = (1..=240)
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
fn booster_command_rejects_wrong_payload_length_without_alerting() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let before = state.serialized_config;
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(!state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Booster.id(),
            0xA3,
            0x04,
            0x21,
        ],
    ));
    assert_eq!(state.serialized_config, before);
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn runtime_tune_applies_all_three_float_out_boy_blocks_and_long_acknowledgement() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let balance_filter_before_tune = state.balance_filter;
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        RUNTIME_TUNE_PACKET,
    ));

    let mut expected_balance_filter = balance_filter_before_tune;
    let filter = state.serialized_config.filter();
    expected_balance_filter.configure(filter.mahony_kp(), filter.mahony_kp_roll());
    assert_eq!(state.balance_filter, expected_balance_filter);

    let bytes = state.serialized_config.as_bytes();
    for (offset, expected) in [
        (4, [0x00, 0xB4]),
        (6, [0x00, 0x64]),
        (8, [0x01, 0xF4]),
        (10, [0x65, 0x90]),
        (14, [0x00, 0x1E]),
        (16, [0x00, 0x50]),
        (121, [0x00, 0xD2]),
        (135, [0x07, 0x6C]),
        (137, [0x01, 0x5E]),
        (34, [0x00, 0x96]),
        (36, [0x03, 0x84]),
        (139, [0x00, 0x96]),
        (141, [0x01, 0x2C]),
        (143, [0x01, 0xF4]),
        (145, [0x01, 0x2C]),
        (149, [0x07, 0xD0]),
        (156, [0x00, 0x00]),
        (158, [0x02, 0xBC]),
        (160, [0x01, 0x2C]),
        (162, [0x01, 0x90]),
        (164, [0xF2, 0x54]),
        (166, [0x03, 0x84]),
        (24, [0x01, 0x90]),
        (26, [0x01, 0xF4]),
        (168, [0x0B, 0xB8]),
        (172, [0x03, 0x20]),
        (174, [0x03, 0x84]),
        (176, [0x00, 0x64]),
        (178, [0x07, 0xD0]),
    ] {
        assert_eq!(&bytes[offset..offset + 2], &expected);
    }

    let changes: Vec<_> = (1..=900)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (26, FloatOutBoyBeeperLevel::Low),
            (52, FloatOutBoyBeeperLevel::High),
            (78, FloatOutBoyBeeperLevel::Low),
        ],
    );
}

#[test]
fn extended_runtime_tune_applies_cutoff_orientation_and_speed_fields() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = RUNTIME_TUNE_PACKET.to_vec();
    packet.extend_from_slice(&[0x54, 0x63]);

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_| true,
        &packet,
    ));

    let config = state.serialized_config;
    assert_eq!(config.filter().mahony_kp_roll(), MahonyRollGain::new(1.4));
    assert_eq!(
        config.balance().turn_tilt_start_angle(),
        AngleDegrees::from_degrees(5.0)
    );
    assert_eq!(
        config.balance().atr_filter_on_speed_limit(),
        AngularVelocity::from_degrees_per_second(6.0)
    );
    assert_eq!(
        config.balance().atr_filter_off_speed_limit(),
        AngularVelocity::from_degrees_per_second(12.0)
    );
    assert_eq!(
        config.balance().torque_tilt_filter_on_speed_limit(),
        AngularVelocity::from_degrees_per_second(6.0)
    );
    assert_eq!(
        config.balance().torque_tilt_filter_off_speed_limit(),
        AngularVelocity::from_degrees_per_second(9.0)
    );
}

#[test]
fn runtime_tune_zero_speed_fields_preserve_existing_atr_filter_limits() {
    let firmware = FirmwareTest::new();

    for optional_extended_speeds in [None, Some(0x03), Some(0x30)] {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        assert!(
            state
                .serialized_config
                .editor()
                .set_atr_on_speed_limit(AngularVelocity::from_degrees_per_second(8.0))
        );
        assert!(
            state
                .serialized_config
                .editor()
                .set_atr_off_speed_limit(AngularVelocity::from_degrees_per_second(10.0))
        );

        let mut packet = RUNTIME_TUNE_PACKET[..14].to_vec();
        packet[9] &= 0x0f;
        if let Some(speeds) = optional_extended_speeds {
            packet.extend_from_slice(&[0, 0, 0, 0, 0, 0, speeds]);
        }

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |_| true,
            &packet,
        ));

        let atr = state.serialized_config.balance();
        assert_eq!(
            atr.atr_filter_on_speed_limit(),
            AngularVelocity::from_degrees_per_second(8.0),
            "extended speeds {optional_extended_speeds:?}",
        );
        assert_eq!(
            atr.atr_filter_off_speed_limit(),
            AngularVelocity::from_degrees_per_second(10.0),
            "extended speeds {optional_extended_speeds:?}",
        );
    }
}

#[test]
fn runtime_tune_preserves_float_out_boy_progressive_payload_lengths() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let original = state.serialized_config;
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RuntimeTune.id(),
            0x11,
        ],
    ));
    assert_eq!(state.serialized_config, original);

    let mut block_one = [0_u8; 14];
    block_one[0] = FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID;
    block_one[1] = FloatOutBoyAppDataCommand::RuntimeTune.id();
    block_one[2] = 0x22;
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &block_one,
    ));
    assert_ne!(
        &state.serialized_config.as_bytes()[4..6],
        &original.as_bytes()[4..6]
    );

    let mut block_two = [0_u8; 18];
    block_two[0] = FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID;
    block_two[1] = FloatOutBoyAppDataCommand::RuntimeTune.id();
    block_two[14] = 0x22;
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &block_two,
    ));
    assert_ne!(
        &state.serialized_config.as_bytes()[149..151],
        &original.as_bytes()[149..151]
    );
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn tilt_tune_applies_duty_settings_and_three_short_beeps() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        TILT_TUNE_PACKET,
    ));

    let bytes = state.serialized_config.as_bytes();
    assert_eq!(&bytes[64..71], &[0x00, 0xFA, 0x01, 0x2C, 0x03, 0x52, 1]);
    assert_eq!(&bytes[84..86], &[0x00, 0x96]);
    let changes: Vec<_> = (1..=560)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (6, FloatOutBoyBeeperLevel::Low),
            (12, FloatOutBoyBeeperLevel::High),
            (18, FloatOutBoyBeeperLevel::Low),
            (24, FloatOutBoyBeeperLevel::High),
            (30, FloatOutBoyBeeperLevel::Low),
            (36, FloatOutBoyBeeperLevel::High),
            (42, FloatOutBoyBeeperLevel::Low),
        ],
    );
}

#[test]
fn tilt_tune_optional_speed_pushback_threshold_is_cutoff_compatible() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    assert!(
        state
            .serialized_config
            .editor()
            .set_speed_pushback_threshold(vescpkg_rs::WireByte::new(12))
    );

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_| true,
        TILT_TUNE_PACKET,
    ));
    assert_f32_eq!(
        state
            .serialized_config
            .speed_pushback_threshold()
            .as_kilometers_per_hour(),
        12.0
    );

    let mut extended = TILT_TUNE_PACKET.to_vec();
    extended.push(34);
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_| true,
        &extended,
    ));
    assert_f32_eq!(
        state
            .serialized_config
            .speed_pushback_threshold()
            .as_kilometers_per_hour(),
        34.0
    );
}

#[test]
fn tune_other_applies_startup_nose_and_input_settings_without_alerting() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        OTHER_TUNE_PACKET,
    ));

    let bytes = state.serialized_config.as_bytes();
    assert_eq!(&bytes[59..64], &[1, 0, 1, 1, 1]);
    assert_eq!(
        &bytes[87..102],
        &[
            0x13, 0x88, 0x07, 0xD0, 0x00, 0xFA, 0x01, 0x5E, 0x0F, 0xA0, 0x01, 0x2C, 2, 0x04, 0xB0
        ]
    );
    assert_eq!(
        &bytes[108..118],
        &[0x00, 0xC8, 0x05, 0xDC, 0x09, 0xC4, 7, 1, 1, 1]
    );
    assert_eq!(&bytes[119..121], &[0x04, 0xE2]);
    assert_eq!(bytes[248], 1);
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn tune_other_decodes_cutoff_negative_variable_tilt_boundaries() {
    let firmware = FirmwareTest::new();

    for (encoded, expected_degrees) in [
        (35, 3.5),
        (100, 10.0),
        (101, -0.1),
        (110, -1.0),
        (u8::MAX, -10.0),
    ] {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut packet = OTHER_TUNE_PACKET.to_vec();
        packet[12] = encoded;

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |_| true,
            &packet,
        ));
        assert_f32_eq!(
            state.serialized_config.tiltback_variable_max().as_degrees(),
            expected_degrees
        );
    }
}

#[test]
fn tune_other_applies_cutoff_secondary_flags() {
    let firmware = FirmwareTest::new();

    for (flags, moving_faults_disabled, foot_beep_enabled, parking_brake_mode) in [
        (0b0000, false, false, FloatOutBoyParkingBrakeMode::ALWAYS),
        (0b0001, true, false, FloatOutBoyParkingBrakeMode::ALWAYS),
        (0b0010, false, true, FloatOutBoyParkingBrakeMode::ALWAYS),
        (0b0100, false, false, FloatOutBoyParkingBrakeMode::IDLE),
        (0b1000, false, false, FloatOutBoyParkingBrakeMode::NEVER),
        (0b1111, true, true, FloatOutBoyParkingBrakeMode::from(3)),
    ] {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut packet = OTHER_TUNE_PACKET.to_vec();
        packet.push(flags);

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |_| true,
            &packet,
        ));
        assert_eq!(
            state.serialized_config.faults().moving_faults_disabled(),
            moving_faults_disabled,
            "flags {flags:#06b}",
        );
        assert_eq!(
            state.serialized_config.foot_beep_enabled(),
            foot_beep_enabled,
            "flags {flags:#06b}",
        );
        assert_eq!(
            state.serialized_config.motor_control().parking_brake_mode(),
            parking_brake_mode,
            "flags {flags:#06b}",
        );
    }
}

#[test]
fn tune_other_preserves_float_out_boy_payload_and_value_gates() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let original_nose = state.serialized_config.as_bytes()[67..84].to_vec();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut reply = |_bytes: &[u8]| true;

    assert!(!state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneOther.id(),
            0,
            1,
            2,
            3,
            4,
            5,
            121,
            6,
            7,
            8,
            9,
        ],
    ));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneOther.id(),
            0,
            1,
            2,
            3,
            4,
            5,
            121,
            6,
            7,
            8,
            9,
            10,
        ],
    ));
    assert_eq!(&state.serialized_config.as_bytes()[67..84], original_nose);

    let original_input = state.serialized_config.as_bytes()[79..84].to_vec();
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut reply,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::TuneOther.id(),
            0,
            1,
            2,
            3,
            4,
            5,
            121,
            6,
            7,
            8,
            9,
            10,
            3,
            11,
        ],
    ));
    assert_eq!(&state.serialized_config.as_bytes()[79..84], original_input);
    assert_eq!(state.tick_beeper(), None);
}
