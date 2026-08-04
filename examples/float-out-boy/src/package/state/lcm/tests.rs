use super::*;
use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyRunState};
use std::vec::Vec;
use vescpkg_rs::test_support::FirmwareTest;

fn external_state() -> FloatOutBoyPackageState {
    let mut state = FloatOutBoyPackageState::default();
    state.set_lcm_hardware_mode_for_test(2);
    state
}

fn dispatch(
    state: &mut FloatOutBoyPackageState,
    firmware: &FirmwareTest,
    packet: &[u8],
) -> Vec<u8> {
    let mut response = Vec::new();
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || vescpkg_rs::prelude::TimestampTicks::from_ticks(0),
        &mut |bytes| {
            response.extend_from_slice(bytes);
            true
        },
        packet,
    ));
    response
}

fn dispatch_command(
    state: &mut FloatOutBoyPackageState,
    firmware: &FirmwareTest,
    command: FloatOutBoyAppDataCommand,
) -> Vec<u8> {
    dispatch(
        state,
        firmware,
        &[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(), command.id()],
    )
}

fn dispatch_payload(
    state: &mut FloatOutBoyPackageState,
    firmware: &FirmwareTest,
    command: FloatOutBoyAppDataCommand,
    payload: &[u8],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(payload.len() + 2);
    packet.extend_from_slice(&[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(), command.id()]);
    packet.extend_from_slice(payload);
    dispatch(state, firmware, &packet)
}

fn external_configured_state() -> FloatOutBoyPackageState {
    let mut state = FloatOutBoyPackageState::default();
    let mut config = state.serialized_config.as_bytes().to_vec();
    config[227] = crate::lcm::FloatOutBoyLedMode::External.id();
    assert!(state.store_serialized_config(&config));
    state
}

#[test]
fn lcm_dispatch_recognizes_exactly_its_six_refloat_commands() {
    let firmware = FirmwareTest::new();

    for command_id in 0..=u8::MAX {
        let mut state = external_state();
        let mut replies = 0;
        let handled = state.handle_lcm_packet(
            firmware.telemetry(),
            &mut |_| {
                replies += 1;
                true
            },
            &[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(), command_id],
        );
        let command = FloatOutBoyAppDataCommand::try_from_id(command_id);
        let expected = matches!(
            command,
            Ok(FloatOutBoyAppDataCommand::LightsControl
                | FloatOutBoyAppDataCommand::LcmPoll
                | FloatOutBoyAppDataCommand::LcmLightInfo
                | FloatOutBoyAppDataCommand::LcmLightControl
                | FloatOutBoyAppDataCommand::LcmDeviceInfo
                | FloatOutBoyAppDataCommand::LcmGetBattery)
        );
        let expected_replies =
            usize::from(expected && command != Ok(FloatOutBoyAppDataCommand::LcmLightControl));

        assert_eq!(handled, expected, "command {command_id}");
        assert_eq!(replies, expected_replies, "command {command_id}");
    }

    for packet in [&[][..], &[101][..], &[100, 24][..]] {
        let mut state = external_state();
        assert!(!state.handle_lcm_packet(firmware.telemetry(), &mut |_| true, packet));
    }
}

#[test]
fn every_lcm_response_starts_with_the_refloat_package_and_command_ids() {
    let state = LcmState::new(2, false);
    assert_eq!(
        &state.light_info_response().as_bytes()[..2],
        [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::LcmLightInfo.id(),
        ]
    );
    assert_eq!(
        &state.device_info_response().as_bytes()[..2],
        [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::LcmDeviceInfo.id(),
        ]
    );
}

#[test]
fn light_info_and_lights_control_match_refloat_wire_contract() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();

    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmLightInfo
        ),
        [101, 25, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );

    assert_eq!(
        dispatch_payload(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl,
            &[0, 0, 0, 3, 3],
        ),
        [101, 20, 3]
    );
}

#[test]
fn startup_lights_control_reflects_serialized_led_flags() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();

    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl
        ),
        [101, 20, 3]
    );
}

#[test]
fn external_lcm_configuration_uses_serialized_led_brightness_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = external_configured_state();

    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmLightInfo
        ),
        [101, 25, 3, 50, 50, 20, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn lights_control_is_temporary_across_later_config_writes() {
    let firmware = FirmwareTest::new();
    let mut state = external_configured_state();
    let mut config = state.serialized_config.as_bytes().to_vec();

    assert_eq!(
        dispatch_payload(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl,
            &[0, 0, 0, 3, 0],
        ),
        [101, 20, 0]
    );
    assert!(state.serialized_config.leds_enabled());
    assert!(state.serialized_config.headlights_enabled());

    config[120] = 40;
    assert!(state.store_serialized_config(&config));
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl
        ),
        [101, 20, 0]
    );
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmLightInfo
        ),
        [101, 25, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn lights_control_partial_mask_tracks_unoverridden_config_field() {
    let firmware = FirmwareTest::new();
    let mut state = external_configured_state();
    let mut config = state.serialized_config.as_bytes().to_vec();

    assert_eq!(
        dispatch_payload(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl,
            &[0, 0, 0, 1, 0],
        ),
        [101, 20, 2]
    );

    config[176] = 0;
    assert!(state.store_serialized_config(&config));
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LightsControl
        ),
        [101, 20, 0]
    );
}

#[test]
fn lights_control_preserves_live_internal_renderer_state_like_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(
        crate::package::test_support::sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ),
    );
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
        crate::domain::FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            crate::domain::FloatOutBoyFootpadSample::new(
                vescpkg_rs::prelude::Voltage::ZERO,
                vescpkg_rs::prelude::Voltage::ZERO,
                crate::domain::FloatOutBoyFootpadState::None,
            ),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );
    let mut config = state.serialized_config.as_bytes().to_vec();
    config[227] = crate::lcm::FloatOutBoyLedMode::Both.id();
    assert!(state.store_serialized_config(&config));
    let _ = crate::package::threads::tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        vescpkg_rs::prelude::OdometerMeters::from_meters(0),
        vescpkg_rs::prelude::TimestampTicks::from_ticks(0),
        1.0,
        |_| {},
        || true,
    );
    let before = state.internal_led_renderer_for_test().unwrap();

    dispatch(
        &mut state,
        &firmware,
        &[
            101,
            FloatOutBoyAppDataCommand::LightsControl.id(),
            0,
            0,
            0,
            2,
            0,
        ],
    );

    assert_eq!(state.internal_led_renderer_for_test(), Some(before));
}

#[test]
fn light_control_payload_is_forwarded_once_by_poll_and_device_info_echoes_name() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || vescpkg_rs::prelude::TimestampTicks::from_ticks(0),
        &mut |_| true,
        &[101, 26, 10, 20, 30, 0xaa, 0x55],
    ));

    let first = dispatch_payload(
        &mut state,
        &firmware,
        FloatOutBoyAppDataCommand::LcmPoll,
        b"LCM\0",
    );
    assert_eq!(&first[..2], &[101, 24]);
    assert_eq!(&first[11..], &[10, 20, 30, 0xaa, 0x55]);

    let second = dispatch_command(&mut state, &firmware, FloatOutBoyAppDataCommand::LcmPoll);
    assert_eq!(second.len(), 14);
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmDeviceInfo
        ),
        [101, 27, b'L', b'C', b'M', 0]
    );
}

#[test]
fn light_control_relay_is_safely_capped_at_refloats_64_byte_storage() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();
    let mut command = vec![101, 26, 10, 20, 30];
    command.extend(0_u8..70);

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || vescpkg_rs::prelude::TimestampTicks::from_ticks(0),
        &mut |_| true,
        &command,
    ));

    let response = dispatch_command(&mut state, &firmware, FloatOutBoyAppDataCommand::LcmPoll);
    assert_eq!(response.len(), 14 + MAX_LCM_PAYLOAD_LENGTH);
    assert_eq!(&response[14..], &(0_u8..64).collect::<Vec<_>>());
}

#[test]
fn refloat_reserved_lcm_debug_command_remains_undispatched() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();
    let mut sent = false;

    assert!(!state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || vescpkg_rs::prelude::TimestampTicks::from_ticks(0),
        &mut |_| {
            sent = true;
            true
        },
        &[101, FloatOutBoyAppDataCommand::LcmDebug.id()],
    ));
    assert!(!sent);
}

#[test]
fn shorter_lcm_name_replaces_the_previous_name_without_a_stale_suffix() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();

    dispatch_payload(
        &mut state,
        &firmware,
        FloatOutBoyAppDataCommand::LcmPoll,
        b"LONG\0",
    );
    dispatch_payload(
        &mut state,
        &firmware,
        FloatOutBoyAppDataCommand::LcmPoll,
        b"N",
    );

    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmDeviceInfo
        ),
        [101, 27, b'N', 0]
    );
}

#[test]
fn lcm_name_stops_at_nul_and_at_refloats_twenty_byte_limit() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();

    dispatch_payload(
        &mut state,
        &firmware,
        FloatOutBoyAppDataCommand::LcmPoll,
        b"A\0B",
    );
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmDeviceInfo
        ),
        [101, 27, b'A', 0]
    );

    let mut poll = vec![101, 24];
    poll.extend(1_u8..=MAX_LCM_NAME_LENGTH as u8 + 1);
    dispatch(&mut state, &firmware, &poll);
    let mut expected = vec![101, 27];
    expected.extend(1_u8..=MAX_LCM_NAME_LENGTH as u8);
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmDeviceInfo
        ),
        expected
    );
}

#[test]
fn battery_response_uses_float32_auto_and_disabled_lcm_stays_minimal() {
    let firmware = FirmwareTest::new();
    let mut state = external_state();
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmGetBattery
        )
        .len(),
        6
    );

    state.set_lcm_hardware_mode_for_test(0);
    assert_eq!(
        dispatch_command(
            &mut state,
            &firmware,
            FloatOutBoyAppDataCommand::LcmLightInfo
        ),
        [101, 25]
    );
    assert_eq!(
        dispatch_command(&mut state, &firmware, FloatOutBoyAppDataCommand::LcmPoll),
        [101, 24]
    );
}
