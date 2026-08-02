//! External LCM protocol state and packet handling.
//!
//! The wire shapes mirror Float Out Boy v1.2.1 `src/lcm.c` and the lights
//! command in `src/main.c`. This module owns only the external protocol seam;
//! internal LED DMA rendering remains a separate runtime slice.

#![expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::indexing_slicing,
    reason = "LCM wire encoding uses fixed-width protocol offsets"
)]

use super::FloatOutBoyPackageState;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
    FloatOutBoyRunState,
};
use crate::wire::{degrees, push_bytes, push_float32_auto, push_u8, push_u16};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::FirmwareFault;

const MAX_LCM_NAME_LENGTH: usize = 20;
const MAX_LCM_PAYLOAD_LENGTH: usize = 64;
const POLL_RESPONSE_CAPACITY: usize = 2 + 3 + 6 + 3 + MAX_LCM_PAYLOAD_LENGTH;

fn nul_terminated_prefix(bytes: &[u8]) -> &[u8] {
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .map_or(bytes.len(), |index| index.saturating_add(1));
    &bytes[..len]
}

fn configured_brightness(config: crate::leds::FloatOutBoyLedsConfig) -> [u8; 3] {
    if !config.is_enabled() {
        return [0; 3];
    }

    let front = config.front().brightness();
    let (active, status) = if config.are_headlights_on() {
        (
            config.headlights().brightness(),
            config.status().brightness_headlights_on(),
        )
    } else {
        (front, config.status().brightness_headlights_off())
    };
    [active, front, status].map(|ratio| (ratio.as_ratio() * 100.0) as u8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LcmState {
    hardware_mode: u8,
    brightness: u8,
    brightness_idle: u8,
    status_brightness: u8,
    lights_off_when_lifted: bool,
    name: [u8; MAX_LCM_NAME_LENGTH],
    payload: [u8; MAX_LCM_PAYLOAD_LENGTH],
    payload_size: usize,
}

impl LcmState {
    // Keep the buffer initialization in its own frame so the loader's direct
    // `package_lib_init` frame stays below the 1,024-byte stack budget.
    #[inline(never)]
    pub(super) fn new(hardware_mode: u8, lights_off_when_lifted: bool) -> Self {
        Self {
            hardware_mode,
            brightness: 0,
            brightness_idle: 0,
            status_brightness: 0,
            lights_off_when_lifted,
            name: [0; MAX_LCM_NAME_LENGTH],
            payload: [0; MAX_LCM_PAYLOAD_LENGTH],
            payload_size: 0,
        }
    }

    pub(super) const fn set_hardware_mode(&mut self, hardware_mode: u8) {
        self.hardware_mode = hardware_mode;
    }

    #[cfg(test)]
    pub(super) const fn hardware_mode(self) -> u8 {
        self.hardware_mode
    }

    pub(super) fn configure(&mut self, config: crate::leds::FloatOutBoyLedsConfig) {
        if !self.enabled() {
            return;
        }

        [
            self.brightness,
            self.brightness_idle,
            self.status_brightness,
        ] = configured_brightness(config);
        self.lights_off_when_lifted = config.turns_lights_off_when_lifted();
    }

    const fn enabled(self) -> bool {
        self.hardware_mode & 0x2 != 0
    }

    fn poll_request(&mut self, payload: &[u8]) {
        if !self.enabled() || payload.is_empty() {
            return;
        }

        self.name.fill(0);
        let payload = nul_terminated_prefix(payload);
        let len = payload.len().min(MAX_LCM_NAME_LENGTH);
        self.name[..len].copy_from_slice(&payload[..len]);
    }

    fn light_control(&mut self, payload: &[u8]) {
        if !self.enabled() || payload.len() < 3 {
            return;
        }

        self.brightness = payload[0];
        self.brightness_idle = payload[1];
        self.status_brightness = payload[2];
        let extra = &payload[3..];
        self.payload_size = extra.len().min(MAX_LCM_PAYLOAD_LENGTH);
        self.payload[..self.payload_size].copy_from_slice(&extra[..self.payload_size]);
    }

    fn poll_response(
        &mut self,
        payloads: crate::domain::FloatOutBoyAllDataPayloads,
        telemetry: &impl MotorTelemetry,
    ) -> LcmPacket<POLL_RESPONSE_CAPACITY> {
        let mut packet = LcmPacket::new(FloatOutBoyAppDataCommand::LcmPoll);

        if !self.enabled() {
            return packet;
        }

        let base = payloads.base();
        let ride_state = base.status().ride_state();
        let mut state = ride_state.float_state_compat() & 0x0f;
        state |= base.footpad().state().id() << 4;
        if matches!(ride_state.mode(), FloatOutBoyMode::HandTest) {
            state |= 0x80;
        }
        packet.push(state);
        packet.push(firmware_fault_code(telemetry.firmware_fault()));

        let duty_or_pitch = if matches!(ride_state.run_state(), FloatOutBoyRunState::Running) {
            (telemetry.duty_cycle().ratio().as_ratio().abs() * 100.0).clamp(0.0, 100.0) as u8
        } else if self.lights_off_when_lifted {
            degrees(base.attitude().pitch().angle()).abs().min(255.0) as u8
        } else {
            0
        };
        packet.push(duty_or_pitch);
        packet.push_scaled_i16(
            telemetry
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute(),
            1.0,
        );
        packet.push_scaled_i16(telemetry.battery_current().current().as_amps(), 1.0);
        packet.push_scaled_i16(telemetry.input_voltage().voltage().as_volts(), 10.0);
        packet.push(self.brightness);
        packet.push(self.brightness_idle);
        packet.push(self.status_brightness);
        packet.extend(&self.payload[..self.payload_size]);
        self.payload_size = 0;
        packet
    }

    fn light_info_response(self) -> LcmPacket<12> {
        let mut packet = LcmPacket::new(FloatOutBoyAppDataCommand::LcmLightInfo);
        if self.enabled() {
            packet.push(3);
            packet.push(self.brightness);
            packet.push(self.brightness_idle);
            packet.push(self.status_brightness);
            // Refloat's Float-specific LED fields are intentionally not sent
            // through this LCM interface.
            for _ in 0..6 {
                packet.push(0);
            }
        }
        packet
    }

    fn device_info_response(self) -> LcmPacket<22> {
        let mut packet = LcmPacket::new(FloatOutBoyAppDataCommand::LcmDeviceInfo);
        if self.enabled() {
            packet.extend(nul_terminated_prefix(&self.name));
        }
        packet
    }

    fn battery_response(self, telemetry: &impl MotorTelemetry) -> LcmPacket<6> {
        let mut packet = LcmPacket::new(FloatOutBoyAppDataCommand::LcmGetBattery);
        if self.enabled() {
            packet.push_float32_auto(telemetry.battery_level().as_fraction());
        }
        packet
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LcmPacket<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LcmPacket<N> {
    const fn new(command: FloatOutBoyAppDataCommand) -> Self {
        let mut bytes = [0; N];
        bytes[0] = FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get();
        bytes[1] = command.id();
        Self { bytes, len: 2 }
    }

    fn push(&mut self, byte: u8) {
        push_u8(&mut self.bytes, &mut self.len, byte);
    }

    fn extend(&mut self, bytes: &[u8]) {
        push_bytes(&mut self.bytes, &mut self.len, bytes);
    }

    fn push_scaled_i16(&mut self, value: f32, scale: f32) {
        let value = (value * scale) as i16 as u16;
        push_u16(&mut self.bytes, &mut self.len, value);
    }

    fn push_float32_auto(&mut self, value: f32) {
        push_float32_auto(&mut self.bytes, &mut self.len, value);
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

fn firmware_fault_code(fault: FirmwareFault) -> u8 {
    match fault {
        FirmwareFault::Active(fault) => fault.wire_code().wire_code(),
        FirmwareFault::None | FirmwareFault::Unknown => 0,
    }
}

impl FloatOutBoyPackageState {
    pub(super) fn handle_lcm_packet(
        &mut self,
        telemetry: &impl MotorTelemetry,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        use FloatOutBoyAppDataCommand as Command;

        let [package_id, command_id, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
            return false;
        }
        let Ok(command) = FloatOutBoyAppDataCommand::try_from_id(*command_id) else {
            return false;
        };

        match command {
            Command::LcmPoll => {
                self.lcm.poll_request(payload);
                reply(
                    self.lcm
                        .poll_response(self.all_data_payloads, telemetry)
                        .bytes(),
                )
            }
            Command::LcmLightInfo => reply(self.lcm.light_info_response().bytes()),
            Command::LcmLightControl => {
                self.lcm.light_control(payload);
                true
            }
            Command::LcmDeviceInfo => reply(self.lcm.device_info_response().bytes()),
            Command::LcmGetBattery => reply(self.lcm.battery_response(telemetry).bytes()),
            Command::LightsControl => {
                if let [_, _, _, mask, value, ..] = payload {
                    if *mask != 0 {
                        self.set_led_runtime_overrides(
                            (mask & 1 != 0).then_some(value & 1 != 0),
                            (mask & 2 != 0).then_some(value & 2 != 0),
                        );
                    }
                }
                let status = self.led_runtime_status();
                reply(&[
                    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                    FloatOutBoyAppDataCommand::LightsControl.id(),
                    u8::from(status.enabled()) | (u8::from(status.headlights_enabled()) << 1),
                ])
            }
            _ => false,
        }
    }

    #[cfg(test)]
    pub(super) fn set_lcm_hardware_mode_for_test(&mut self, mode: u8) {
        self.lcm.set_hardware_mode(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FloatOutBoyAllDataPayloads;
    use std::vec::Vec;
    use vescpkg_rs::test_support::FirmwareTest;

    fn external_state() -> FloatOutBoyPackageState {
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
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
            &state.light_info_response().bytes()[..2],
            [
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                FloatOutBoyAppDataCommand::LcmLightInfo.id(),
            ]
        );
        assert_eq!(
            &state.device_info_response().bytes()[..2],
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
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LcmLightInfo.id()]
            ),
            [101, 25, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[
                    101,
                    FloatOutBoyAppDataCommand::LightsControl.id(),
                    0,
                    0,
                    0,
                    3,
                    3,
                ]
            ),
            [101, 20, 3]
        );
    }

    #[test]
    fn startup_lights_control_reflects_serialized_led_flags() {
        let firmware = FirmwareTest::new();
        let mut state = external_state();

        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LightsControl.id()]
            ),
            [101, 20, 3]
        );
    }

    #[test]
    fn external_lcm_configuration_uses_serialized_led_brightness_like_refloat() {
        let firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = state.serialized_config.as_bytes().to_vec();
        config[227] = crate::lcm::FloatOutBoyLedMode::External.id();
        assert!(state.store_serialized_config(&config));

        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LcmLightInfo.id()]
            ),
            [101, 25, 3, 50, 50, 20, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn lights_control_is_temporary_across_later_config_writes() {
        let firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = state.serialized_config.as_bytes().to_vec();
        config[227] = crate::lcm::FloatOutBoyLedMode::External.id();
        assert!(state.store_serialized_config(&config));

        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[
                    101,
                    FloatOutBoyAppDataCommand::LightsControl.id(),
                    0,
                    0,
                    0,
                    3,
                    0
                ]
            ),
            [101, 20, 0]
        );
        assert!(state.serialized_config.leds_enabled());
        assert!(state.serialized_config.headlights_enabled());

        config[120] = 40;
        assert!(state.store_serialized_config(&config));
        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LightsControl.id()]
            ),
            [101, 20, 0]
        );
        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LcmLightInfo.id()]
            ),
            [101, 25, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn lights_control_partial_mask_tracks_unoverridden_config_field() {
        let firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = state.serialized_config.as_bytes().to_vec();
        config[227] = crate::lcm::FloatOutBoyLedMode::External.id();
        assert!(state.store_serialized_config(&config));

        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[
                    101,
                    FloatOutBoyAppDataCommand::LightsControl.id(),
                    0,
                    0,
                    0,
                    1,
                    0
                ]
            ),
            [101, 20, 2]
        );

        config[176] = 0;
        assert!(state.store_serialized_config(&config));
        assert_eq!(
            dispatch(
                &mut state,
                &firmware,
                &[101, FloatOutBoyAppDataCommand::LightsControl.id()]
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

        let first = dispatch(&mut state, &firmware, &[101, 24, b'L', b'C', b'M', 0]);
        assert_eq!(&first[..2], &[101, 24]);
        assert_eq!(&first[11..], &[10, 20, 30, 0xaa, 0x55]);

        let second = dispatch(&mut state, &firmware, &[101, 24]);
        assert_eq!(second.len(), 14);
        assert_eq!(
            dispatch(&mut state, &firmware, &[101, 27]),
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

        let response = dispatch(&mut state, &firmware, &[101, 24]);
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

        dispatch(&mut state, &firmware, &[101, 24, b'L', b'O', b'N', b'G', 0]);
        dispatch(&mut state, &firmware, &[101, 24, b'N']);

        assert_eq!(
            dispatch(&mut state, &firmware, &[101, 27]),
            [101, 27, b'N', 0]
        );
    }

    #[test]
    fn lcm_name_stops_at_nul_and_at_refloats_twenty_byte_limit() {
        let firmware = FirmwareTest::new();
        let mut state = external_state();

        dispatch(&mut state, &firmware, &[101, 24, b'A', 0, b'B']);
        assert_eq!(
            dispatch(&mut state, &firmware, &[101, 27]),
            [101, 27, b'A', 0]
        );

        let mut poll = vec![101, 24];
        poll.extend(1_u8..=MAX_LCM_NAME_LENGTH as u8 + 1);
        dispatch(&mut state, &firmware, &poll);
        let mut expected = vec![101, 27];
        expected.extend(1_u8..=MAX_LCM_NAME_LENGTH as u8);
        assert_eq!(dispatch(&mut state, &firmware, &[101, 27]), expected);
    }

    #[test]
    fn battery_response_uses_float32_auto_and_disabled_lcm_stays_minimal() {
        let firmware = FirmwareTest::new();
        let mut state = external_state();
        assert_eq!(dispatch(&mut state, &firmware, &[101, 29]).len(), 6);

        state.set_lcm_hardware_mode_for_test(0);
        assert_eq!(dispatch(&mut state, &firmware, &[101, 25]), [101, 25]);
        assert_eq!(dispatch(&mut state, &firmware, &[101, 24]), [101, 24]);
    }
}
