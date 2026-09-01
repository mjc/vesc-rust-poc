//! Float Out Boy compact all-data response types.
//!
//! C map: `cmd_send_all_data` encodes these response variants at
//! `third_party/float-out-boy/src/main.c:1313-1399`.

use super::realtime::{
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeFilteredMotorCurrent,
    FloatOutBoyRealtimeMotorCurrents, FloatOutBoyRealtimeMotorTemperatures,
    FloatOutBoyRealtimeRuntimeSetpoints,
};
use super::state::{FloatOutBoyBeepReason, FloatOutBoyMode};
use super::wire::{
    float_out_boy_append_all_data_mode2, float_out_boy_append_all_data_mode3,
    float_out_boy_append_all_data_mode4, float_out_boy_degrees, float_out_boy_offset_scaled_u8,
    float_out_boy_scaled_u8,
};
use super::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAllDataRequest,
    FloatOutBoyAppDataCommand, FloatOutBoyFootpadSample, FloatOutBoyRideState,
};
use crate::wire::FloatOutBoyPacket;
use vescpkg_rs::prelude::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, BatteryVoltage,
    DirectionalMotorCurrent, DutyCycle, ElectricalSpeed, FirmwareFaultWireCode, ImuPitch, ImuRoll,
    MotorCurrent, OdometerMeters, Temperature, TripDistance, VehicleSpeed, WattHoursCharged,
    WattHoursDischarged,
};

/// Fixed-size Float Out Boy all-data response bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyAllDataResponse {
    /// Fault response bytes.
    Fault([u8; 4]),
    /// Base response bytes.
    Base([u8; 34]),
    /// Mode 2 response bytes.
    Mode2([u8; 41]),
    /// Mode 3 response bytes.
    Mode3([u8; 54]),
    /// Mode 4 response bytes.
    Mode4([u8; 58]),
}

impl FloatOutBoyAllDataResponse {
    /// Encode a Float Out Boy all-data fault response.
    #[must_use]
    pub const fn fault(fault: FirmwareFaultWireCode) -> Self {
        Self::Fault([
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::GetAllData.id(),
            69,
            fault.wire_code(),
        ])
    }

    /// Return the encoded response bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Fault(bytes) => bytes,
            Self::Base(bytes) => bytes,
            Self::Mode2(bytes) => bytes,
            Self::Mode3(bytes) => bytes,
            Self::Mode4(bytes) => bytes,
        }
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy compact all-data attitude fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataAttitude {
        balance_pitch: FloatOutBoyRealtimeBalancePitch => balance_pitch,
        roll: ImuRoll => roll,
        pitch: ImuPitch => pitch,
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy compact all-data status fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct FloatOutBoyAllDataStatus {
        ride_state: FloatOutBoyRideState => ride_state,
        beep_reason: FloatOutBoyBeepReason => beep_reason,
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy compact all-data motor fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataMotorPayload {
        battery_voltage: BatteryVoltage => battery_voltage => with_battery_voltage,
        electrical_speed: ElectricalSpeed => electrical_speed,
        vehicle_speed: VehicleSpeed => vehicle_speed,
        currents: FloatOutBoyRealtimeMotorCurrents => currents,
        duty_cycle: DutyCycle => duty_cycle => with_duty_cycle,
        foc_id_current: Option<MotorCurrent> => foc_id_current,
    }
}

impl FloatOutBoyAllDataMotorPayload {
    /// Return motor current.
    #[must_use]
    pub const fn motor_current(self) -> MotorCurrent {
        self.currents.motor()
    }

    /// Return directional motor current.
    #[must_use]
    pub const fn directional_motor_current(self) -> DirectionalMotorCurrent {
        self.currents.directional()
    }

    /// Return Float Out Boy's filtered directional motor current.
    #[must_use]
    pub const fn filtered_motor_current(self) -> FloatOutBoyRealtimeFilteredMotorCurrent {
        self.currents.filtered()
    }

    /// Return battery current.
    #[must_use]
    pub const fn battery_current(self) -> BatteryCurrent {
        self.currents.battery()
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy compact all-data base payload fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataBasePayload {
        balance_current: FloatOutBoyRealtimeBalanceCurrent => balance_current,
        attitude: FloatOutBoyAllDataAttitude => attitude,
        status: FloatOutBoyAllDataStatus => status => with_status,
        footpad: FloatOutBoyFootpadSample => footpad => with_footpad,
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints => setpoints,
        booster_current: FloatOutBoyRealtimeBoosterCurrent => booster_current,
        motor: FloatOutBoyAllDataMotorPayload => motor => with_motor,
    }
}

impl FloatOutBoyAllDataBasePayload {
    /// Return the Float Out Boy app-data command this payload belongs to.
    #[must_use]
    pub const fn command(self) -> FloatOutBoyAppDataCommand {
        FloatOutBoyAppDataCommand::GetAllData
    }

    /// Encode the compact all-data base response bytes.
    ///
    /// C map: `cmd_all_data` writes degree-valued IMU fields with scale 10 at
    /// `third_party/float-out-boy/src/main.c:1328-1365`; Rust stores the source IMU
    /// readings as typed radians and converts at this wire boundary.
    #[must_use]
    pub fn encode_base_response(&self, mode: u8) -> [u8; 34] {
        let mut packet = FloatOutBoyPacket::new();

        packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
        packet.push(self.command().id());
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
            self.booster_current.current().current().as_amps(),
            1.0,
            128.0,
        ));
        self.encode_motor_response(&mut packet);

        packet.into_bytes()
    }

    fn encode_motor_response<const N: usize>(&self, packet: &mut FloatOutBoyPacket<N>) {
        packet.push_scaled_i16(self.motor.battery_voltage().voltage().as_volts(), 10.0);
        packet.push_i16(crate::wire::saturating_trunc_f32_to_i16(
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

    /// Encode the compact all-data mode 4 response bytes.
    #[must_use]
    pub fn encode_mode4_response(
        &self,
        mode2: FloatOutBoyAllDataMode2Payload,
        mode3: FloatOutBoyAllDataMode3Payload,
        mode4: FloatOutBoyAllDataMode4Payload,
    ) -> [u8; 58] {
        self.encode_mode4_response_for_mode(4, mode2, mode3, mode4)
    }

    /// Encode the compact all-data mode 2 response bytes.
    #[must_use]
    pub fn encode_mode2_response(
        &self,
        mode: FloatOutBoyAllDataMode,
        mode2: FloatOutBoyAllDataMode2Payload,
    ) -> [u8; 41] {
        let mut packet = FloatOutBoyPacket::new();
        let base = self.encode_base_response(mode.source_id());
        packet.extend(&base);
        float_out_boy_append_all_data_mode2(&mut packet, mode2);

        packet.into_bytes()
    }

    /// Encode the compact all-data mode 3 response bytes.
    #[must_use]
    pub fn encode_mode3_response(
        &self,
        mode: FloatOutBoyAllDataMode,
        mode2: FloatOutBoyAllDataMode2Payload,
        mode3: FloatOutBoyAllDataMode3Payload,
    ) -> [u8; 54] {
        let mut packet = FloatOutBoyPacket::new();
        let base = self.encode_base_response(mode.source_id());
        packet.extend(&base);
        float_out_boy_append_all_data_mode2(&mut packet, mode2);
        float_out_boy_append_all_data_mode3(&mut packet, mode3);

        packet.into_bytes()
    }

    fn encode_mode4_response_for_mode(
        &self,
        mode: u8,
        mode2: FloatOutBoyAllDataMode2Payload,
        mode3: FloatOutBoyAllDataMode3Payload,
        mode4: FloatOutBoyAllDataMode4Payload,
    ) -> [u8; 58] {
        let mut packet = FloatOutBoyPacket::new();
        let base = self.encode_base_response(mode);
        packet.extend(&base);
        float_out_boy_append_all_data_mode2(&mut packet, mode2);
        float_out_boy_append_all_data_mode3(&mut packet, mode3);
        float_out_boy_append_all_data_mode4(&mut packet, mode4);

        packet.into_bytes()
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

vescpkg_rs::typed_fields! {
    /// Float Out Boy all-data payload snapshot used to answer compact all-data requests.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataPayloads {
        base: FloatOutBoyAllDataBasePayload => base => with_base,
        mode2: FloatOutBoyAllDataMode2Payload => mode2,
        mode3: FloatOutBoyAllDataMode3Payload => mode3 => with_mode3_ride_totals,
        mode4: FloatOutBoyAllDataMode4Payload => mode4 => with_mode4_charging,
    }
}

impl FloatOutBoyAllDataPayloads {
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
        if mode.includes_mode4() {
            FloatOutBoyAllDataResponse::Mode4(self.base.encode_mode4_response_for_mode(
                mode.source_id(),
                self.mode2,
                self.mode3,
                self.mode4,
            ))
        } else if mode.includes_mode3() {
            FloatOutBoyAllDataResponse::Mode3(
                self.base
                    .encode_mode3_response(mode, self.mode2, self.mode3),
            )
        } else if mode.includes_mode2() {
            FloatOutBoyAllDataResponse::Mode2(self.base.encode_mode2_response(mode, self.mode2))
        } else {
            FloatOutBoyAllDataResponse::Base(self.base.encode_base_response(mode.source_id()))
        }
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

vescpkg_rs::typed_fields! {
    /// Float Out Boy all-data mode 2 extension fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataMode2Payload {
        distance_abs: TripDistance => distance_abs => with_distance_abs,
        temperatures: FloatOutBoyRealtimeMotorTemperatures => temperatures => with_temperatures,
        battery_temperature: Option<Temperature> => battery_temperature,
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy all-data mode 3 extension fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataMode3Payload {
        odometer: OdometerMeters => odometer,
        discharged_charge: AmpHoursDischarged => discharged_charge,
        charged_charge: AmpHoursCharged => charged_charge,
        discharged_energy: WattHoursDischarged => discharged_energy,
        charged_energy: WattHoursCharged => charged_energy,
        battery_level: BatteryLevel => battery_level,
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy all-data mode 4 extension fields.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyAllDataMode4Payload {
        current: BatteryCurrent => current,
        voltage: BatteryVoltage => voltage,
    }
}
