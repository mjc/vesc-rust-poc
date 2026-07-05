//! Refloat app-data packet processing.

use crate::domain::{
    RefloatAllDataMode3Payload, RefloatAllDataMode4Payload, RefloatAllDataPayloads,
    RefloatAllDataRequest, RefloatAllDataResponse, RefloatAppDataCommand, RefloatFirmwareFaultCode,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
    RefloatRealtimeMotorTemperatures,
};
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, LoopbackLifecycle, MotorTelemetryApi,
    MotorTelemetryBindings, ffi,
};

/// Process one Refloat app-data packet from a typed all-data payload snapshot.
pub fn process_refloat_app_data(
    payloads: RefloatAllDataPayloads,
    bytes: &[u8],
) -> Option<RefloatAllDataResponse> {
    let request = RefloatAllDataRequest::parse(bytes).ok()?;
    Some(payloads.encode_response(request))
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn handle_refloat_app_data_packet<B: AppDataBindings, M: MotorTelemetryBindings>(
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    data: *mut u8,
    len: u32,
) -> bool {
    let Some(data) = core::ptr::NonNull::new(data) else {
        return false;
    };
    let Ok(len) = usize::try_from(len) else {
        return false;
    };
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr().cast_const(), len) };
    state.handle_packet_with_telemetry(lifecycle, telemetry, bytes)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn loaded_image_base() -> usize {
    let loaded_handler: usize;
    unsafe {
        core::arch::asm!(
            "adr {loaded_handler}, {handler}",
            loaded_handler = out(reg) loaded_handler,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
    }
    let loaded_handler = loaded_handler & !1;
    let image_handler = refloat_handle_app_data as *const () as usize & !1;
    loaded_handler - image_handler
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn refloat_state_from_arg() -> Option<&'static mut RefloatAppDataState> {
    let arg_slot = unsafe { ffi::raw::vesc_get_arg(loaded_image_base() as u32) };
    let arg_slot = unsafe { arg_slot.as_mut()? };
    let state = (*arg_slot).cast::<RefloatAppDataState>();
    unsafe { state.as_mut() }
}

/// Device entrypoint invoked by firmware app-data delivery.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn refloat_handle_app_data(data: *mut u8, len: u32) {
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
    let _ = unsafe { handle_refloat_app_data_packet(state, &lifecycle, &telemetry, data, len) };
}

/// Install source-startup Refloat app-data state through the supplied lifecycle.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `state` and
/// `handler` must remain valid until firmware clears/replaces the handler and
/// stops the package.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) unsafe fn install_refloat_startup_app_data_with<B: AppDataBindings>(
    info: *mut ffi::LibInfo,
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    *state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());
    unsafe { lifecycle.install_with_state(info, state, handler) }.is_ok()
}

/// Allocate and install Refloat startup app-data state using firmware memory.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data(info: *mut ffi::LibInfo) -> bool {
    let alloc_bindings = vescpkg_rs::RealBindings;
    let allocator = vescpkg_rs::FirmwareAllocator::new(&alloc_bindings);
    let Ok(mut state) = allocator.allocate_for::<RefloatAppDataState>(1) else {
        return false;
    };
    let Some(state_ref) = (unsafe { state.as_mut_ptr().as_mut() }) else {
        return false;
    };

    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    if unsafe {
        install_refloat_startup_app_data_with(info, state_ref, &lifecycle, refloat_handle_app_data)
    } {
        let _ = state.into_raw();
        true
    } else {
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::null_mut();
            info.stop_fun = None;
        }
        false
    }
}

/// Refloat package app-data state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAppDataState {
    all_data_payloads: RefloatAllDataPayloads,
}

impl RefloatAppDataState {
    /// Build app-data state from the current all-data payload snapshot.
    pub const fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        Self { all_data_payloads }
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    /// Recover typed app-data state from VESC loader metadata.
    ///
    /// # Safety
    ///
    /// `info.arg` must either be null or contain a valid pointer to a live
    /// `RefloatAppDataState`.
    pub unsafe fn from_info_arg(info: &mut ffi::LibInfo) -> Option<&mut Self> {
        let ptr = core::ptr::NonNull::new(info.arg.cast::<Self>())?;
        Some(unsafe { ptr.as_ptr().as_mut()? })
    }

    /// Handle one app-data packet through the supplied lifecycle transport.
    pub fn handle_packet<B: AppDataBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }
        lifecycle.send_response(self.all_data_payloads, bytes)
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    pub fn handle_packet_with_telemetry<B: AppDataBindings, M: MotorTelemetryBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }

        let Ok(request) = RefloatAllDataRequest::parse(bytes) else {
            return false;
        };
        let fault = telemetry.firmware_fault();
        if !fault.is_none() {
            return lifecycle.send_response_bytes(&RefloatAllDataResponse::fault(
                RefloatFirmwareFaultCode::from_compat_code(fault.compat_code()),
            ));
        }
        let mode = request.mode();
        let payloads = self
            .all_data_payloads
            .with_base_battery_voltage(BatteryVoltage::new(
                telemetry.input_voltage_filtered().voltage(),
            ));
        let payloads = if mode.includes_mode2() {
            self.runtime_all_data_payloads(payloads, telemetry, mode.includes_mode3())
        } else {
            payloads
        };
        lifecycle.send_all_data_response(payloads, request)
    }

    fn handle_charging_state_packet(&mut self, bytes: &[u8]) -> bool {
        let [package_id, command_id, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != crate::domain::REFLOAT_APP_DATA_PACKAGE_ID.get()
            || RefloatAppDataCommand::try_from_id(*command_id)
                != Ok(RefloatAppDataCommand::ChargingState)
            || payload.len() < 6
            || payload[0] != 151
        {
            return false;
        }

        let (voltage, current) = if payload[1] > 0 {
            (
                refloat_read_scaled_i16([payload[2], payload[3]], 10.0),
                refloat_read_scaled_i16([payload[4], payload[5]], 10.0),
            )
        } else {
            (0.0, 0.0)
        };
        self.all_data_payloads =
            self.all_data_payloads
                .with_mode4_charging(RefloatAllDataMode4Payload::new(
                    RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(
                        current,
                    ))),
                    RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(
                        voltage,
                    ))),
                ));
        true
    }

    fn runtime_all_data_payloads<M: MotorTelemetryBindings>(
        self,
        payloads: RefloatAllDataPayloads,
        telemetry: &MotorTelemetryApi<M>,
        include_mode3: bool,
    ) -> RefloatAllDataPayloads {
        let payloads = payloads
            .with_mode2_distance_abs(telemetry.distance_abs())
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

fn refloat_read_scaled_i16(bytes: [u8; 2], scale: f32) -> f32 {
    i16::from_be_bytes(bytes) as f32 / scale
}

/// Refloat app-data lifecycle wiring.
pub struct RefloatAppDataLifecycle<B> {
    lifecycle: LoopbackLifecycle<B>,
}

impl<B: AppDataBindings> RefloatAppDataLifecycle<B> {
    /// Build Refloat app-data lifecycle wiring from firmware bindings.
    pub fn new(bindings: B) -> Self {
        Self {
            lifecycle: LoopbackLifecycle::new(bindings),
        }
    }

    /// Return the wrapped firmware bindings.
    pub fn bindings(&self) -> &B {
        self.lifecycle.bindings()
    }

    /// Install Refloat stop cleanup and app-data handler.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. The supplied
    /// handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install(
        &self,
        info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        unsafe {
            let _ = self.lifecycle.install(info, stop_refloat_app_data, handler);
        }
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Install Refloat state, stop cleanup, and app-data handler.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` and
    /// `handler` must remain valid until firmware clears/replaces the handler
    /// and stops the package.
    pub unsafe fn install_with_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::from_mut(state).cast();
        }
        unsafe { self.install(info, handler) }
    }

    /// Clear the Refloat app-data handler during package stop.
    pub fn stop(&self) -> Result<(), AppDataHandlerRegistrationError> {
        self.lifecycle.clear_app_data_handler()
    }

    /// Process one Refloat app-data packet and send a response when accepted.
    pub fn send_response(&self, payloads: RefloatAllDataPayloads, bytes: &[u8]) -> bool {
        let Some(response) = process_refloat_app_data(payloads, bytes) else {
            return false;
        };
        self.send_response_bytes(&response)
    }

    /// Encode and send one parsed Refloat all-data response.
    pub fn send_all_data_response(
        &self,
        payloads: RefloatAllDataPayloads,
        request: RefloatAllDataRequest,
    ) -> bool {
        self.send_response_bytes(&payloads.encode_response(request))
    }

    fn send_response_bytes(&self, response: &RefloatAllDataResponse) -> bool {
        let bytes = response.as_bytes();
        unsafe {
            self.lifecycle
                .send_app_data(bytes.as_ptr(), bytes.len() as u32)
        };
        true
    }
}

unsafe extern "C" fn stop_refloat_app_data(_arg: *mut core::ffi::c_void) {
    #[cfg(not(test))]
    {
        let _ = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings).stop();
    }
    #[cfg(all(not(test), target_arch = "arm"))]
    if let Some(ptr) = core::ptr::NonNull::new(_arg.cast::<RefloatAppDataState>()) {
        let bindings = vescpkg_rs::RealBindings;
        let _allocation =
            unsafe { vescpkg_rs::FirmwareAllocation::from_raw_parts(ptr, 1, &bindings) };
    }
}

#[cfg(test)]
mod tests {
    use super::{RefloatAppDataLifecycle, RefloatAppDataState};
    use super::{
        handle_refloat_app_data_packet, install_refloat_startup_app_data_with,
        process_refloat_app_data,
    };
    use crate::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature,
        RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
        RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataStatus,
        RefloatAppDataCommand, RefloatBeepReason, RefloatFocIdCurrent, RefloatMode,
        RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
        RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
        RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
        RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState,
        RefloatSetpointAdjustment, RefloatStopCondition,
    };
    use core::cell::Cell;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FakeMotorTelemetryBindings;
    use vescpkg_rs::{AppDataBindings, ffi};

    #[test]
    fn app_data_processes_all_data_requests_from_payload_snapshot() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        )
        .expect("all-data request should produce a response");

        assert_eq!(response.as_bytes().len(), 58);
        assert_eq!(&response.as_bytes()[..3], &[101, 10, 4]);
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::GetAllData.id(),
                ]
            ),
            None
        );
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::PrintInfo.id(),
                    4,
                ]
            ),
            None
        );
    }

    #[test]
    fn lifecycle_installs_refloat_app_data_handler_and_stop_cleanup() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(unsafe { lifecycle.install(&mut info, handler) }, Ok(()));
        assert!(info.stop_fun.is_some());
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            handler as *const () as usize
        );

        assert_eq!(lifecycle.stop(), Ok(()));
        assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
        assert_eq!(lifecycle.bindings().last_handler.get(), 0);
    }

    #[test]
    fn lifecycle_sends_refloat_app_data_responses_through_bindings() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());

        assert!(lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 4]);

        assert!(!lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::PrintInfo.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    }

    #[test]
    fn app_data_state_handles_packets_through_lifecycle_send_boundary() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet(
            &lifecycle,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(state.all_data_payloads(), sample_all_data_payloads());
    }

    #[test]
    fn app_data_state_refreshes_mode2_distance_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_distance_abs(
            TripDistance::new(Distance::from_meters(12.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_distance_bits.get(),
            12.5_f32.to_bits()
        );
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 1);
    }

    #[test]
    fn app_data_state_refreshes_mode2_temperatures_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_temperatures(
            MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_temperature_bytes.get(),
            [74, 97]
        );
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_mode3_ride_totals_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_ride_totals(
            OdometerMeters::from_meters(123_456),
            AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
            AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
            WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
            WattHoursCharged::new(Energy::from_watt_hours(18.5)),
            BatteryLevel::new(Ratio::from_ratio_const(0.72)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                3,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 54);
        assert_eq!(
            lifecycle.bindings().last_sent_mode3_ride_total_bytes.get(),
            [0, 1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144]
        );
        assert_eq!(telemetry.bindings().odometer_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 1);
    }

    #[test]
    fn app_data_state_sends_fault_response_before_refreshing_mode_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_firmware_fault(
            FirmwareFaultCode::from_compat_code(5),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 4);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 69]);
        assert_eq!(telemetry.bindings().firmware_fault_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_updates_mode4_charging_fields_from_charging_state_command() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::ChargingState.id(),
                151,
                1,
                1,
                244,
                0,
                123,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 0);

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(
            lifecycle.bindings().last_sent_mode4_charging_bytes.get(),
            [0, 123, 1, 244]
        );
    }

    #[test]
    fn app_data_state_does_not_refresh_distance_for_base_all_data() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_distance_abs(
            TripDistance::new(Distance::from_meters(12.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_base_battery_voltage_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_input_voltage_filtered(
                InputVoltage::new(Voltage::from_volts(84.2)),
            ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_base_motor_voltage_bytes
                .get(),
            [3, 74]
        );
        assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn lifecycle_installs_typed_refloat_state_for_handler_retrieval() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(
            unsafe { lifecycle.install_with_state(&mut info, &mut state, handler) },
            Ok(())
        );
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            sample_all_data_payloads()
        );
        let mut empty_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        assert!(unsafe { RefloatAppDataState::from_info_arg(&mut empty_info) }.is_none());
    }

    #[test]
    fn raw_handler_boundary_rejects_null_and_sends_valid_packets() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(!unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                core::ptr::null_mut(),
                0,
            )
        });

        let mut request = [101, 10, 0];
        assert!(unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                request.as_mut_ptr(),
                request.len() as u32,
            )
        });
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 0]);
    }

    #[test]
    fn startup_app_data_install_seeds_state_and_registers_handler() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe {
            install_refloat_startup_app_data_with(&mut info, &mut state, &lifecycle, handler)
        });
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            state.all_data_payloads(),
            RefloatAllDataPayloads::source_startup()
        );
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    struct RecordingAppDataBindings {
        handler_calls: Cell<usize>,
        last_handler: Cell<usize>,
        send_calls: Cell<usize>,
        last_sent_len: Cell<u32>,
        last_sent_prefix: Cell<[u8; 3]>,
        last_sent_base_motor_voltage_bytes: Cell<[u8; 2]>,
        last_sent_mode2_distance_bits: Cell<u32>,
        last_sent_mode2_temperature_bytes: Cell<[u8; 2]>,
        last_sent_mode3_ride_total_bytes: Cell<[u8; 13]>,
        last_sent_mode4_charging_bytes: Cell<[u8; 4]>,
        handler_results: Cell<[bool; 2]>,
    }

    impl RecordingAppDataBindings {
        fn accepting() -> Self {
            Self {
                handler_calls: Cell::new(0),
                last_handler: Cell::new(0),
                send_calls: Cell::new(0),
                last_sent_len: Cell::new(0),
                last_sent_prefix: Cell::new([0; 3]),
                last_sent_base_motor_voltage_bytes: Cell::new([0; 2]),
                last_sent_mode2_distance_bits: Cell::new(0),
                last_sent_mode2_temperature_bytes: Cell::new([0; 2]),
                last_sent_mode3_ride_total_bytes: Cell::new([0; 13]),
                last_sent_mode4_charging_bytes: Cell::new([0; 4]),
                handler_results: Cell::new([true, true]),
            }
        }
    }

    impl AppDataBindings for RecordingAppDataBindings {
        unsafe fn set_app_data_handler(&self, handler: ffi::AppDataHandler) -> bool {
            self.handler_calls.set(self.handler_calls.get() + 1);
            self.last_handler.set(handler as *const () as usize);
            let index = self.handler_calls.get().saturating_sub(1).min(1);
            self.handler_results.get()[index]
        }

        fn system_time_ticks(&self) -> u32 {
            0
        }

        unsafe fn send_app_data(&self, data: *const u8, len: u32) {
            self.send_calls.set(self.send_calls.get() + 1);
            self.last_sent_len.set(len);
            if len >= 3 {
                let bytes = unsafe { core::slice::from_raw_parts(data, len as usize) };
                self.last_sent_prefix.set([bytes[0], bytes[1], bytes[2]]);
                if bytes.len() >= 24 {
                    self.last_sent_base_motor_voltage_bytes
                        .set([bytes[22], bytes[23]]);
                }
                if bytes.len() >= 38 {
                    self.last_sent_mode2_distance_bits.set(u32::from_be_bytes([
                        bytes[34], bytes[35], bytes[36], bytes[37],
                    ]));
                }
                if bytes.len() >= 40 {
                    self.last_sent_mode2_temperature_bytes
                        .set([bytes[38], bytes[39]]);
                }
                if bytes.len() >= 54 {
                    self.last_sent_mode3_ride_total_bytes.set([
                        bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46],
                        bytes[47], bytes[48], bytes[49], bytes[50], bytes[51], bytes[52],
                        bytes[53],
                    ]);
                }
                if bytes.len() >= 58 {
                    self.last_sent_mode4_charging_bytes
                        .set([bytes[54], bytes[55], bytes[56], bytes[57]]);
                }
            }
        }
    }

    fn sample_all_data_payloads() -> RefloatAllDataPayloads {
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.60)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.40)),
            FootpadSensorState::Both,
        );
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
        );

        RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
                    ImuRoll::new(AngleRadians::from_radians(-0.5)),
                    ImuPitch::new(AngleRadians::from_radians(2.3)),
                ),
                RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
                footpad,
                setpoints,
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
                RefloatAllDataMotorPayload::new(
                    BatteryVoltage::new(Voltage::from_volts(72.0)),
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
                    MotorCurrent::new(Current::from_amps(5.0)),
                    BatteryCurrent::new(Current::from_amps(-2.0)),
                    DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
                    RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
                ),
            ),
            RefloatAllDataMode2Payload::new(
                TripDistance::new(Distance::from_meters(64.0)),
                RefloatRealtimeMotorTemperatures::new(
                    MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
                    MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
                ),
                RefloatAllDataBatteryTemperature::unavailable(),
            ),
            RefloatAllDataMode3Payload::new(
                OdometerMeters::from_meters(123_456),
                AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
                AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
                WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
                WattHoursCharged::new(Energy::from_watt_hours(18.5)),
                BatteryLevel::new(Ratio::from_ratio_const(0.72)),
            ),
            RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
                RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
            ),
        )
    }
}
