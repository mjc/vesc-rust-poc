use super::super::protocol::{
    encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_info_response, encode_float_out_boy_realtime_data_ids_response,
    encode_float_out_boy_realtime_data_response_with_runtime,
};
use super::FloatOutBoyPackageState;
use super::float_out_boy_command_payload;
use crate::domain::{
    FloatOutBoyAllDataMode3Payload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataRequest,
    FloatOutBoyAllDataResponse, FloatOutBoyAppDataCommand, FloatOutBoyRealtimeDataHeader,
    FloatOutBoyRealtimeMotorTemperatures, FloatOutBoyRealtimeReservedFlags,
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
        if let Some(payload) = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::Info)
        {
            #[cfg(any(test, target_arch = "arm"))]
            let internal_leds_operational = self.internal_leds_operational();
            #[cfg(all(not(test), not(target_arch = "arm")))]
            let internal_leds_operational = false;
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

        if float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::RealtimeDataIds)
            .is_some()
        {
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
        match float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::GetRealtimeData) {
            Some(_) => {
                // C map: `on_command_received` dispatches legacy `COMMAND_GET_RTDATA` at
                // `third_party/float-out-boy/src/main.c:2162-2164`.
                let response = encode_float_out_boy_get_realtime_data_response_with_remote(
                    &self.all_data_payloads,
                    self.remote_control.input(),
                    self.ride_modifiers.atr_accel_diff(),
                );
                reply(&response)
            }
            None => false,
        }
    }

    pub(super) fn reply_to_realtime_data_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        match float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::RealtimeData) {
            Some(_) => {
                let payloads = self
                    .all_data_payloads
                    .with_base_battery_voltage(BatteryVoltage::new(
                        telemetry.input_voltage().voltage(),
                    ))
                    .with_mode2_temperatures(FloatOutBoyRealtimeMotorTemperatures::new(
                        telemetry.mosfet_temperature(),
                        telemetry.motor_temperature(),
                    ));
                // Float Out Boy's main loop updates `d->time.now` before app-data reads it
                // in `cmd_realtime_data` at `third_party/float-out-boy/src/main.c:1931`.
                let system_timestamp = now();
                let base = payloads.base();
                let header = FloatOutBoyRealtimeDataHeader::new(
                    system_timestamp,
                    base.status().ride_state(),
                    base.footpad().state(),
                    base.status().beep_reason(),
                )
                .with_fatal_error(self.alert_tracker.fatal_error())
                .with_data_recorder(self.data_recorder.flags());
                let tail = FloatOutBoyRealtimeTail::new(
                    self.alert_tracker.active_alerts(),
                    FloatOutBoyRealtimeReservedFlags::none(),
                    self.alert_tracker.firmware_fault_code(),
                );
                let response = encode_float_out_boy_realtime_data_response_with_runtime(
                    &payloads,
                    header,
                    tail,
                    self.remote_control.input(),
                    self.ride_modifiers.atr_accel_diff(),
                    self.ride_modifiers.atr_speed_boost(),
                );
                reply(response.as_bytes())
            }
            None => false,
        }
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
                let response = FloatOutBoyAllDataResponse::fault(fault.wire_code());
                reply(response.as_bytes())
            }
            // Preserve the fail-closed behavior for an ABI value this SDK
            // cannot safely represent in Float Out Boy's wire format.
            (Ok(request), _) => {
                let mode = request.mode();
                let payloads =
                    self.all_data_payloads
                        .with_base_battery_voltage(BatteryVoltage::new(
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
            .with_mode2_distance_abs(telemetry.trip_distance())
            .with_mode2_temperatures(FloatOutBoyRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads.with_mode3_ride_totals(FloatOutBoyAllDataMode3Payload::new(
                telemetry.odometer(),
                telemetry.amp_hours_discharged(),
                telemetry.amp_hours_charged(),
                telemetry.watt_hours_discharged(),
                telemetry.watt_hours_charged(),
                telemetry.battery_level(),
            ))
        } else {
            payloads
        }
    }
}

#[cfg(test)]
mod tests;
