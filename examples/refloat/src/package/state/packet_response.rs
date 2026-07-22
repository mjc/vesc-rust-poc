use super::super::protocol::{
    encode_refloat_get_realtime_data_response_with_remote, encode_refloat_info_response,
    encode_refloat_realtime_data_ids_response, encode_refloat_realtime_data_response_with_runtime,
};
use super::RefloatPackageState;
use super::refloat_command_payload;
use crate::domain::{
    RefloatAllDataMode3Payload, RefloatAllDataPayloads, RefloatAllDataRequest,
    RefloatAllDataResponse, RefloatAppDataCommand, RefloatDataRecorderFlags,
    RefloatRealtimeDataHeader, RefloatRealtimeMotorTemperatures, RefloatRealtimeReservedFlags,
    RefloatRealtimeTail,
};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{BatteryVoltage, FirmwareFaultWireCode, TimestampTicks};

impl RefloatPackageState {
    pub(super) fn send_metadata_packet_response(
        &self,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        if let Some(payload) = refloat_command_payload(bytes, RefloatAppDataCommand::Info) {
            // C map: `on_command_received` dispatches COMMAND_INFO at
            // `third_party/refloat/src/main.c:2158-2160`; `cmd_info` writes
            // the requested v1 or v2 metadata shape at
            // `third_party/refloat/src/main.c:2070-2139`.
            let response = encode_refloat_info_response(
                payload,
                self.serialized_config.hardware_led_mode_id(),
            );
            return send(response.as_bytes());
        }

        if refloat_command_payload(bytes, RefloatAppDataCommand::RealtimeDataIds).is_some() {
            // C map: `on_command_received` dispatches realtime-data IDs at
            // `third_party/refloat/src/main.c:2275-2277`; `cmd_realtime_data_ids`
            // sends the counted ID table at `third_party/refloat/src/main.c:1876-1901`.
            // Keep the response as callback-local bytes like upstream's stack buffer.
            let response = encode_refloat_realtime_data_ids_response();
            return send(&response);
        }

        false
    }

    pub(super) fn send_legacy_realtime_data_packet_response(
        &self,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        match refloat_command_payload(bytes, RefloatAppDataCommand::GetRealtimeData) {
            Some(_) => {
                // C map: `on_command_received` dispatches legacy `COMMAND_GET_RTDATA` at
                // `third_party/refloat/src/main.c:2162-2164`.
                let response = encode_refloat_get_realtime_data_response_with_remote(
                    &self.all_data_payloads,
                    self.remote_control.input(),
                    self.ride_modifiers.atr_accel_diff(),
                );
                send(&response)
            }
            None => false,
        }
    }

    pub(super) fn send_realtime_data_packet_response(
        &self,
        telemetry: &impl MotorTelemetry,
        now: &mut impl FnMut() -> TimestampTicks,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        match refloat_command_payload(bytes, RefloatAppDataCommand::RealtimeData) {
            Some(_) => {
                let payloads = self
                    .all_data_payloads
                    .with_base_battery_voltage(BatteryVoltage::new(
                        telemetry.input_voltage().voltage(),
                    ))
                    .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                        telemetry.mosfet_temperature(),
                        telemetry.motor_temperature(),
                    ));
                // Refloat's main loop updates `d->time.now` before app-data reads it
                // in `cmd_realtime_data` at `third_party/refloat/src/main.c:1931`.
                let system_timestamp = now();
                let base = payloads.base();
                let header = RefloatRealtimeDataHeader::new(
                    system_timestamp,
                    base.status().ride_state(),
                    base.footpad().state(),
                    base.status().beep_reason(),
                )
                .with_fatal_error(self.alert_tracker.fatal_error())
                .with_data_recorder(RefloatDataRecorderFlags::inactive());
                let tail = RefloatRealtimeTail::new(
                    self.alert_tracker.active_alerts(),
                    RefloatRealtimeReservedFlags::none(),
                    self.alert_tracker.firmware_fault_code(),
                );
                let response = encode_refloat_realtime_data_response_with_runtime(
                    &payloads,
                    header,
                    tail,
                    self.remote_control.input(),
                    self.ride_modifiers.atr_accel_diff(),
                    self.ride_modifiers.atr_speed_boost(),
                );
                send(response.as_bytes())
            }
            None => false,
        }
    }

    pub(super) fn send_all_data_packet_response(
        &self,
        telemetry: &impl MotorTelemetry,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        // C map: `on_command_received` only calls `cmd_send_all_data` for
        // three-byte COMMAND_GET_ALLDATA packets at `third_party/refloat/src/main.c:2212-2218`.
        match (
            RefloatAllDataRequest::parse(bytes),
            telemetry.firmware_fault(),
        ) {
            (Err(_), _) => false,
            (Ok(_), fault) if !fault.is_none() => {
                FirmwareFaultWireCode::try_from(fault).is_ok_and(|fault| {
                    let response = RefloatAllDataResponse::fault(fault);
                    send(response.as_bytes())
                })
            }
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
                send(response.as_bytes())
            }
        }
    }

    fn runtime_all_data_payloads(
        payloads: RefloatAllDataPayloads,
        telemetry: &impl MotorTelemetry,
        include_mode3: bool,
    ) -> RefloatAllDataPayloads {
        // C map: mode >= 2 samples slower telemetry fields at
        // `third_party/refloat/src/main.c:1373-1379`; mode >= 3 appends ride
        // totals at `third_party/refloat/src/main.c:1381-1389`.
        let payloads = payloads
            .with_mode2_distance_abs(telemetry.trip_distance())
            .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads.with_mode3_ride_totals(RefloatAllDataMode3Payload::new(
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
mod tests {
    use super::*;
    use crate::domain::REFLOAT_APP_DATA_PACKAGE_ID;
    use std::vec::Vec;
    use vescpkg_rs::prelude::FirmwareFaultCode;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn realtime_packet_response_uses_system_ticks_like_refloat() {
        let app_data = TimestampTicks::from_ticks(0x0102_0304);
        let telemetry = FirmwareTest::new();
        let state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        let mut packet = Vec::new();
        let mut now = || app_data;
        let mut send = |bytes: &[u8]| {
            packet.extend_from_slice(bytes);
            true
        };

        assert!(state.send_realtime_data_packet_response(
            telemetry.telemetry(),
            &mut now,
            &mut send,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Refloat v1.2.1 writes `d->time.now` into realtime packets at
        // `third_party/refloat/src/main.c:1931`; VESC system ticks are 100 us ticks.
        assert_eq!(&packet[4..8], &[1, 2, 3, 4]);
    }

    #[test]
    fn realtime_packet_reports_live_firmware_fault_alert_like_refloat() {
        let now = TimestampTicks::from_ticks(42);
        let firmware =
            FirmwareTest::new().with_firmware_fault(FirmwareFaultCode::from_wire_code(5));
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);
        let mut packet = Vec::new();

        assert!(state.send_realtime_data_packet_response(
            firmware.telemetry(),
            &mut || now,
            &mut |bytes| {
                packet.extend_from_slice(bytes);
                true
            },
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        assert_eq!(packet[3] & 0x08, 0x08);
        assert_eq!(&packet[packet.len() - 9..packet.len() - 5], &[0, 0, 0, 1]);
        assert_eq!(packet.last(), Some(&5));
    }

    #[test]
    fn alerts_list_command_returns_source_header_when_empty() {
        let firmware = FirmwareTest::new();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        let mut packet = Vec::new();

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |bytes| {
                packet.extend_from_slice(bytes);
                true
            },
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::AlertsList.id(),
            ],
        ));

        assert_eq!(packet, [101, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn alerts_list_command_returns_firmware_fault_name_and_record() {
        let now = TimestampTicks::from_ticks(42);
        let firmware =
            FirmwareTest::new().with_firmware_fault(FirmwareFaultCode::from_wire_code(5));
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);
        let mut packet = Vec::new();

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || now,
            &mut |bytes| {
                packet.extend_from_slice(bytes);
                true
            },
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::AlertsList.id(),
            ],
        ));

        let name = b"OVER_TEMP_FET";
        assert_eq!(&packet[..11], &[101, 35, 0, 0, 0, 1, 0, 0, 0, 0, 5]);
        assert_eq!(packet[11], name.len() as u8);
        assert_eq!(&packet[12..25], name);
        assert_eq!(&packet[25..34], &[1, 0, 0, 0, 42, 1, 1, 5, 13]);
        assert_eq!(&packet[34..], name);
    }

    #[test]
    fn alerts_control_clears_the_persistent_fatal_without_hiding_the_live_fault() {
        let now = TimestampTicks::from_ticks(42);
        let firmware =
            FirmwareTest::new().with_firmware_fault(FirmwareFaultCode::from_wire_code(5));
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        state.refresh_runtime_state(firmware.telemetry(), firmware.imu(), now);

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || now,
            &mut |_| true,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::AlertsControl.id(),
                1,
            ],
        ));

        let mut packet = Vec::new();
        assert!(state.send_realtime_data_packet_response(
            firmware.telemetry(),
            &mut || now,
            &mut |bytes| {
                packet.extend_from_slice(bytes);
                true
            },
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        assert_eq!(packet[3] & 0x08, 0);
        assert_eq!(&packet[packet.len() - 9..packet.len() - 5], &[0, 0, 0, 1]);
        assert_eq!(packet.last(), Some(&5));
    }

    #[test]
    fn metadata_packet_response_defaults_to_legacy_info_like_refloat() {
        let state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        let mut packet = Vec::new();

        assert!(state.send_metadata_packet_response(
            &mut |bytes| {
                packet.extend_from_slice(bytes);
                true
            },
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::Info.id(),
            ],
        ));

        assert_eq!(packet, [101, 0, 12, 1, 0]);
    }

    #[test]
    fn metadata_packet_response_sends_realtime_ids_directly() {
        let state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        let mut packet = Vec::new();
        let mut send = |bytes: &[u8]| {
            packet.extend_from_slice(bytes);
            true
        };

        assert!(state.send_metadata_packet_response(
            &mut send,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeDataIds.id(),
            ],
        ));

        // C map: QML asks for this packet at `ui.qml.in:704-705`; Refloat C replies
        // from `third_party/refloat/src/main.c:1876-1901`.
        assert_eq!(packet.len(), 405);
        assert_eq!(
            &packet[..3],
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeDataIds.id(),
                16,
            ]
        );
    }
}
