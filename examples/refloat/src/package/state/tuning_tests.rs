use super::RefloatPackageState;
use crate::beeper::RefloatBeeperLevel;
use crate::domain::{REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataPayloads, RefloatAppDataCommand};
use std::vec::Vec;
use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent, TimestampTicks};
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn booster_command_decodes_nibbles_and_acknowledges_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::Booster.id(),
            0xA3,
            0x04,
            0x21,
            0xF2,
        ],
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
            (80, RefloatBeeperLevel::Low),
            (160, RefloatBeeperLevel::High),
            (240, RefloatBeeperLevel::Low),
        ],
    );
}

#[test]
fn booster_command_rejects_wrong_payload_length_without_alerting() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let before = state.serialized_config;
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(!state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::Booster.id(),
            0xA3,
            0x04,
            0x21,
        ],
    ));
    assert_eq!(state.serialized_config, before);
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn runtime_tune_applies_all_three_refloat_blocks_and_long_acknowledgement() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RuntimeTune.id(),
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
    ));

    let bytes = state.serialized_config.as_bytes();
    for (offset, expected) in [
        (4, [0x00, 0xB4]),
        (6, [0x00, 0x64]),
        (8, [0x01, 0xF4]),
        (10, [0x65, 0x90]),
        (14, [0x00, 0x1E]),
        (16, [0x00, 0x50]),
        (104, [0x00, 0xD2]),
        (118, [0x07, 0x6C]),
        (120, [0x01, 0x5E]),
        (122, [0x00, 0x96]),
        (124, [0x03, 0x84]),
        (126, [0x00, 0x96]),
        (128, [0x01, 0x2C]),
        (130, [0x01, 0xF4]),
        (132, [0x01, 0x2C]),
        (136, [0x07, 0xD0]),
        (145, [0x00, 0x00]),
        (147, [0x02, 0xBC]),
        (149, [0x01, 0x2C]),
        (151, [0x01, 0x90]),
        (153, [0xF2, 0x54]),
        (155, [0x03, 0x84]),
        (157, [0x01, 0x90]),
        (159, [0x01, 0xF4]),
        (161, [0x05, 0xDC]),
        (163, [0x0B, 0xB8]),
        (167, [0x03, 0x20]),
        (169, [0x03, 0x84]),
        (171, [0x00, 0x64]),
        (173, [0x07, 0xD0]),
    ] {
        assert_eq!(&bytes[offset..offset + 2], &expected);
    }

    let changes: Vec<_> = (1..=900)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (300, RefloatBeeperLevel::Low),
            (600, RefloatBeeperLevel::High),
            (900, RefloatBeeperLevel::Low),
        ],
    );
}

#[test]
fn runtime_tune_preserves_refloat_progressive_payload_lengths() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let original = state.serialized_config;
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RuntimeTune.id(),
            0x11,
        ],
    ));
    assert_eq!(state.serialized_config, original);

    let mut block_one = [0_u8; 14];
    block_one[0] = REFLOAT_APP_DATA_PACKAGE_ID.get();
    block_one[1] = RefloatAppDataCommand::RuntimeTune.id();
    block_one[2] = 0x22;
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &block_one,
    ));
    assert_ne!(
        &state.serialized_config.as_bytes()[4..6],
        &original.as_bytes()[4..6]
    );

    let mut block_two = [0_u8; 18];
    block_two[0] = REFLOAT_APP_DATA_PACKAGE_ID.get();
    block_two[1] = RefloatAppDataCommand::RuntimeTune.id();
    block_two[14] = 0x22;
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
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
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::TuneTilt.id(),
            1,
            15,
            85,
            25,
            30,
        ],
    ));

    let bytes = state.serialized_config.as_bytes();
    assert_eq!(&bytes[44..51], &[0x00, 0xFA, 0x01, 0x2C, 0x03, 0x52, 1]);
    assert_eq!(&bytes[64..66], &[0x00, 0x96]);
    let changes: Vec<_> = (1..=560)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (80, RefloatBeeperLevel::Low),
            (160, RefloatBeeperLevel::High),
            (240, RefloatBeeperLevel::Low),
            (320, RefloatBeeperLevel::High),
            (400, RefloatBeeperLevel::Low),
            (480, RefloatBeeperLevel::High),
            (560, RefloatBeeperLevel::Low),
        ],
    );
}

#[test]
fn tune_other_applies_startup_nose_and_input_settings_without_alerting() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::TuneOther.id(),
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
    ));

    let bytes = state.serialized_config.as_bytes();
    assert_eq!(&bytes[39..44], &[1, 0, 1, 1, 1]);
    assert_eq!(
        &bytes[67..84],
        &[
            0x13, 0x88, 0x07, 0xD0, 0x00, 0xFA, 0x01, 0x5E, 0x0F, 0xA0, 0x01, 0x2C, 2, 0x04, 0xB0,
            0x03, 0x20
        ]
    );
    assert_eq!(
        &bytes[91..101],
        &[0x00, 0xC8, 0x05, 0xDC, 0x09, 0xC4, 7, 1, 1, 1]
    );
    assert_eq!(&bytes[102..104], &[0x04, 0xE2]);
    assert_eq!(bytes[242], 1);
    assert_eq!(state.tick_beeper(), None);
}

#[test]
fn tune_other_preserves_refloat_payload_and_value_gates() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    let original_nose = state.serialized_config.as_bytes()[67..84].to_vec();
    let mut now = || TimestampTicks::from_ticks(0);
    let mut send = |_bytes: &[u8]| true;

    assert!(!state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::TuneOther.id(),
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
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::TuneOther.id(),
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
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::TuneOther.id(),
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
