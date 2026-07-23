//! External LCM protocol state and packet handling.
//!
//! The wire shapes mirror Float Out Boy v1.2.1 `src/lcm.c` and the lights
//! command in `src/main.c`. This module owns only the external protocol seam;
//! internal LED DMA rendering remains a separate runtime slice.

use super::FloatOutBoyPackageState;
use super::float_out_boy_command_payload;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
    FloatOutBoyRunState,
};
use crate::wire::{degrees, push_float32_auto, push_u8, push_u16};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::FirmwareFault;

const MAX_LCM_NAME_LENGTH: usize = 20;
const MAX_LCM_PAYLOAD_LENGTH: usize = 64;
const POLL_RESPONSE_CAPACITY: usize = 2 + 3 + 6 + 3 + MAX_LCM_PAYLOAD_LENGTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LcmState {
    hardware_mode: u8,
    brightness: u8,
    brightness_idle: u8,
    status_brightness: u8,
    lights_off_when_lifted: bool,
    lights_enabled: bool,
    headlights_enabled: bool,
    name: [u8; MAX_LCM_NAME_LENGTH],
    payload: [u8; MAX_LCM_PAYLOAD_LENGTH],
    payload_size: usize,
}

impl LcmState {
    // Keep the buffer initialization in its own frame so the loader's direct
    // `package_lib_init` frame stays below the 1,024-byte stack budget.
    #[inline(never)]
    pub(super) fn new(hardware_mode: u8) -> Self {
        Self {
            hardware_mode,
            brightness: 0,
            brightness_idle: 0,
            status_brightness: 0,
            lights_off_when_lifted: true,
            lights_enabled: false,
            headlights_enabled: false,
            name: [0; MAX_LCM_NAME_LENGTH],
            payload: [0; MAX_LCM_PAYLOAD_LENGTH],
            payload_size: 0,
        }
    }

    pub(super) const fn set_hardware_mode(&mut self, hardware_mode: u8) {
        self.hardware_mode = hardware_mode;
    }

    const fn enabled(self) -> bool {
        self.hardware_mode & 0x2 != 0
    }

    fn poll_request(&mut self, payload: &[u8]) {
        if !self.enabled() || payload.is_empty() {
            return;
        }

        for (index, byte) in payload
            .iter()
            .copied()
            .take(MAX_LCM_NAME_LENGTH)
            .enumerate()
        {
            self.name[index] = byte;
            if byte == 0 {
                break;
            }
        }
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

    fn lights_control(&mut self, payload: &[u8]) {
        // `lights_control_request` requires a four-byte mask and one value,
        // then ignores masks outside the low byte.
        if payload.len() < 5 {
            return;
        }

        let mask = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
        if mask & 0xff == 0 {
            return;
        }

        let value = payload[4];
        if mask & 0x1 != 0 {
            self.lights_enabled = value & 0x1 != 0;
        }
        if mask & 0x2 != 0 {
            self.headlights_enabled = value & 0x2 != 0;
        }
    }

    fn poll_response(
        &mut self,
        payloads: crate::domain::FloatOutBoyAllDataPayloads,
        telemetry: &impl MotorTelemetry,
    ) -> LcmPacket<POLL_RESPONSE_CAPACITY> {
        let mut packet = LcmPacket::new();
        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
        packet.push(FloatOutBoyAppDataCommand::LcmPoll.id());

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
        for byte in self.payload.iter().copied().take(self.payload_size) {
            packet.push(byte);
        }
        self.payload_size = 0;
        packet
    }

    fn light_info_response(self) -> LcmPacket<12> {
        let mut packet = LcmPacket::new();
        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
        packet.push(FloatOutBoyAppDataCommand::LcmLightInfo.id());
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
        let mut packet = LcmPacket::new();
        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
        packet.push(FloatOutBoyAppDataCommand::LcmDeviceInfo.id());
        if self.enabled() {
            for byte in self.name.iter().copied() {
                packet.push(byte);
                if byte == 0 {
                    break;
                }
            }
        }
        packet
    }

    fn battery_response(self, telemetry: &impl MotorTelemetry) -> LcmPacket<6> {
        let mut packet = LcmPacket::new();
        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
        packet.push(FloatOutBoyAppDataCommand::LcmGetBattery.id());
        if self.enabled() {
            packet.push_float32_auto(telemetry.battery_level().as_fraction());
        }
        packet
    }

    fn lights_control_response(self) -> [u8; 3] {
        [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::LightsControl.id(),
            u8::from(self.lights_enabled) | (u8::from(self.headlights_enabled) << 1),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LcmPacket<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LcmPacket<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) {
        push_u8(&mut self.bytes, &mut self.len, byte);
    }

    fn push_scaled_i16(&mut self, value: f32, scale: f32) {
        push_u16(
            &mut self.bytes,
            &mut self.len,
            (value * scale) as i16 as u16,
        );
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
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        if let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LcmPoll)
        {
            self.lcm.poll_request(payload);
            let packet = self.lcm.poll_response(self.all_data_payloads, telemetry);
            return send(packet.bytes());
        }
        if float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LcmLightInfo).is_some() {
            let packet = self.lcm.light_info_response();
            return send(packet.bytes());
        }
        if let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LcmLightControl)
        {
            self.lcm.light_control(payload);
            return true;
        }
        if float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LcmDeviceInfo).is_some()
        {
            let packet = self.lcm.device_info_response();
            return send(packet.bytes());
        }
        if float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LcmGetBattery).is_some()
        {
            let packet = self.lcm.battery_response(telemetry);
            return send(packet.bytes());
        }
        if let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::LightsControl)
        {
            self.lcm.lights_control(payload);
            return send(&self.lcm.lights_control_response());
        }
        false
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
    fn battery_response_uses_float32_auto_and_disabled_lcm_stays_minimal() {
        let firmware = FirmwareTest::new();
        let mut state = external_state();
        assert_eq!(dispatch(&mut state, &firmware, &[101, 29]).len(), 6);

        state.set_lcm_hardware_mode_for_test(0);
        assert_eq!(dispatch(&mut state, &firmware, &[101, 25]), [101, 25]);
        assert_eq!(dispatch(&mut state, &firmware, &[101, 24]), [101, 24]);
    }
}
