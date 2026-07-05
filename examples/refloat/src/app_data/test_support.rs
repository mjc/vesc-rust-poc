use super::*;
use crate::domain::{
    FootpadSensorSample, RefloatAllDataBatteryTemperature, RefloatAllDataMode2Payload,
    RefloatBeepReason,
};
use core::cell::Cell;
use core::ffi::c_void;
use vescpkg_rs::prelude::*;
use vescpkg_rs::{
    AllocBindings, AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings, ffi,
};

pub(super) struct RecordingAppDataBindings {
    pub(super) handler_calls: Cell<usize>,
    pub(super) last_handler: Cell<usize>,
    pub(super) send_calls: Cell<usize>,
    pub(super) last_sent_len: Cell<u32>,
    pub(super) last_sent_prefix: Cell<[u8; 3]>,
    pub(super) last_sent_base_foc_id_byte: Cell<u8>,
    pub(super) last_sent_base_motor_voltage_bytes: Cell<[u8; 2]>,
    pub(super) last_sent_realtime_timestamp_bytes: Cell<[u8; 4]>,
    pub(super) last_sent_realtime_voltage_bytes: Cell<[u8; 2]>,
    pub(super) last_sent_realtime_temperature_bytes: Cell<[u8; 4]>,
    pub(super) last_sent_mode2_distance_bits: Cell<u32>,
    pub(super) last_sent_mode2_temperature_bytes: Cell<[u8; 2]>,
    pub(super) last_sent_mode3_ride_total_bytes: Cell<[u8; 13]>,
    pub(super) last_sent_mode4_charging_bytes: Cell<[u8; 4]>,
    pub(super) custom_config_register_calls: Cell<usize>,
    pub(super) custom_config_clear_calls: Cell<usize>,
    pub(super) imu_read_callback_calls: Cell<usize>,
    pub(super) last_imu_read_callback: Cell<usize>,
    pub(super) system_time_ticks: Cell<u32>,
    pub(super) handler_results: Cell<[bool; 2]>,
}

pub(super) struct RecordingAllocBindings {
    pub(super) malloc_calls: Cell<usize>,
    pub(super) free_calls: Cell<usize>,
    pub(super) next_ptr: Cell<*mut c_void>,
    pub(super) last_requested_len: Cell<usize>,
}

impl RecordingAllocBindings {
    pub(super) fn new(next_ptr: *mut c_void) -> Self {
        Self {
            malloc_calls: Cell::new(0),
            free_calls: Cell::new(0),
            next_ptr: Cell::new(next_ptr),
            last_requested_len: Cell::new(0),
        }
    }
}

impl AllocBindings for RecordingAllocBindings {
    unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
        self.malloc_calls.set(self.malloc_calls.get() + 1);
        self.last_requested_len.set(bytes);
        self.next_ptr.get()
    }

    unsafe fn free(&self, _ptr: *mut c_void) {
        self.free_calls.set(self.free_calls.get() + 1);
    }
}

impl RecordingAppDataBindings {
    pub(super) fn accepting() -> Self {
        Self {
            handler_calls: Cell::new(0),
            last_handler: Cell::new(0),
            send_calls: Cell::new(0),
            last_sent_len: Cell::new(0),
            last_sent_prefix: Cell::new([0; 3]),
            last_sent_base_foc_id_byte: Cell::new(0),
            last_sent_base_motor_voltage_bytes: Cell::new([0; 2]),
            last_sent_realtime_timestamp_bytes: Cell::new([0; 4]),
            last_sent_realtime_voltage_bytes: Cell::new([0; 2]),
            last_sent_realtime_temperature_bytes: Cell::new([0; 4]),
            last_sent_mode2_distance_bits: Cell::new(0),
            last_sent_mode2_temperature_bytes: Cell::new([0; 2]),
            last_sent_mode3_ride_total_bytes: Cell::new([0; 13]),
            last_sent_mode4_charging_bytes: Cell::new([0; 4]),
            custom_config_register_calls: Cell::new(0),
            custom_config_clear_calls: Cell::new(0),
            imu_read_callback_calls: Cell::new(0),
            last_imu_read_callback: Cell::new(0),
            system_time_ticks: Cell::new(0),
            handler_results: Cell::new([true, true]),
        }
    }

    pub(super) fn with_system_time_ticks(self, ticks: u32) -> Self {
        self.system_time_ticks.set(ticks);
        self
    }
}

impl AppDataBindings for RecordingAppDataBindings {
    unsafe fn set_app_data_handler(&self, handler: ffi::AppDataHandler) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(handler as *const () as usize);
        let index = self.handler_calls.get().saturating_sub(1).min(1);
        self.handler_results.get()[index]
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(0);
        let index = self.handler_calls.get().saturating_sub(1).min(1);
        self.handler_results.get()[index]
    }

    fn system_time_ticks(&self) -> u32 {
        self.system_time_ticks.get()
    }

    fn app_data_arg(&self, _prog_addr: u32) -> Option<core::ptr::NonNull<core::ffi::c_void>> {
        None
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
            if bytes.len() >= 34 {
                self.last_sent_base_foc_id_byte.set(bytes[33]);
            }
            if bytes.len() >= 32 && bytes[1] == RefloatAppDataCommand::RealtimeData.id() {
                self.last_sent_realtime_timestamp_bytes
                    .set([bytes[4], bytes[5], bytes[6], bytes[7]]);
                self.last_sent_realtime_voltage_bytes
                    .set([bytes[24], bytes[25]]);
                self.last_sent_realtime_temperature_bytes
                    .set([bytes[28], bytes[29], bytes[30], bytes[31]]);
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
                    bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46], bytes[47],
                    bytes[48], bytes[49], bytes[50], bytes[51], bytes[52], bytes[53],
                ]);
            }
            if bytes.len() >= 58 {
                self.last_sent_mode4_charging_bytes
                    .set([bytes[54], bytes[55], bytes[56], bytes[57]]);
            }
        }
    }
}

impl CustomConfigBindings for RecordingAppDataBindings {
    unsafe fn register_custom_config(
        &self,
        _get_cfg: ffi::raw::CustomConfigGet,
        _set_cfg: ffi::raw::CustomConfigSet,
        _get_cfg_xml: ffi::raw::CustomConfigXml,
    ) -> bool {
        // Refloat v1.2.1 registers custom config during init at `third_party/refloat/src/main.c:2456`.
        self.custom_config_register_calls
            .set(self.custom_config_register_calls.get() + 1);
        true
    }

    unsafe fn clear_custom_configs(&self) -> bool {
        // Refloat v1.2.1 clears custom config during stop at `third_party/refloat/src/main.c:2403`.
        self.custom_config_clear_calls
            .set(self.custom_config_clear_calls.get() + 1);
        true
    }
}

impl ImuReadCallbackBindings for RecordingAppDataBindings {
    unsafe fn set_imu_read_callback(&self, callback: ffi::raw::ImuReadCallback) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback
            .set(callback as *const () as usize);
    }

    unsafe fn clear_imu_read_callback(&self) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback.set(0);
    }
}

pub(super) fn sample_all_data_payloads() -> RefloatAllDataPayloads {
    sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal)
}

pub(super) fn sample_all_data_payloads_with_ride_state(
    run_state: RefloatRunState,
    mode: RefloatMode,
) -> RefloatAllDataPayloads {
    let ride_state = RefloatRideState::new(
        run_state,
        mode,
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
