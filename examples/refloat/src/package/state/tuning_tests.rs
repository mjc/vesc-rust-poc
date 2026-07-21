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
