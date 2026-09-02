use super::super::protocol::{
    encode_float_out_boy_all_data_fault_response,
    encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_info_response, encode_float_out_boy_realtime_data_ids_response,
    encode_float_out_boy_realtime_data_response_with_runtime,
    encode_float_out_boy_realtime_selected_response,
};
use super::FloatOutBoyPackageState;
use crate::domain::{
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataRequest, FloatOutBoyAppDataCommand as Command,
    FloatOutBoyFatalErrorState as FatalError, FloatOutBoyRealtimeDataHeader,
    FloatOutBoyRealtimeMotorTemperatures, FloatOutBoyRealtimeSelectedRequest,
    FloatOutBoyRealtimeTail,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{BatteryVoltage, FirmwareFault, TimestampTicks};

#[cfg(test)]
fn test_command(bytes: &[u8]) -> Option<(Command, &[u8])> {
    vescpkg_rs::protocol_app_data::parse_app_data_command(
        bytes,
        crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
    )
}

impl FloatOutBoyPackageState {
    fn realtime_header(
        &self,
        payloads: &FloatOutBoyAllDataPayloads,
        timestamp: TimestampTicks,
    ) -> FloatOutBoyRealtimeDataHeader {
        FloatOutBoyRealtimeDataHeader::new(
            timestamp,
            payloads.ride_state(),
            payloads.footpad().state(),
            payloads.beep_reason(),
        )
        .with_fatal_error(if self.alert_tracker.fatal_error() {
            FatalError::Present
        } else {
            FatalError::None
        })
        .with_data_recorder(self.data_recorder.0.available_flags())
    }

    pub(super) fn reply_to_metadata_command(
        &self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: Command,
        payload: &[u8],
    ) -> bool {
        if command == Command::Info {
            // C map: `on_command_received` dispatches COMMAND_INFO at
            // `third_party/float-out-boy/src/main.c:2158-2160`; `cmd_info` writes
            // the requested v1 or v2 metadata shape at
            // `third_party/float-out-boy/src/main.c:2070-2139`.
            let response = encode_float_out_boy_info_response(
                payload,
                self.serialized_config.hardware_led_mode().id(),
                self.internal_leds_operational(),
                self.data_recorder.0.has_capability(),
            );
            return reply(response.as_bytes());
        }

        if command == Command::RealtimeDataIds {
            // C map: `on_command_received` dispatches realtime-data IDs at
            // `third_party/float-out-boy/src/main.c:2275-2277`; `cmd_realtime_data_ids`
            // sends the counted ID table at `third_party/float-out-boy/src/main.c:1876-1901`.
            // Keep the response as callback-local bytes like upstream's stack buffer.
            return reply(&encode_float_out_boy_realtime_data_ids_response());
        }

        false
    }

    pub(super) fn reply_to_legacy_realtime_data_command(
        &self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: Command,
    ) -> bool {
        if command != Command::GetRealtimeData {
            return false;
        }
        // C map: `on_command_received` dispatches legacy `COMMAND_GET_RTDATA` at
        // `third_party/float-out-boy/src/main.c:2162-2164`.
        reply(
            &encode_float_out_boy_get_realtime_data_response_with_remote(
                &self.all_data_payloads,
                self.remote_control.input(),
                self.ride_modifiers.atr_accel_diff(),
            ),
        )
    }

    pub(super) fn reply_to_realtime_data_command(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: Command,
    ) -> bool {
        if command != Command::RealtimeData {
            return false;
        }
        let payloads = self
            .all_data_payloads
            .with_motor_battery_voltage(BatteryVoltage::new(telemetry.input_voltage().voltage()))
            .with_temperatures(FloatOutBoyRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));
        // Float Out Boy's main loop updates `d->time.now` before app-data reads it
        // in `cmd_realtime_data` at `third_party/float-out-boy/src/main.c:1931`.
        let header = self.realtime_header(&payloads, now());
        let tail = FloatOutBoyRealtimeTail::new(
            self.alert_tracker.firmware_fault_active(),
            self.alert_tracker.firmware_fault_code(),
        );
        reply(
            encode_float_out_boy_realtime_data_response_with_runtime(
                &payloads,
                header,
                tail,
                self.realtime_live_values(),
            )
            .as_bytes(),
        )
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn reply_to_realtime_selected_command(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: Command,
        payload: &[u8],
    ) -> bool {
        if command != Command::RealtimeDataSelected {
            return false;
        }
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
        let header = self.realtime_header(&payloads, now());
        reply(
            encode_float_out_boy_realtime_selected_response(
                request,
                &payloads,
                header,
                self.realtime_live_values(),
                gnss,
            )
            .as_bytes(),
        )
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn reply_to_all_data_command(
        &self,
        telemetry: &impl MotorTelemetry,
        reply: &mut impl FnMut(&[u8]) -> bool,
        command: Command,
        payload: &[u8],
    ) -> bool {
        // C map: `on_command_received` only calls `cmd_send_all_data` for
        // three-byte COMMAND_GET_ALLDATA packets at `third_party/float-out-boy/src/main.c:2212-2218`.
        let request = (command == Command::GetAllData)
            .then_some(FloatOutBoyAllDataRequest::parse_payload(payload))
            .flatten();
        match (request, telemetry.firmware_fault()) {
            (None, _) | (Some(_), FirmwareFault::Unknown) => false,
            (Some(_), FirmwareFault::Active(fault)) => {
                reply(encode_float_out_boy_all_data_fault_response(fault.wire_code()).as_bytes())
            }
            // Preserve the fail-closed behavior for an ABI value this SDK
            // cannot safely represent in Float Out Boy's wire format.
            (Some(request), _) => {
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
                // Refloat commit 98bfe765 keeps the last reason available to
                // later command-7 readers after its active condition ends.
                reply(payloads.encode_response(request).as_bytes())
            }
        }
    }

    #[cfg(test)]
    pub(super) fn reply_to_metadata_packet(
        &self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        test_command(bytes).is_some_and(|(command, payload)| {
            self.reply_to_metadata_command(reply, command, payload)
        })
    }

    #[cfg(test)]
    pub(super) fn reply_to_realtime_data_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        test_command(bytes).is_some_and(|(command, _)| {
            self.reply_to_realtime_data_command(telemetry, now, reply, command)
        })
    }

    #[cfg(test)]
    pub(super) fn reply_to_all_data_packet(
        &self,
        telemetry: &impl MotorTelemetry,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        test_command(bytes).is_some_and(|(command, payload)| {
            self.reply_to_all_data_command(telemetry, reply, command, payload)
        })
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
