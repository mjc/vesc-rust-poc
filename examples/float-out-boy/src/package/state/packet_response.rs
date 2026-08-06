use super::super::protocol::{
    encode_float_out_boy_all_data_fault_response,
    encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_info_response, encode_float_out_boy_realtime_data_ids_response,
    encode_float_out_boy_realtime_data_response_with_runtime,
    encode_float_out_boy_realtime_selected_response,
};
use super::FloatOutBoyPackageState;
use super::float_out_boy_command_payload;
use crate::domain::{
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataRequest, FloatOutBoyAppDataCommand as Command,
    FloatOutBoyFatalErrorState as FatalError, FloatOutBoyRealtimeDataHeader,
    FloatOutBoyRealtimeMotorTemperatures, FloatOutBoyRealtimeSelectedRequest,
    FloatOutBoyRealtimeTail,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{BatteryVoltage, FirmwareFault, TimestampTicks};

impl FloatOutBoyPackageState {
    pub(super) fn reply_to_metadata_packet(
        &self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        if let Some(payload) = float_out_boy_command_payload(bytes, Command::Info) {
            let internal_leds_operational = self.internal_leds_operational();
            // C map: `on_command_received` dispatches COMMAND_INFO at
            // `third_party/float-out-boy/src/main.c:2158-2160`; `cmd_info` writes
            // the requested v1 or v2 metadata shape at
            // `third_party/float-out-boy/src/main.c:2070-2139`.
            let response = encode_float_out_boy_info_response(
                payload,
                self.serialized_config.hardware_led_mode_id(),
                internal_leds_operational,
                self.data_recorder.has_capability(),
            );
            return reply(response.as_bytes());
        }

        if float_out_boy_command_payload(bytes, Command::RealtimeDataIds).is_some() {
            // C map: `on_command_received` dispatches realtime-data IDs at
            // `third_party/float-out-boy/src/main.c:2275-2277`; `cmd_realtime_data_ids`
            // sends the counted ID table at `third_party/float-out-boy/src/main.c:1876-1901`.
            // Keep the response as callback-local bytes like upstream's stack buffer.
            let response = encode_float_out_boy_realtime_data_ids_response();
            return reply(&response);
        }

        false
    }

    pub(super) fn reply_to_legacy_realtime_data_packet(
        &self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some(_) = float_out_boy_command_payload(bytes, Command::GetRealtimeData) else {
            return false;
        };
        // C map: `on_command_received` dispatches legacy `COMMAND_GET_RTDATA` at
        // `third_party/float-out-boy/src/main.c:2162-2164`.
        let response = encode_float_out_boy_get_realtime_data_response_with_remote(
            &self.all_data_payloads,
            self.remote_control.input(),
            self.ride_modifiers.atr_accel_diff(),
        );
        reply(&response)
    }

    pub(super) fn reply_to_realtime_data_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some(_) = float_out_boy_command_payload(bytes, Command::RealtimeData) else {
            return false;
        };
        let payloads = self
            .all_data_payloads
            .with_motor_battery_voltage(BatteryVoltage::new(telemetry.input_voltage().voltage()))
            .with_temperatures(FloatOutBoyRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));
        // Float Out Boy's main loop updates `d->time.now` before app-data reads it
        // in `cmd_realtime_data` at `third_party/float-out-boy/src/main.c:1931`.
        let fatal = if self.alert_tracker.fatal_error() {
            FatalError::Present
        } else {
            FatalError::None
        };
        let header = FloatOutBoyRealtimeDataHeader::new(
            now(),
            payloads.ride_state(),
            payloads.footpad().state(),
            payloads.beep_reason(),
        )
        .with_fatal_error(fatal)
        .with_data_recorder(self.data_recorder.flags());
        let tail = FloatOutBoyRealtimeTail::new(
            self.alert_tracker.firmware_fault_active(),
            self.alert_tracker.firmware_fault_code(),
        );
        let response = encode_float_out_boy_realtime_data_response_with_runtime(
            &payloads,
            header,
            tail,
            self.realtime_live_values(),
        );
        reply(response.as_bytes())
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn reply_to_realtime_selected_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some(payload) = float_out_boy_command_payload(bytes, Command::RealtimeDataSelected)
        else {
            return false;
        };
        let Some(request) = FloatOutBoyRealtimeSelectedRequest::parse(payload) else {
            return false;
        };
        let gnss = if request.mask2().selects_gnss() {
            let Ok(snapshot) = vescpkg_rs::Gnss.snapshot() else {
                return false;
            };
            Some(snapshot)
        } else {
            None
        };
        let payloads = Self::runtime_all_data_payloads(
            self.all_data_payloads
                .with_motor_battery_voltage(BatteryVoltage::new(
                    telemetry.input_voltage().voltage(),
                )),
            telemetry,
            true,
        );
        let header = FloatOutBoyRealtimeDataHeader::new(
            now(),
            payloads.ride_state(),
            payloads.footpad().state(),
            payloads.beep_reason(),
        )
        .with_fatal_error(if self.alert_tracker.fatal_error() {
            FatalError::Present
        } else {
            FatalError::None
        })
        .with_data_recorder(self.data_recorder.flags());
        let response = encode_float_out_boy_realtime_selected_response(
            request,
            &payloads,
            header,
            self.realtime_live_values(),
            gnss,
        );
        reply(response.as_bytes())
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn reply_to_all_data_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        // C map: `on_command_received` only calls `cmd_send_all_data` for
        // three-byte COMMAND_GET_ALLDATA packets at `third_party/float-out-boy/src/main.c:2212-2218`.
        match (
            FloatOutBoyAllDataRequest::parse(bytes),
            telemetry.firmware_fault(),
        ) {
            (Err(_), _) | (Ok(_), FirmwareFault::Unknown) => false,
            (Ok(_), FirmwareFault::Active(fault)) => {
                let response = encode_float_out_boy_all_data_fault_response(fault.wire_code());
                reply(response.as_bytes())
            }
            // Preserve the fail-closed behavior for an ABI value this SDK
            // cannot safely represent in Float Out Boy's wire format.
            (Ok(request), _) => {
                let mode = request.mode();
                let payloads =
                    self.all_data_payloads
                        .with_motor_battery_voltage(BatteryVoltage::new(
                            telemetry.input_voltage().voltage(),
                        ));
                let payloads = if mode.includes_mode2() {
                    Self::runtime_all_data_payloads(payloads, telemetry, mode.includes_mode3())
                } else {
                    payloads
                };
                let response = payloads.encode_response(request);
                // Refloat commit 98bfe765 keeps the last reason available to
                // later command-7 readers after its active condition ends.
                reply(response.as_bytes())
            }
        }
    }

    fn runtime_all_data_payloads(
        payloads: FloatOutBoyAllDataPayloads,
        telemetry: &impl MotorTelemetry,
        include_mode3: bool,
    ) -> FloatOutBoyAllDataPayloads {
        // C map: mode >= 2 samples slower telemetry fields at
        // `third_party/float-out-boy/src/main.c:1373-1379`; mode >= 3 appends ride
        // totals at `third_party/float-out-boy/src/main.c:1381-1389`.
        let payloads = payloads
            .with_distance_abs(telemetry.trip_distance())
            .with_temperatures(FloatOutBoyRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads
                .with_odometer(telemetry.odometer())
                .with_discharged_charge(telemetry.amp_hours_discharged())
                .with_charged_charge(telemetry.amp_hours_charged())
                .with_discharged_energy(telemetry.watt_hours_discharged())
                .with_charged_energy(telemetry.watt_hours_charged())
                .with_battery_level(telemetry.battery_level())
        } else {
            payloads
        }
    }
}

#[cfg(test)]
mod tests;
