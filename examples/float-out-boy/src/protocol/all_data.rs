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
    /// Float Out Boy compact all-data attitude fields.
    pub struct FloatOutBoyAllDataAttitude {
        balance_pitch: FloatOutBoyRealtimeBalancePitch => balance_pitch,
        roll: ImuRoll => roll,
        pitch: ImuPitch => pitch,
    }
    /// Float Out Boy compact all-data status fields.
    #[derive(Eq)]
    pub struct FloatOutBoyAllDataStatus {
        ride_state: FloatOutBoyRideState => ride_state,
        beep_reason: FloatOutBoyBeepReason => beep_reason,
    }
    /// Float Out Boy compact all-data motor fields.
    pub struct FloatOutBoyAllDataMotorPayload {
        battery_voltage: BatteryVoltage => battery_voltage => with_battery_voltage,
        electrical_speed: ElectricalSpeed => electrical_speed,
        vehicle_speed: VehicleSpeed => vehicle_speed,
        currents: FloatOutBoyRealtimeMotorCurrents => currents,
        duty_cycle: DutyCycle => duty_cycle => with_duty_cycle,
        foc_id_current: Option<MotorCurrent> => foc_id_current,
    }
    /// Float Out Boy compact all-data base payload fields.
    pub struct FloatOutBoyAllDataBasePayload {
        balance_current: FloatOutBoyRealtimeBalanceCurrent => balance_current => with_balance_current,
        attitude: FloatOutBoyAllDataAttitude => attitude => with_attitude,
        status: FloatOutBoyAllDataStatus => status => with_status,
        footpad: FloatOutBoyFootpadSample => footpad => with_footpad,
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints => setpoints => with_setpoints,
        booster_torque: FloatOutBoyRealtimeBoosterTorque => booster_torque => with_booster_torque,
        motor: FloatOutBoyAllDataMotorPayload => motor => with_motor,
    }
    /// Float Out Boy all-data payload snapshot used to answer compact all-data requests.
    pub struct FloatOutBoyAllDataPayloads {
        base: FloatOutBoyAllDataBasePayload => base => with_base,
        mode2: FloatOutBoyAllDataMode2Payload => mode2,
        mode3: FloatOutBoyAllDataMode3Payload => mode3 => with_mode3_ride_totals,
        mode4: FloatOutBoyAllDataMode4Payload => mode4 => with_mode4_charging,
    }
    /// Float Out Boy all-data mode 2 extension fields.
    pub struct FloatOutBoyAllDataMode2Payload {
        distance_abs: TripDistance => distance_abs => with_distance_abs,
        temperatures: FloatOutBoyRealtimeMotorTemperatures => temperatures => with_temperatures,
        battery_temperature: Option<Temperature> => battery_temperature,
    }
    /// Float Out Boy all-data mode 3 extension fields.
    pub struct FloatOutBoyAllDataMode3Payload {
        odometer: OdometerMeters => odometer,
        discharged_charge: AmpHoursDischarged => discharged_charge,
        charged_charge: AmpHoursCharged => charged_charge,
        discharged_energy: WattHoursDischarged => discharged_energy,
        charged_energy: WattHoursCharged => charged_energy,
        battery_level: BatteryLevel => battery_level,
    }
    /// Float Out Boy all-data mode 4 extension fields.
    pub struct FloatOutBoyAllDataMode4Payload {
        current: BatteryCurrent => current,
        voltage: BatteryVoltage => voltage,
    }
}

#[cfg(test)]
#[path = "all_data/tests.rs"]
mod tests;

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

impl FloatOutBoyAllDataBasePayload {
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
        packet.push_scaled_i16(self.balance_current.current().current().as_amps(), 10.0);
        packet.push_scaled_i16(
            float_out_boy_degrees(self.attitude.balance_pitch().angle()),
            10.0,
        );
        packet.push_scaled_i16(float_out_boy_degrees(self.attitude.roll().angle()), 10.0);

        let ride_state = self.status.ride_state;
        packet.push(
            (ride_state.float_state_compat() & 0x0f)
                | (ride_state.setpoint_adjustment_compat() << 4),
        );

        let handtest = matches!(ride_state.mode(), FloatOutBoyMode::HandTest);
        let switch_state = self.footpad.state().switch_compat() | u8::from(handtest) << 3;
        packet.push((switch_state & 0x0f) | (self.status.beep_reason.id() << 4));
        packet.push(float_out_boy_scaled_u8(
            self.footpad.left_voltage().as_volts(),
            50.0,
        ));
        packet.push(float_out_boy_scaled_u8(
            self.footpad.right_voltage().as_volts(),
            50.0,
        ));

        for setpoint in [
            self.setpoints.board(),
            self.setpoints.atr(),
            self.setpoints.brake_tilt(),
            self.setpoints.torque_tilt(),
            self.setpoints.turn_tilt(),
            self.setpoints.remote(),
        ] {
            let value = float_out_boy_offset_scaled_u8(setpoint.angle().as_degrees(), 5.0, 128.0);
            packet.push(value);
        }

        packet.push_scaled_i16(float_out_boy_degrees(self.attitude.pitch().angle()), 10.0);
        packet.push(float_out_boy_offset_scaled_u8(
            self.booster_torque.torque().as_newton_meters(),
            1.0,
            128.0,
        ));
        self.encode_motor_response(&mut packet);

        packet.into_bytes()
    }

    fn encode_motor_response<const N: usize>(&self, packet: &mut FloatOutBoyPacket<N>) {
        packet.push_scaled_i16(self.motor.battery_voltage().voltage().as_volts(), 10.0);
        packet.push_i16(super::packet::saturating_trunc_f32_to_i16(
            self.motor
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute(),
        ));
        packet.push_scaled_i16(
            self.motor.vehicle_speed().speed().as_meters_per_second(),
            10.0,
        );
        packet.push_scaled_i16(self.motor.motor_current().current().as_amps(), 10.0);
        packet.push_scaled_i16(self.motor.battery_current().current().as_amps(), 10.0);
        packet.push(float_out_boy_offset_scaled_u8(
            self.motor.duty_cycle().ratio().as_ratio(),
            100.0,
            128.0,
        ));
        packet.push(self.motor.foc_id_current().map_or(222, |current| {
            float_out_boy_scaled_u8(current.current().as_amps().abs(), 3.0)
        }));
    }

    /// Return base all-data fields with refreshed motor battery voltage.
    #[must_use]
    pub const fn with_motor_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            motor: self.motor.with_battery_voltage(battery_voltage),
            ..self
        }
    }
}

impl FloatOutBoyAllDataPayloads {
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
        response.extend(&self.base.encode_base_response(mode.source_id()));
        if mode.includes_mode2() {
            float_out_boy_append_all_data_mode2(&mut response, self.mode2);
        }
        if mode.includes_mode3() {
            float_out_boy_append_all_data_mode3(&mut response, self.mode3);
        }
        if mode.includes_mode4() {
            float_out_boy_append_all_data_mode4(&mut response, self.mode4);
        }
        response
    }

    /// Return a payload snapshot with refreshed base battery voltage.
    #[must_use]
    pub const fn with_base_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            base: self.base.with_motor_battery_voltage(battery_voltage),
            ..self
        }
    }

    /// Return a payload snapshot with refreshed absolute-distance mode 2 data.
    #[must_use]
    pub const fn with_mode2_distance_abs(self, distance_abs: TripDistance) -> Self {
        Self {
            mode2: self.mode2.with_distance_abs(distance_abs),
            ..self
        }
    }

    /// Return a payload snapshot with refreshed mode 2 motor temperatures.
    #[must_use]
    pub const fn with_mode2_temperatures(
        self,
        temperatures: FloatOutBoyRealtimeMotorTemperatures,
    ) -> Self {
        Self {
            mode2: self.mode2.with_temperatures(temperatures),
            ..self
        }
    }
}
