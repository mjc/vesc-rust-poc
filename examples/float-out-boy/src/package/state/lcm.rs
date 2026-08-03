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
use crate::wire::{FloatOutBoyPacket, degrees};
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
    if !config.on {
        return [0; 3];
    }

    let front = config.front.brightness;
    let (active, status) = if config.headlights_on {
        (
            config.headlights.brightness,
            config.status.brightness_headlights_on,
        )
    } else {
        (front, config.status.brightness_headlights_off)
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
        self.lights_off_when_lifted = config.lifted.lights_off;
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
    ) -> FloatOutBoyPacket<POLL_RESPONSE_CAPACITY> {
        let mut packet = lcm_packet(FloatOutBoyAppDataCommand::LcmPoll);

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

    fn light_info_response(self) -> FloatOutBoyPacket<12> {
        let mut packet = lcm_packet(FloatOutBoyAppDataCommand::LcmLightInfo);
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

    fn device_info_response(self) -> FloatOutBoyPacket<22> {
        let mut packet = lcm_packet(FloatOutBoyAppDataCommand::LcmDeviceInfo);
        if self.enabled() {
            packet.extend(nul_terminated_prefix(&self.name));
        }
        packet
    }

    fn battery_response(self, telemetry: &impl MotorTelemetry) -> FloatOutBoyPacket<6> {
        let mut packet = lcm_packet(FloatOutBoyAppDataCommand::LcmGetBattery);
        if self.enabled() {
            packet.push_float32_auto(telemetry.battery_level().as_fraction());
        }
        packet
    }
}

fn lcm_packet<const N: usize>(command: FloatOutBoyAppDataCommand) -> FloatOutBoyPacket<N> {
    let mut packet = FloatOutBoyPacket::new();
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
    packet.push(command.id());
    packet
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
                        .as_bytes(),
                )
            }
            Command::LcmLightInfo => reply(self.lcm.light_info_response().as_bytes()),
            Command::LcmLightControl => {
                self.lcm.light_control(payload);
                true
            }
            Command::LcmDeviceInfo => reply(self.lcm.device_info_response().as_bytes()),
            Command::LcmGetBattery => reply(self.lcm.battery_response(telemetry).as_bytes()),
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
                    u8::from(status.enabled) | (u8::from(status.headlights_enabled) << 1),
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
mod tests;
