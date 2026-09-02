//! Compact Float Out Boy all-data response types.
//!
//! C map: `cmd_send_all_data` encodes these response variants at
//! `third_party/float-out-boy/src/main.c:1313-1399`.

use super::all_data_wire::{
    float_out_boy_append_all_data_mode2, float_out_boy_append_all_data_mode3,
    float_out_boy_append_all_data_mode4, float_out_boy_degrees, float_out_boy_offset_scaled_u8,
    float_out_boy_scaled_u8,
};
use super::packet::FloatOutBoyPacket;
use super::realtime::{
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterTorque, FloatOutBoyRealtimeFilteredMotorCurrent,
    FloatOutBoyRealtimeMotorCurrents, FloatOutBoyRealtimeMotorTemperatures,
    FloatOutBoyRealtimeRuntimeSetpoints,
};
use super::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataRequest, FloatOutBoyAppDataCommand,
    FloatOutBoyFootpadSample, FloatOutBoyRideState,
};
use super::{FloatOutBoyBeepReason, FloatOutBoyMode};
use vescpkg_rs::prelude::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, BatteryVoltage,
    DirectionalMotorCurrent, DutyCycle, ElectricalSpeed, FirmwareFaultWireCode, ImuPitch, ImuRoll,
    MotorCurrent, OdometerMeters, Temperature, TripDistance, VehicleSpeed, WattHoursCharged,
    WattHoursDischarged,
};

/// Maximum-capacity Float Out Boy all-data response bytes.
pub type FloatOutBoyAllDataResponse = FloatOutBoyPacket<58>;

/// Encode a Float Out Boy all-data fault response.
#[must_use]
pub fn encode_float_out_boy_all_data_fault_response(
    fault: FirmwareFaultWireCode,
) -> FloatOutBoyAllDataResponse {
    let mut response = FloatOutBoyAllDataResponse::new();
    response.extend(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::GetAllData.id(),
        69,
        fault.wire_code(),
    ]);
    response
}

vescpkg_rs::typed_field_groups! {
    attributes { #[derive(Debug, Default, Clone, Copy, PartialEq)] }
    /// One flat, typed telemetry snapshot shared by control and wire encoders.
    pub struct FloatOutBoyAllDataPayloads {
        balance_current: FloatOutBoyRealtimeBalanceCurrent => balance_current => with_balance_current,
        balance_pitch: FloatOutBoyRealtimeBalancePitch => balance_pitch => with_balance_pitch,
        roll: ImuRoll => roll => with_roll,
        pitch: ImuPitch => pitch => with_pitch,
        ride_state: FloatOutBoyRideState => ride_state => with_ride_state,
        beep_reason: FloatOutBoyBeepReason => beep_reason => with_beep_reason,
        footpad: FloatOutBoyFootpadSample => footpad => with_footpad,
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints => setpoints => with_setpoints,
        booster_torque: FloatOutBoyRealtimeBoosterTorque => booster_torque => with_booster_torque,
        motor_battery_voltage: BatteryVoltage => motor_battery_voltage => with_motor_battery_voltage,
        electrical_speed: ElectricalSpeed => electrical_speed => with_electrical_speed,
        vehicle_speed: VehicleSpeed => vehicle_speed => with_vehicle_speed,
        currents: FloatOutBoyRealtimeMotorCurrents => currents => with_currents,
        duty_cycle: DutyCycle => duty_cycle => with_duty_cycle,
        foc_id_current: Option<MotorCurrent> => foc_id_current => with_foc_id_current,
        distance_abs: TripDistance => distance_abs => with_distance_abs,
        temperatures: FloatOutBoyRealtimeMotorTemperatures => temperatures => with_temperatures,
        battery_temperature: Option<Temperature> => battery_temperature => with_battery_temperature,
        odometer: OdometerMeters => odometer => with_odometer,
        discharged_charge: AmpHoursDischarged => discharged_charge => with_discharged_charge,
        charged_charge: AmpHoursCharged => charged_charge => with_charged_charge,
        discharged_energy: WattHoursDischarged => discharged_energy => with_discharged_energy,
        charged_energy: WattHoursCharged => charged_energy => with_charged_energy,
        battery_level: BatteryLevel => battery_level => with_battery_level,
        charging_current: BatteryCurrent => charging_current => with_charging_current,
        charging_voltage: BatteryVoltage => charging_voltage => with_charging_voltage,
    }
}

#[cfg(any(test, feature = "test-support"))]
vescpkg_rs::typed_field_groups! {
    attributes { #[derive(Debug, Default, Clone, Copy, PartialEq)] }
    /// Test-fixture compatibility group for attitude fields.
    pub struct FloatOutBoyAllDataAttitude {
        balance_pitch: FloatOutBoyRealtimeBalancePitch => balance_pitch,
        roll: ImuRoll => roll,
        pitch: ImuPitch => pitch,
    }
    /// Test-fixture compatibility group for status fields.
    #[derive(Eq)]
    pub struct FloatOutBoyAllDataStatus {
        ride_state: FloatOutBoyRideState => ride_state,
        beep_reason: FloatOutBoyBeepReason => beep_reason,
    }
    /// Test-fixture compatibility group for motor fields.
    pub struct FloatOutBoyAllDataMotorPayload {
        battery_voltage: BatteryVoltage => battery_voltage => with_battery_voltage,
        electrical_speed: ElectricalSpeed => electrical_speed,
        vehicle_speed: VehicleSpeed => vehicle_speed,
        currents: FloatOutBoyRealtimeMotorCurrents => currents,
        duty_cycle: DutyCycle => duty_cycle => with_duty_cycle,
        foc_id_current: Option<MotorCurrent> => foc_id_current,
    }
    /// Test-fixture compatibility group for base fields.
    pub struct FloatOutBoyAllDataBasePayload {
        balance_current: FloatOutBoyRealtimeBalanceCurrent => balance_current => with_balance_current,
        attitude: FloatOutBoyAllDataAttitude => attitude => with_attitude,
        status: FloatOutBoyAllDataStatus => status => with_status,
        footpad: FloatOutBoyFootpadSample => footpad => with_footpad,
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints => setpoints => with_setpoints,
        booster_torque: FloatOutBoyRealtimeBoosterTorque => booster_torque => with_booster_torque,
        motor: FloatOutBoyAllDataMotorPayload => motor => with_motor,
    }
    /// Test-fixture compatibility group for mode 2 fields.
    pub struct FloatOutBoyAllDataMode2Payload {
        distance_abs: TripDistance => distance_abs => with_distance_abs,
        temperatures: FloatOutBoyRealtimeMotorTemperatures => temperatures => with_temperatures,
        battery_temperature: Option<Temperature> => battery_temperature,
    }
    /// Test-fixture compatibility group for mode 3 fields.
    pub struct FloatOutBoyAllDataMode3Payload {
        odometer: OdometerMeters => odometer,
        discharged_charge: AmpHoursDischarged => discharged_charge,
        charged_charge: AmpHoursCharged => charged_charge,
        discharged_energy: WattHoursDischarged => discharged_energy,
        charged_energy: WattHoursCharged => charged_energy,
        battery_level: BatteryLevel => battery_level,
    }
    /// Test-fixture compatibility group for mode 4 fields.
    pub struct FloatOutBoyAllDataMode4Payload {
        current: BatteryCurrent => current,
        voltage: BatteryVoltage => voltage,
    }
}

#[cfg(test)]
#[path = "all_data/tests.rs"]
mod tests;

#[cfg(any(test, feature = "test-support"))]
impl FloatOutBoyAllDataMotorPayload {
    vescpkg_rs::const_forward_getters! {
        /// Return motor current.
        pub fn motor_current -> MotorCurrent = currents.motor();
        /// Return directional motor current.
        pub fn directional_motor_current -> DirectionalMotorCurrent = currents.directional();
        /// Return Float Out Boy's filtered directional motor current.
        pub fn filtered_motor_current -> FloatOutBoyRealtimeFilteredMotorCurrent = currents.filtered();
        /// Return battery current.
        pub fn battery_current -> BatteryCurrent = currents.battery();
    }
}

#[cfg(any(test, feature = "test-support"))]
impl FloatOutBoyAllDataBasePayload {
    /// Encode this historical grouped fixture through the flat snapshot.
    #[must_use]
    pub fn encode_base_response(&self, mode: u8) -> [u8; 34] {
        FloatOutBoyAllDataPayloads::from_groups(
            *self,
            FloatOutBoyAllDataMode2Payload::default(),
            FloatOutBoyAllDataMode3Payload::default(),
            FloatOutBoyAllDataMode4Payload::default(),
        )
        .encode_base_response(mode)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl FloatOutBoyAllDataPayloads {
    /// Build a flat snapshot from the historical grouped test fixtures.
    #[must_use]
    pub const fn from_groups(
        base: FloatOutBoyAllDataBasePayload,
        mode2: FloatOutBoyAllDataMode2Payload,
        mode3: FloatOutBoyAllDataMode3Payload,
        mode4: FloatOutBoyAllDataMode4Payload,
    ) -> Self {
        let attitude = base.attitude();
        let status = base.status();
        let motor = base.motor();
        Self::new(
            base.balance_current(),
            attitude.balance_pitch(),
            attitude.roll(),
            attitude.pitch(),
            status.ride_state(),
            status.beep_reason(),
            base.footpad(),
            base.setpoints(),
            base.booster_torque(),
            motor.battery_voltage(),
            motor.electrical_speed(),
            motor.vehicle_speed(),
            motor.currents(),
            motor.duty_cycle(),
            motor.foc_id_current(),
            mode2.distance_abs(),
            mode2.temperatures(),
            mode2.battery_temperature(),
            mode3.odometer(),
            mode3.discharged_charge(),
            mode3.charged_charge(),
            mode3.discharged_energy(),
            mode3.charged_energy(),
            mode3.battery_level(),
            mode4.current(),
            mode4.voltage(),
        )
    }

    /// Rebuild the historical base group for existing fixtures.
    #[must_use]
    pub const fn base(self) -> FloatOutBoyAllDataBasePayload {
        FloatOutBoyAllDataBasePayload::new(
            self.balance_current(),
            FloatOutBoyAllDataAttitude::new(self.balance_pitch(), self.roll(), self.pitch()),
            FloatOutBoyAllDataStatus::new(self.ride_state(), self.beep_reason()),
            self.footpad(),
            self.setpoints(),
            self.booster_torque(),
            FloatOutBoyAllDataMotorPayload::new(
                self.motor_battery_voltage(),
                self.electrical_speed(),
                self.vehicle_speed(),
                self.currents(),
                self.duty_cycle(),
                self.foc_id_current(),
            ),
        )
    }

    /// Rebuild the historical mode 2 group for existing fixtures.
    #[must_use]
    pub const fn mode2(self) -> FloatOutBoyAllDataMode2Payload {
        FloatOutBoyAllDataMode2Payload::new(
            self.distance_abs(),
            self.temperatures(),
            self.battery_temperature(),
        )
    }

    /// Rebuild the historical mode 3 group for existing fixtures.
    #[must_use]
    pub const fn mode3(self) -> FloatOutBoyAllDataMode3Payload {
        FloatOutBoyAllDataMode3Payload::new(
            self.odometer(),
            self.discharged_charge(),
            self.charged_charge(),
            self.discharged_energy(),
            self.charged_energy(),
            self.battery_level(),
        )
    }

    /// Rebuild the historical mode 4 group for existing fixtures.
    #[must_use]
    pub const fn mode4(self) -> FloatOutBoyAllDataMode4Payload {
        FloatOutBoyAllDataMode4Payload::new(self.charging_current(), self.charging_voltage())
    }
}

impl FloatOutBoyAllDataPayloads {
    vescpkg_rs::const_forward_getters! {
        /// Return motor current.
        pub fn motor_current -> MotorCurrent = currents.motor();
        /// Return directional motor current.
        pub fn directional_motor_current -> DirectionalMotorCurrent = currents.directional();
        /// Return Float Out Boy's filtered directional motor current.
        pub fn filtered_motor_current -> FloatOutBoyRealtimeFilteredMotorCurrent = currents.filtered();
        /// Return battery current.
        pub fn battery_current -> BatteryCurrent = currents.battery();
    }

    /// Encode the compact all-data base response bytes.
    ///
    /// C map: `cmd_all_data` writes degree-valued IMU fields with scale 10 at
    /// `third_party/float-out-boy/src/main.c:1328-1365`; Rust stores the source IMU
    /// readings as typed radians and converts at this wire boundary.
    #[must_use]
    pub fn encode_base_response(&self, mode: u8) -> [u8; 34] {
        let mut packet = FloatOutBoyPacket::new();

        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
        packet.push(FloatOutBoyAppDataCommand::GetAllData.id());
        packet.push(mode);
        packet.push_scaled_i16(self.balance_current().current().current().as_amps(), 10.0);
        packet.push_scaled_i16(float_out_boy_degrees(self.balance_pitch().angle()), 10.0);
        packet.push_scaled_i16(float_out_boy_degrees(self.roll().angle()), 10.0);

        let ride_state = self.ride_state();
        packet.push(
            (ride_state.float_state_compat() & 0x0f)
                | (ride_state.setpoint_adjustment_compat() << 4),
        );

        let handtest = matches!(ride_state.mode(), FloatOutBoyMode::HandTest);
        let switch_state = self.footpad().state().switch_compat() | u8::from(handtest) << 3;
        packet.push((switch_state & 0x0f) | (self.beep_reason().id() << 4));
        packet.push(float_out_boy_scaled_u8(
            self.footpad().left_voltage().as_volts(),
            50.0,
        ));
        packet.push(float_out_boy_scaled_u8(
            self.footpad().right_voltage().as_volts(),
            50.0,
        ));

        for setpoint in [
            self.setpoints().board(),
            self.setpoints().atr(),
            self.setpoints().brake_tilt(),
            self.setpoints().torque_tilt(),
            self.setpoints().turn_tilt(),
            self.setpoints().remote(),
        ] {
            let value = float_out_boy_offset_scaled_u8(setpoint.angle().as_degrees(), 5.0, 128.0);
            packet.push(value);
        }

        packet.push_scaled_i16(float_out_boy_degrees(self.pitch().angle()), 10.0);
        packet.push(float_out_boy_offset_scaled_u8(
            self.booster_torque().torque().as_newton_meters(),
            1.0,
            128.0,
        ));
        self.encode_motor_response(&mut packet);

        packet.into_bytes()
    }

    fn encode_motor_response<const N: usize>(&self, packet: &mut FloatOutBoyPacket<N>) {
        packet.push_scaled_i16(self.motor_battery_voltage().voltage().as_volts(), 10.0);
        packet.push_i16(super::packet::saturating_trunc_f32_to_i16(
            self.electrical_speed().rpm().as_revolutions_per_minute(),
        ));
        packet.push_scaled_i16(self.vehicle_speed().speed().as_meters_per_second(), 10.0);
        packet.push_scaled_i16(self.motor_current().current().as_amps(), 10.0);
        packet.push_scaled_i16(self.battery_current().current().as_amps(), 10.0);
        packet.push(float_out_boy_offset_scaled_u8(
            self.duty_cycle().ratio().as_ratio(),
            100.0,
            128.0,
        ));
        packet.push(self.foc_id_current().map_or(222, |current| {
            float_out_boy_scaled_u8(current.current().as_amps().abs(), 3.0)
        }));
    }

    /// Build the default startup snapshot for host-side fixtures.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn source_startup() -> Self {
        Self::default()
    }

    /// Encode the source-compatible response for a parsed all-data request.
    ///
    /// The byte order and mode gates mirror `cmd_send_all_data` in upstream
    /// `third_party/float-out-boy/src/main.c:1313-1399`.
    #[inline(never)]
    #[must_use]
    pub fn encode_response(
        &self,
        request: FloatOutBoyAllDataRequest,
    ) -> FloatOutBoyAllDataResponse {
        let mode = request.mode();
        let mut response = FloatOutBoyAllDataResponse::new();
        response.extend(&self.encode_base_response(mode.source_id()));
        if mode.includes_mode2() {
            float_out_boy_append_all_data_mode2(&mut response, self);
        }
        if mode.includes_mode3() {
            float_out_boy_append_all_data_mode3(&mut response, self);
        }
        if mode.includes_mode4() {
            float_out_boy_append_all_data_mode4(&mut response, self);
        }
        response
    }
}
