//! Refloat compact all-data response types.
//!
//! C map: `cmd_send_all_data` encodes these response variants at
//! `third_party/refloat/src/main.c:1313-1399`.

use super::realtime::{
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
    RefloatRealtimeRuntimeSetpoints,
};
use super::state::{
    RefloatBeepReason, RefloatMode, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition,
};
use super::wire::{
    refloat_append_all_data_mode2, refloat_append_all_data_mode3, refloat_append_all_data_mode4,
    refloat_degrees, refloat_offset_scaled_u8, refloat_push_i16, refloat_push_scaled_i16,
    refloat_push_u8, refloat_scaled_u8,
};
use super::{
    FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataMode,
    RefloatAllDataRequest, RefloatAppDataCommand, RefloatRideState,
};
use vescpkg_rs::prelude::{
    AdcDecodedLevel, AmpHoursCharged, AmpHoursDischarged, AngleDegrees, AngleRadians,
    BatteryCurrent, BatteryLevel, BatteryVoltage, Charge, Current, Distance, DutyCycle,
    ElectricalSpeed, Energy, FirmwareFaultCompatCode, ImuPitch, ImuRoll, MosfetTemperature,
    MotorCurrent, MotorTemperature, OdometerMeters, Ratio, Rpm, SignedRatio, Speed, Temperature,
    TripDistance, VehicleSpeed, Voltage, WattHoursCharged, WattHoursDischarged,
};

/// Fixed-size Refloat all-data response bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefloatAllDataResponse {
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

impl RefloatAllDataResponse {
    /// Encode a Refloat all-data fault response.
    pub const fn fault(fault: FirmwareFaultCompatCode) -> Self {
        Self::Fault([
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            69,
            fault.compat_code(),
        ])
    }

    /// Return the encoded response bytes.
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

/// Refloat compact all-data attitude fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataAttitude {
    balance_pitch: RefloatRealtimeBalancePitch,
    roll: ImuRoll,
    pitch: ImuPitch,
}

impl RefloatAllDataAttitude {
    /// Build typed compact all-data attitude fields.
    pub const fn new(
        balance_pitch: RefloatRealtimeBalancePitch,
        roll: ImuRoll,
        pitch: ImuPitch,
    ) -> Self {
        Self {
            balance_pitch,
            roll,
            pitch,
        }
    }

    /// Return balance pitch.
    pub const fn balance_pitch(self) -> RefloatRealtimeBalancePitch {
        self.balance_pitch
    }

    /// Return IMU roll.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }

    /// Return IMU pitch.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }
}

/// Refloat compact all-data status fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatAllDataStatus {
    ride_state: RefloatRideState,
    beep_reason: RefloatBeepReason,
}

impl RefloatAllDataStatus {
    /// Build typed compact all-data status fields.
    pub const fn new(ride_state: RefloatRideState, beep_reason: RefloatBeepReason) -> Self {
        Self {
            ride_state,
            beep_reason,
        }
    }

    /// Return ride state.
    pub const fn ride_state(self) -> RefloatRideState {
        self.ride_state
    }

    /// Return beep reason.
    pub const fn beep_reason(self) -> RefloatBeepReason {
        self.beep_reason
    }
}

/// Refloat compact all-data FOC ID current state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefloatFocIdCurrent {
    /// A measured FOC ID current is available.
    Measured(MotorCurrent),
    /// Refloat will emit its source-backed unavailable marker during encoding.
    Unavailable,
}

impl RefloatFocIdCurrent {
    /// Build a measured FOC ID current value.
    pub const fn measured(current: MotorCurrent) -> Self {
        Self::Measured(current)
    }

    /// Build an unavailable FOC ID current marker.
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    /// Return the measured current, when available.
    pub const fn as_measured(self) -> Option<MotorCurrent> {
        match self {
            Self::Measured(current) => Some(current),
            Self::Unavailable => None,
        }
    }
}

/// Refloat compact all-data motor fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMotorPayload {
    battery_voltage: BatteryVoltage,
    electrical_speed: ElectricalSpeed,
    vehicle_speed: VehicleSpeed,
    motor_current: MotorCurrent,
    battery_current: BatteryCurrent,
    duty_cycle: DutyCycle,
    foc_id_current: RefloatFocIdCurrent,
}

impl RefloatAllDataMotorPayload {
    /// Build typed compact all-data motor fields.
    pub const fn new(
        battery_voltage: BatteryVoltage,
        electrical_speed: ElectricalSpeed,
        vehicle_speed: VehicleSpeed,
        motor_current: MotorCurrent,
        battery_current: BatteryCurrent,
        duty_cycle: DutyCycle,
        foc_id_current: RefloatFocIdCurrent,
    ) -> Self {
        Self {
            battery_voltage,
            electrical_speed,
            vehicle_speed,
            motor_current,
            battery_current,
            duty_cycle,
            foc_id_current,
        }
    }

    /// Return battery voltage.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }

    /// Return motor fields with refreshed battery voltage.
    pub const fn with_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            battery_voltage,
            electrical_speed: self.electrical_speed,
            vehicle_speed: self.vehicle_speed,
            motor_current: self.motor_current,
            battery_current: self.battery_current,
            duty_cycle: self.duty_cycle,
            foc_id_current: self.foc_id_current,
        }
    }

    /// Return electrical speed.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return vehicle speed.
    pub const fn vehicle_speed(self) -> VehicleSpeed {
        self.vehicle_speed
    }

    /// Return motor current.
    pub const fn motor_current(self) -> MotorCurrent {
        self.motor_current
    }

    /// Return battery current.
    pub const fn battery_current(self) -> BatteryCurrent {
        self.battery_current
    }

    /// Return duty cycle.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return FOC ID current state.
    pub const fn foc_id_current(self) -> RefloatFocIdCurrent {
        self.foc_id_current
    }
}

/// Refloat compact all-data base payload fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataBasePayload {
    balance_current: RefloatRealtimeBalanceCurrent,
    attitude: RefloatAllDataAttitude,
    status: RefloatAllDataStatus,
    footpad: FootpadSensorSample,
    setpoints: RefloatRealtimeRuntimeSetpoints,
    booster_current: RefloatRealtimeBoosterCurrent,
    motor: RefloatAllDataMotorPayload,
}

impl RefloatAllDataBasePayload {
    /// Build typed compact all-data base payload fields.
    pub const fn new(
        balance_current: RefloatRealtimeBalanceCurrent,
        attitude: RefloatAllDataAttitude,
        status: RefloatAllDataStatus,
        footpad: FootpadSensorSample,
        setpoints: RefloatRealtimeRuntimeSetpoints,
        booster_current: RefloatRealtimeBoosterCurrent,
        motor: RefloatAllDataMotorPayload,
    ) -> Self {
        Self {
            balance_current,
            attitude,
            status,
            footpad,
            setpoints,
            booster_current,
            motor,
        }
    }

    /// Return the Refloat app-data command this payload belongs to.
    pub const fn command(self) -> RefloatAppDataCommand {
        RefloatAppDataCommand::GetAllData
    }

    /// Encode the compact all-data base response bytes.
    ///
    /// C map: `cmd_all_data` writes degree-valued IMU fields with scale 10 at
    /// `third_party/refloat/src/main.c:1328-1365`; Rust stores the source IMU
    /// readings as typed radians and converts at this wire boundary.
    pub fn encode_base_response(&self, mode: u8) -> [u8; 34] {
        let mut buffer = [0; 34];
        let mut ind = 0;

        refloat_push_u8(&mut buffer, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
        refloat_push_u8(&mut buffer, &mut ind, self.command().id());
        refloat_push_u8(&mut buffer, &mut ind, mode);
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.balance_current.current().current().as_amps(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            refloat_degrees(self.attitude.balance_pitch().angle()),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            refloat_degrees(self.attitude.roll().angle()),
            10.0,
        );

        let ride_state = self.status.ride_state;
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            (ride_state.float_state_compat() & 0x0f)
                + (ride_state.setpoint_adjustment_compat() << 4),
        );

        let handtest = matches!(ride_state.mode(), RefloatMode::HandTest);
        let switch_state = self.footpad.state().switch_compat() | u8::from(handtest) << 3;
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            (switch_state & 0x0f) + (self.status.beep_reason.id() << 4),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_scaled_u8(self.footpad.adc1_volts(), 50.0),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_scaled_u8(self.footpad.adc2_volts(), 50.0),
        );

        [
            self.setpoints.board(),
            self.setpoints.atr(),
            self.setpoints.brake_tilt(),
            self.setpoints.torque_tilt(),
            self.setpoints.turn_tilt(),
            self.setpoints.remote(),
        ]
        .into_iter()
        .map(|setpoint| refloat_offset_scaled_u8(setpoint.angle().as_degrees(), 5.0, 128.0))
        .for_each(|value| refloat_push_u8(&mut buffer, &mut ind, value));

        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            refloat_degrees(self.attitude.pitch().angle()),
            10.0,
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_offset_scaled_u8(
                self.booster_current.current().current().as_amps(),
                1.0,
                128.0,
            ),
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.battery_voltage().voltage().as_volts(),
            10.0,
        );
        refloat_push_i16(
            &mut buffer,
            &mut ind,
            self.motor
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute() as i16,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.vehicle_speed().speed().as_meters_per_second(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.motor_current().current().as_amps(),
            10.0,
        );
        refloat_push_scaled_i16(
            &mut buffer,
            &mut ind,
            self.motor.battery_current().current().as_amps(),
            10.0,
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            refloat_offset_scaled_u8(self.motor.duty_cycle().ratio().as_ratio(), 100.0, 128.0),
        );
        refloat_push_u8(
            &mut buffer,
            &mut ind,
            self.motor
                .foc_id_current()
                .as_measured()
                .map_or(222, |current| {
                    refloat_scaled_u8(current.current().as_amps().abs(), 3.0)
                }),
        );

        buffer
    }

    /// Encode the compact all-data mode 4 response bytes.
    pub fn encode_mode4_response(
        &self,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> [u8; 58] {
        self.encode_mode4_response_for_mode(4, mode2, mode3, mode4)
    }

    /// Encode the compact all-data mode 2 response bytes.
    pub fn encode_mode2_response(
        &self,
        mode: RefloatAllDataMode,
        mode2: RefloatAllDataMode2Payload,
    ) -> [u8; 41] {
        let mut buffer = [0; 41];
        let base = self.encode_base_response(mode.source_id());
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);

        buffer
    }

    /// Encode the compact all-data mode 3 response bytes.
    pub fn encode_mode3_response(
        &self,
        mode: RefloatAllDataMode,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
    ) -> [u8; 54] {
        let mut buffer = [0; 54];
        let base = self.encode_base_response(mode.source_id());
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);
        refloat_append_all_data_mode3(&mut buffer, &mut ind, mode3);

        buffer
    }

    fn encode_mode4_response_for_mode(
        &self,
        mode: u8,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> [u8; 58] {
        let mut buffer = [0; 58];
        let base = self.encode_base_response(mode);
        buffer[..base.len()].copy_from_slice(&base);
        let mut ind = base.len();

        refloat_append_all_data_mode2(&mut buffer, &mut ind, mode2);
        refloat_append_all_data_mode3(&mut buffer, &mut ind, mode3);
        refloat_append_all_data_mode4(&mut buffer, &mut ind, mode4);

        buffer
    }

    /// Return balance current.
    pub const fn balance_current(self) -> RefloatRealtimeBalanceCurrent {
        self.balance_current
    }

    /// Return attitude fields.
    pub const fn attitude(self) -> RefloatAllDataAttitude {
        self.attitude
    }

    /// Return status fields.
    pub const fn status(self) -> RefloatAllDataStatus {
        self.status
    }

    /// Return footpad sample.
    pub const fn footpad(self) -> FootpadSensorSample {
        self.footpad
    }

    /// Return runtime setpoints.
    pub const fn setpoints(self) -> RefloatRealtimeRuntimeSetpoints {
        self.setpoints
    }

    /// Return booster current.
    pub const fn booster_current(self) -> RefloatRealtimeBoosterCurrent {
        self.booster_current
    }

    /// Return motor payload.
    pub const fn motor(self) -> RefloatAllDataMotorPayload {
        self.motor
    }

    /// Return base all-data fields with refreshed motor battery voltage.
    pub const fn with_motor_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self {
            balance_current: self.balance_current,
            attitude: self.attitude,
            status: self.status,
            footpad: self.footpad,
            setpoints: self.setpoints,
            booster_current: self.booster_current,
            motor: self.motor.with_battery_voltage(battery_voltage),
        }
    }
}

/// Refloat all-data payload snapshot used to answer compact all-data requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataPayloads {
    base: RefloatAllDataBasePayload,
    mode2: RefloatAllDataMode2Payload,
    mode3: RefloatAllDataMode3Payload,
    mode4: RefloatAllDataMode4Payload,
}

impl RefloatAllDataPayloads {
    /// Build a complete all-data payload snapshot.
    pub const fn new(
        base: RefloatAllDataBasePayload,
        mode2: RefloatAllDataMode2Payload,
        mode3: RefloatAllDataMode3Payload,
        mode4: RefloatAllDataMode4Payload,
    ) -> Self {
        Self {
            base,
            mode2,
            mode3,
            mode4,
        }
    }

    /// Build the Refloat `v1.2.1` startup all-data snapshot after `data_init`.
    ///
    /// Upstream zeroes and initializes `Data` in `third_party/refloat/src/main.c:1190-1205`; this
    /// Rust snapshot is a test/default model, not proof of hardware state.
    pub const fn source_startup() -> Self {
        let zero_current = Current::from_amps(0.0);
        let zero_angle = AngleRadians::from_radians(0.0);
        let zero_motor_current = MotorCurrent::new(zero_current);
        let zero_battery_current = BatteryCurrent::new(zero_current);
        let zero_voltage = BatteryVoltage::new(Voltage::from_volts(0.0));
        let ride_state = RefloatRideState::new(
            RefloatRunState::Startup,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        Self::new(
            RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(zero_motor_current),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(zero_angle),
                    ImuRoll::new(zero_angle),
                    ImuPitch::new(zero_angle),
                ),
                RefloatAllDataStatus::new(ride_state, RefloatBeepReason::None),
                FootpadSensorSample::new(
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    FootpadSensorState::None,
                ),
                RefloatRealtimeRuntimeSetpoints::new(
                    setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
                ),
                RefloatRealtimeBoosterCurrent::new(zero_motor_current),
                RefloatAllDataMotorPayload::new(
                    zero_voltage,
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                    zero_motor_current,
                    zero_battery_current,
                    DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
                    RefloatFocIdCurrent::unavailable(),
                ),
            ),
            RefloatAllDataMode2Payload::new(
                TripDistance::new(Distance::from_meters(0.0)),
                RefloatRealtimeMotorTemperatures::new(
                    MosfetTemperature::new(Temperature::from_degrees_celsius(0.0)),
                    MotorTemperature::new(Temperature::from_degrees_celsius(0.0)),
                ),
                RefloatAllDataBatteryTemperature::unavailable(),
            ),
            RefloatAllDataMode3Payload::new(
                OdometerMeters::from_meters(0),
                AmpHoursDischarged::new(Charge::from_amp_hours(0.0)),
                AmpHoursCharged::new(Charge::from_amp_hours(0.0)),
                WattHoursDischarged::new(Energy::from_watt_hours(0.0)),
                WattHoursCharged::new(Energy::from_watt_hours(0.0)),
                BatteryLevel::new(Ratio::from_ratio_const(0.0)),
            ),
            RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(zero_battery_current),
                RefloatRealtimeChargingVoltage::new(zero_voltage),
            ),
        )
    }

    /// Encode the source-compatible response for a parsed all-data request.
    ///
    /// The byte order and mode gates mirror `cmd_send_all_data` in upstream
    /// `third_party/refloat/src/main.c:1313-1399`.
    #[inline(never)]
    pub fn encode_response(&self, request: RefloatAllDataRequest) -> RefloatAllDataResponse {
        let mode = request.mode();
        if mode.includes_mode4() {
            RefloatAllDataResponse::Mode4(self.base.encode_mode4_response_for_mode(
                mode.source_id(),
                self.mode2,
                self.mode3,
                self.mode4,
            ))
        } else if mode.includes_mode3() {
            RefloatAllDataResponse::Mode3(
                self.base
                    .encode_mode3_response(mode, self.mode2, self.mode3),
            )
        } else if mode.includes_mode2() {
            RefloatAllDataResponse::Mode2(self.base.encode_mode2_response(mode, self.mode2))
        } else {
            RefloatAllDataResponse::Base(self.base.encode_base_response(mode.source_id()))
        }
    }

    /// Return base all-data payload fields.
    pub const fn base(self) -> RefloatAllDataBasePayload {
        self.base
    }

    /// Return mode 2 all-data extension fields.
    pub const fn mode2(self) -> RefloatAllDataMode2Payload {
        self.mode2
    }

    /// Return a payload snapshot with refreshed base battery voltage.
    pub const fn with_base_battery_voltage(self, battery_voltage: BatteryVoltage) -> Self {
        Self::new(
            self.base.with_motor_battery_voltage(battery_voltage),
            self.mode2,
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed absolute-distance mode 2 data.
    pub const fn with_mode2_distance_abs(self, distance_abs: TripDistance) -> Self {
        Self::new(
            self.base,
            self.mode2.with_distance_abs(distance_abs),
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed mode 2 motor temperatures.
    pub const fn with_mode2_temperatures(
        self,
        temperatures: RefloatRealtimeMotorTemperatures,
    ) -> Self {
        Self::new(
            self.base,
            self.mode2.with_temperatures(temperatures),
            self.mode3,
            self.mode4,
        )
    }

    /// Return a payload snapshot with refreshed mode 3 ride totals.
    pub const fn with_mode3_ride_totals(self, mode3: RefloatAllDataMode3Payload) -> Self {
        Self::new(self.base, self.mode2, mode3, self.mode4)
    }

    /// Return mode 3 all-data extension fields.
    pub const fn mode3(self) -> RefloatAllDataMode3Payload {
        self.mode3
    }

    /// Return a payload snapshot with refreshed mode 4 charging data.
    pub const fn with_mode4_charging(self, mode4: RefloatAllDataMode4Payload) -> Self {
        Self::new(self.base, self.mode2, self.mode3, mode4)
    }

    /// Return mode 4 all-data extension fields.
    pub const fn mode4(self) -> RefloatAllDataMode4Payload {
        self.mode4
    }
}

/// Refloat all-data battery-temperature state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefloatAllDataBatteryTemperature {
    /// A measured battery temperature is available.
    Measured(Temperature),
    /// Refloat `v1.2.1` emits a zero placeholder for this field.
    Unavailable,
}

impl RefloatAllDataBatteryTemperature {
    /// Build a measured battery-temperature value.
    pub const fn measured(temperature: Temperature) -> Self {
        Self::Measured(temperature)
    }

    /// Build an unavailable battery-temperature marker.
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    /// Return the measured battery temperature, when available.
    pub const fn as_measured(self) -> Option<Temperature> {
        match self {
            Self::Measured(temperature) => Some(temperature),
            Self::Unavailable => None,
        }
    }
}

/// Refloat all-data mode 2 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode2Payload {
    distance_abs: TripDistance,
    temperatures: RefloatRealtimeMotorTemperatures,
    battery_temperature: RefloatAllDataBatteryTemperature,
}

impl RefloatAllDataMode2Payload {
    /// Build typed all-data mode 2 extension fields.
    pub const fn new(
        distance_abs: TripDistance,
        temperatures: RefloatRealtimeMotorTemperatures,
        battery_temperature: RefloatAllDataBatteryTemperature,
    ) -> Self {
        Self {
            distance_abs,
            temperatures,
            battery_temperature,
        }
    }

    /// Return absolute distance.
    pub const fn distance_abs(self) -> TripDistance {
        self.distance_abs
    }

    /// Return mode 2 fields with refreshed absolute distance.
    pub const fn with_distance_abs(self, distance_abs: TripDistance) -> Self {
        Self::new(distance_abs, self.temperatures, self.battery_temperature)
    }

    /// Return mode 2 fields with refreshed motor temperatures.
    pub const fn with_temperatures(self, temperatures: RefloatRealtimeMotorTemperatures) -> Self {
        Self::new(self.distance_abs, temperatures, self.battery_temperature)
    }

    /// Return motor temperatures.
    pub const fn temperatures(self) -> RefloatRealtimeMotorTemperatures {
        self.temperatures
    }

    /// Return battery-temperature state.
    pub const fn battery_temperature(self) -> RefloatAllDataBatteryTemperature {
        self.battery_temperature
    }
}

/// Refloat all-data mode 3 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode3Payload {
    odometer: OdometerMeters,
    discharged_charge: AmpHoursDischarged,
    charged_charge: AmpHoursCharged,
    discharged_energy: WattHoursDischarged,
    charged_energy: WattHoursCharged,
    battery_level: BatteryLevel,
}

impl RefloatAllDataMode3Payload {
    /// Build typed all-data mode 3 extension fields.
    pub const fn new(
        odometer: OdometerMeters,
        discharged_charge: AmpHoursDischarged,
        charged_charge: AmpHoursCharged,
        discharged_energy: WattHoursDischarged,
        charged_energy: WattHoursCharged,
        battery_level: BatteryLevel,
    ) -> Self {
        Self {
            odometer,
            discharged_charge,
            charged_charge,
            discharged_energy,
            charged_energy,
            battery_level,
        }
    }

    /// Return odometer distance.
    pub const fn odometer(self) -> OdometerMeters {
        self.odometer
    }

    /// Return discharged amp-hours.
    pub const fn discharged_charge(self) -> AmpHoursDischarged {
        self.discharged_charge
    }

    /// Return charged amp-hours.
    pub const fn charged_charge(self) -> AmpHoursCharged {
        self.charged_charge
    }

    /// Return discharged watt-hours.
    pub const fn discharged_energy(self) -> WattHoursDischarged {
        self.discharged_energy
    }

    /// Return charged watt-hours.
    pub const fn charged_energy(self) -> WattHoursCharged {
        self.charged_energy
    }

    /// Return battery state of charge.
    pub const fn battery_level(self) -> BatteryLevel {
        self.battery_level
    }
}

/// Refloat all-data mode 4 extension fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAllDataMode4Payload {
    current: RefloatRealtimeChargingCurrent,
    voltage: RefloatRealtimeChargingVoltage,
}

impl RefloatAllDataMode4Payload {
    /// Build typed all-data mode 4 extension fields.
    pub const fn new(
        current: RefloatRealtimeChargingCurrent,
        voltage: RefloatRealtimeChargingVoltage,
    ) -> Self {
        Self { current, voltage }
    }

    /// Return charging current.
    pub const fn current(self) -> RefloatRealtimeChargingCurrent {
        self.current
    }

    /// Return charging voltage.
    pub const fn voltage(self) -> RefloatRealtimeChargingVoltage {
        self.voltage
    }
}
