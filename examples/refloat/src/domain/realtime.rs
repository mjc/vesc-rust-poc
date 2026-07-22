//! Refloat realtime semantic domain types.
//!
//! Source map: realtime ID lists and payloads mirror Refloat `v1.2.1` at
//! `third_party/refloat/src/rt_data.h:38-66` and `third_party/refloat/src/main.c:1876-1960`.

use super::{
    RefloatBeepReason, RefloatChargingState, RefloatDarkRideState, RefloatDataRecorderFlags,
    RefloatFatalErrorState, RefloatFootpadSample, RefloatFootpadState, RefloatRideState,
    RefloatRunState, RefloatWheelSlipState,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, BatteryVoltage, DirectionalMotorCurrent, DutyCycle,
    ElectricalSpeed, FirmwareFaultWireCode, ImuPitch, ImuRoll, MosfetTemperature, MotorCurrent,
    MotorTemperature, SignedRatio, TimestampTicks, VehicleSpeed,
};

/// The ID-list packet format is described in upstream `third_party/refloat/src/main.c:1884-1898`.
pub const REFLOAT_REALTIME_DATA_ITEMS: [RefloatRealtimeDataItem; 16] = [
    RefloatRealtimeDataItem::MotorSpeed,
    RefloatRealtimeDataItem::MotorErpm,
    RefloatRealtimeDataItem::MotorCurrent,
    RefloatRealtimeDataItem::MotorDirectionalCurrent,
    RefloatRealtimeDataItem::MotorFilteredCurrent,
    RefloatRealtimeDataItem::MotorDutyCycle,
    RefloatRealtimeDataItem::MotorBatteryVoltage,
    RefloatRealtimeDataItem::MotorBatteryCurrent,
    RefloatRealtimeDataItem::MotorMosfetTemperature,
    RefloatRealtimeDataItem::MotorTemperature,
    RefloatRealtimeDataItem::ImuPitch,
    RefloatRealtimeDataItem::ImuBalancePitch,
    RefloatRealtimeDataItem::ImuRoll,
    RefloatRealtimeDataItem::FootpadAdc1,
    RefloatRealtimeDataItem::FootpadAdc2,
    RefloatRealtimeDataItem::RemoteInput,
];

/// Refloat realtime-data items sent only while running.
///
/// Upstream appends this second ID set after the always-sent set in
/// `third_party/refloat/src/main.c:1892-1898`.
pub const REFLOAT_REALTIME_RUNTIME_ITEMS: [RefloatRealtimeDataItem; 10] = [
    RefloatRealtimeDataItem::Setpoint,
    RefloatRealtimeDataItem::AtrSetpoint,
    RefloatRealtimeDataItem::BrakeTiltSetpoint,
    RefloatRealtimeDataItem::TorqueTiltSetpoint,
    RefloatRealtimeDataItem::TurnTiltSetpoint,
    RefloatRealtimeDataItem::RemoteSetpoint,
    RefloatRealtimeDataItem::BalanceCurrent,
    RefloatRealtimeDataItem::AtrAccelDiff,
    RefloatRealtimeDataItem::AtrSpeedBoost,
    RefloatRealtimeDataItem::BoosterCurrent,
];

/// Refloat realtime-data items recorded by the data recorder.
///
/// This list mirrors the port's current data-recorder model; re-check against
/// upstream `third_party/refloat/src/data_recorder.c` before treating it as hardware parity.
pub const REFLOAT_REALTIME_RECORDED_ITEMS: [RefloatRealtimeDataItem; 10] = [
    RefloatRealtimeDataItem::MotorErpm,
    RefloatRealtimeDataItem::MotorDirectionalCurrent,
    RefloatRealtimeDataItem::MotorDutyCycle,
    RefloatRealtimeDataItem::MotorBatteryVoltage,
    RefloatRealtimeDataItem::ImuPitch,
    RefloatRealtimeDataItem::ImuBalancePitch,
    RefloatRealtimeDataItem::Setpoint,
    RefloatRealtimeDataItem::AtrSetpoint,
    RefloatRealtimeDataItem::TorqueTiltSetpoint,
    RefloatRealtimeDataItem::BalanceCurrent,
];

/// Refloat realtime-data item group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataItemGroup {
    /// Always sent in realtime data.
    Always,
    /// Sent only while the board is running.
    Runtime,
}

/// Refloat data-recorder policy for a realtime-data item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataRecordPolicy {
    /// Send in realtime data only.
    SendOnly,
    /// Send in realtime data and record in the data recorder.
    Record,
}

/// Refloat realtime-data item ID.
///
/// C map: item order, always/runtime grouping, and data-recorder policy mirror
/// `RT_DATA_ITEMS` / `RT_DATA_RUNTIME_ITEMS` in
/// `third_party/refloat/src/rt_data.h:38-66`; upstream sends the two ID lists
/// from `third_party/refloat/src/main.c:1876-1901`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRealtimeDataItem {
    /// `motor.speed`.
    MotorSpeed,
    /// `motor.erpm`.
    MotorErpm,
    /// `motor.current`.
    MotorCurrent,
    /// `motor.dir_current`.
    MotorDirectionalCurrent,
    /// `motor.filt_current`.
    MotorFilteredCurrent,
    /// `motor.duty_cycle`.
    MotorDutyCycle,
    /// `motor.batt_voltage`.
    MotorBatteryVoltage,
    /// `motor.batt_current`.
    MotorBatteryCurrent,
    /// `motor.mosfet_temp`.
    MotorMosfetTemperature,
    /// `motor.motor_temp`.
    MotorTemperature,
    /// `imu.pitch`.
    ImuPitch,
    /// `imu.balance_pitch`.
    ImuBalancePitch,
    /// `imu.roll`.
    ImuRoll,
    /// `footpad.adc1`.
    FootpadAdc1,
    /// `footpad.adc2`.
    FootpadAdc2,
    /// `remote.input`.
    RemoteInput,
    /// `setpoint`.
    Setpoint,
    /// `atr.setpoint`.
    AtrSetpoint,
    /// `brake_tilt.setpoint`.
    BrakeTiltSetpoint,
    /// `torque_tilt.setpoint`.
    TorqueTiltSetpoint,
    /// `turn_tilt.setpoint`.
    TurnTiltSetpoint,
    /// `remote.setpoint`.
    RemoteSetpoint,
    /// `balance_current`.
    BalanceCurrent,
    /// `atr.accel_diff`.
    AtrAccelDiff,
    /// `atr.speed_boost`.
    AtrSpeedBoost,
    /// `booster.current`.
    BoosterCurrent,
}

impl RefloatRealtimeDataItem {
    /// Return the Refloat `v1.2.1` realtime-data string ID.
    pub const fn id(self) -> &'static str {
        match self {
            Self::MotorSpeed => "motor.speed",
            Self::MotorErpm => "motor.erpm",
            Self::MotorCurrent => "motor.current",
            Self::MotorDirectionalCurrent => "motor.dir_current",
            Self::MotorFilteredCurrent => "motor.filt_current",
            Self::MotorDutyCycle => "motor.duty_cycle",
            Self::MotorBatteryVoltage => "motor.batt_voltage",
            Self::MotorBatteryCurrent => "motor.batt_current",
            Self::MotorMosfetTemperature => "motor.mosfet_temp",
            Self::MotorTemperature => "motor.motor_temp",
            Self::ImuPitch => "imu.pitch",
            Self::ImuBalancePitch => "imu.balance_pitch",
            Self::ImuRoll => "imu.roll",
            Self::FootpadAdc1 => "footpad.adc1",
            Self::FootpadAdc2 => "footpad.adc2",
            Self::RemoteInput => "remote.input",
            Self::Setpoint => "setpoint",
            Self::AtrSetpoint => "atr.setpoint",
            Self::BrakeTiltSetpoint => "brake_tilt.setpoint",
            Self::TorqueTiltSetpoint => "torque_tilt.setpoint",
            Self::TurnTiltSetpoint => "turn_tilt.setpoint",
            Self::RemoteSetpoint => "remote.setpoint",
            Self::BalanceCurrent => "balance_current",
            Self::AtrAccelDiff => "atr.accel_diff",
            Self::AtrSpeedBoost => "atr.speed_boost",
            Self::BoosterCurrent => "booster.current",
        }
    }

    /// Return the Refloat `v1.2.1` realtime-data group.
    pub const fn group(self) -> RefloatRealtimeDataItemGroup {
        match self {
            Self::Setpoint
            | Self::AtrSetpoint
            | Self::BrakeTiltSetpoint
            | Self::TorqueTiltSetpoint
            | Self::TurnTiltSetpoint
            | Self::RemoteSetpoint
            | Self::BalanceCurrent
            | Self::AtrAccelDiff
            | Self::AtrSpeedBoost
            | Self::BoosterCurrent => RefloatRealtimeDataItemGroup::Runtime,
            Self::MotorSpeed
            | Self::MotorErpm
            | Self::MotorCurrent
            | Self::MotorDirectionalCurrent
            | Self::MotorFilteredCurrent
            | Self::MotorDutyCycle
            | Self::MotorBatteryVoltage
            | Self::MotorBatteryCurrent
            | Self::MotorMosfetTemperature
            | Self::MotorTemperature
            | Self::ImuPitch
            | Self::ImuBalancePitch
            | Self::ImuRoll
            | Self::FootpadAdc1
            | Self::FootpadAdc2
            | Self::RemoteInput => RefloatRealtimeDataItemGroup::Always,
        }
    }

    /// Return the Refloat `v1.2.1` data-recorder policy.
    pub const fn record_policy(self) -> RefloatRealtimeDataRecordPolicy {
        match self {
            Self::MotorErpm
            | Self::MotorDirectionalCurrent
            | Self::MotorDutyCycle
            | Self::MotorBatteryVoltage
            | Self::ImuPitch
            | Self::ImuBalancePitch
            | Self::Setpoint
            | Self::AtrSetpoint
            | Self::TorqueTiltSetpoint
            | Self::BalanceCurrent => RefloatRealtimeDataRecordPolicy::Record,
            Self::MotorSpeed
            | Self::MotorCurrent
            | Self::MotorFilteredCurrent
            | Self::MotorBatteryCurrent
            | Self::MotorMosfetTemperature
            | Self::MotorTemperature
            | Self::ImuRoll
            | Self::FootpadAdc1
            | Self::FootpadAdc2
            | Self::RemoteInput
            | Self::BrakeTiltSetpoint
            | Self::TurnTiltSetpoint
            | Self::RemoteSetpoint
            | Self::AtrAccelDiff
            | Self::AtrSpeedBoost
            | Self::BoosterCurrent => RefloatRealtimeDataRecordPolicy::SendOnly,
        }
    }
}

/// Refloat `motor.filt_current` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeFilteredMotorCurrent(DirectionalMotorCurrent);

impl RefloatRealtimeFilteredMotorCurrent {
    /// Build a typed Refloat filtered-current value.
    pub const fn new(current: DirectionalMotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed filtered current without erasing it to a primitive.
    pub const fn current(self) -> DirectionalMotorCurrent {
        self.0
    }
}

/// Refloat `imu.balance_pitch` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBalancePitch(AngleRadians);

impl RefloatRealtimeBalancePitch {
    /// Build a typed Refloat balance-pitch value.
    pub const fn new(angle: AngleRadians) -> Self {
        Self(angle)
    }

    /// Return the typed balance-pitch angle without erasing it to a primitive.
    pub const fn angle(self) -> AngleRadians {
        self.0
    }

    /// Return the balance pitch in degrees for Refloat PID and booster math.
    pub fn angle_degrees(self) -> AngleDegrees {
        AngleDegrees::from(self.0)
    }
}

/// Refloat `remote.input` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeRemoteInput(SignedRatio);

impl RefloatRealtimeRemoteInput {
    /// Build a typed Refloat remote-input value.
    pub const fn new(ratio: SignedRatio) -> Self {
        Self(ratio)
    }

    /// Return the typed remote input without erasing it to a primitive.
    pub const fn ratio(self) -> SignedRatio {
        self.0
    }
}

/// Refloat realtime motor-current values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorCurrents {
    motor: MotorCurrent,
    directional: DirectionalMotorCurrent,
    filtered: RefloatRealtimeFilteredMotorCurrent,
    battery: BatteryCurrent,
}

impl RefloatRealtimeMotorCurrents {
    /// Build typed Refloat realtime current values.
    pub const fn new(
        motor: MotorCurrent,
        directional: DirectionalMotorCurrent,
        filtered: RefloatRealtimeFilteredMotorCurrent,
        battery: BatteryCurrent,
    ) -> Self {
        Self {
            motor,
            directional,
            filtered,
            battery,
        }
    }

    /// Return `motor.current`.
    pub const fn motor(self) -> MotorCurrent {
        self.motor
    }

    /// Return `motor.dir_current`.
    pub const fn directional(self) -> DirectionalMotorCurrent {
        self.directional
    }

    /// Return `motor.filt_current`.
    pub const fn filtered(self) -> RefloatRealtimeFilteredMotorCurrent {
        self.filtered
    }

    /// Return `motor.batt_current`.
    pub const fn battery(self) -> BatteryCurrent {
        self.battery
    }
}

/// Refloat realtime motor-temperature values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorTemperatures {
    mosfet: MosfetTemperature,
    motor: MotorTemperature,
}

impl RefloatRealtimeMotorTemperatures {
    /// Build typed Refloat realtime motor-temperature values.
    pub const fn new(mosfet: MosfetTemperature, motor: MotorTemperature) -> Self {
        Self { mosfet, motor }
    }

    /// Return `motor.mosfet_temp`.
    pub const fn mosfet(self) -> MosfetTemperature {
        self.mosfet
    }

    /// Return `motor.motor_temp`.
    pub const fn motor(self) -> MotorTemperature {
        self.motor
    }
}

/// Refloat realtime motor payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeMotorPayload {
    speed: VehicleSpeed,
    electrical_speed: ElectricalSpeed,
    currents: RefloatRealtimeMotorCurrents,
    duty_cycle: DutyCycle,
    battery_voltage: BatteryVoltage,
    temperatures: RefloatRealtimeMotorTemperatures,
}

impl RefloatRealtimeMotorPayload {
    /// Build typed Refloat realtime motor values.
    pub const fn new(
        speed: VehicleSpeed,
        electrical_speed: ElectricalSpeed,
        currents: RefloatRealtimeMotorCurrents,
        duty_cycle: DutyCycle,
        battery_voltage: BatteryVoltage,
        temperatures: RefloatRealtimeMotorTemperatures,
    ) -> Self {
        Self {
            speed,
            electrical_speed,
            currents,
            duty_cycle,
            battery_voltage,
            temperatures,
        }
    }

    /// Return `motor.speed`.
    pub const fn speed(self) -> VehicleSpeed {
        self.speed
    }

    /// Return `motor.erpm`.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return grouped motor-current values.
    pub const fn currents(self) -> RefloatRealtimeMotorCurrents {
        self.currents
    }

    /// Return `motor.duty_cycle`.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return `motor.batt_voltage`.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }

    /// Return grouped motor-temperature values.
    pub const fn temperatures(self) -> RefloatRealtimeMotorTemperatures {
        self.temperatures
    }
}

/// Refloat realtime IMU payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeImuPayload {
    pitch: ImuPitch,
    balance_pitch: RefloatRealtimeBalancePitch,
    roll: ImuRoll,
}

impl RefloatRealtimeImuPayload {
    /// Build typed Refloat realtime IMU values.
    pub const fn new(
        pitch: ImuPitch,
        balance_pitch: RefloatRealtimeBalancePitch,
        roll: ImuRoll,
    ) -> Self {
        Self {
            pitch,
            balance_pitch,
            roll,
        }
    }

    /// Return `imu.pitch`.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }

    /// Return `imu.balance_pitch`.
    pub const fn balance_pitch(self) -> RefloatRealtimeBalancePitch {
        self.balance_pitch
    }

    /// Return `imu.roll`.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }
}

/// Refloat realtime payload values that are always sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeAlwaysPayload {
    motor: RefloatRealtimeMotorPayload,
    imu: RefloatRealtimeImuPayload,
    footpad: RefloatFootpadSample,
    remote_input: RefloatRealtimeRemoteInput,
}

impl RefloatRealtimeAlwaysPayload {
    /// Build typed Refloat realtime values that are always sent.
    pub const fn new(
        motor: RefloatRealtimeMotorPayload,
        imu: RefloatRealtimeImuPayload,
        footpad: RefloatFootpadSample,
        remote_input: RefloatRealtimeRemoteInput,
    ) -> Self {
        Self {
            motor,
            imu,
            footpad,
            remote_input,
        }
    }

    /// Return the source-backed item contract for this payload section.
    pub const fn item_contract(self) -> [RefloatRealtimeDataItem; 16] {
        REFLOAT_REALTIME_DATA_ITEMS
    }

    /// Return grouped motor values.
    pub const fn motor(self) -> RefloatRealtimeMotorPayload {
        self.motor
    }

    /// Return grouped IMU values.
    pub const fn imu(self) -> RefloatRealtimeImuPayload {
        self.imu
    }

    /// Return grouped footpad values.
    pub const fn footpad(self) -> RefloatFootpadSample {
        self.footpad
    }

    /// Return `remote.input`.
    pub const fn remote_input(self) -> RefloatRealtimeRemoteInput {
        self.remote_input
    }
}

/// Refloat runtime setpoint angle value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeRuntimeSetpoint(AngleDegrees);

impl RefloatRealtimeRuntimeSetpoint {
    /// Build a typed Refloat runtime setpoint value.
    pub const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    /// Return the typed setpoint angle without erasing it to a primitive.
    pub const fn angle(self) -> AngleDegrees {
        self.0
    }
}

/// Refloat runtime setpoint values sent only while running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimeSetpoints {
    board: RefloatRealtimeRuntimeSetpoint,
    atr: RefloatRealtimeRuntimeSetpoint,
    brake_tilt: RefloatRealtimeRuntimeSetpoint,
    torque_tilt: RefloatRealtimeRuntimeSetpoint,
    turn_tilt: RefloatRealtimeRuntimeSetpoint,
    remote: RefloatRealtimeRuntimeSetpoint,
}

impl RefloatRealtimeRuntimeSetpoints {
    /// Build typed Refloat runtime setpoint values.
    pub const fn new(
        board: RefloatRealtimeRuntimeSetpoint,
        atr: RefloatRealtimeRuntimeSetpoint,
        brake_tilt: RefloatRealtimeRuntimeSetpoint,
        torque_tilt: RefloatRealtimeRuntimeSetpoint,
        turn_tilt: RefloatRealtimeRuntimeSetpoint,
        remote: RefloatRealtimeRuntimeSetpoint,
    ) -> Self {
        Self {
            board,
            atr,
            brake_tilt,
            torque_tilt,
            turn_tilt,
            remote,
        }
    }

    /// Return `setpoint`.
    pub const fn board(self) -> RefloatRealtimeRuntimeSetpoint {
        self.board
    }

    /// Return `atr.setpoint`.
    pub const fn atr(self) -> RefloatRealtimeRuntimeSetpoint {
        self.atr
    }

    /// Return `brake_tilt.setpoint`.
    pub const fn brake_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.brake_tilt
    }

    /// Return `torque_tilt.setpoint`.
    pub const fn torque_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.torque_tilt
    }

    /// Return `turn_tilt.setpoint`.
    pub const fn turn_tilt(self) -> RefloatRealtimeRuntimeSetpoint {
        self.turn_tilt
    }

    /// Return `remote.setpoint`.
    pub const fn remote(self) -> RefloatRealtimeRuntimeSetpoint {
        self.remote
    }
}

/// Refloat `balance_current` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBalanceCurrent(MotorCurrent);

impl RefloatRealtimeBalanceCurrent {
    /// Build a typed Refloat balance-current value.
    pub const fn new(current: MotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed balance current without erasing it to a primitive.
    pub const fn current(self) -> MotorCurrent {
        self.0
    }
}

/// Refloat `atr.accel_diff` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatRealtimeAtrAccelerationDiff(f32);

impl RefloatRealtimeAtrAccelerationDiff {
    /// Build a typed Refloat ATR acceleration-difference value from ERPM delta units.
    pub const fn from_erpm_delta(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat ATR acceleration-difference value in ERPM delta units.
    pub const fn as_erpm_delta(self) -> f32 {
        self.0
    }
}

/// Refloat `atr.speed_boost` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RefloatRealtimeAtrSpeedBoost(f32);

impl RefloatRealtimeAtrSpeedBoost {
    /// Build a typed Refloat ATR speed-boost value.
    pub const fn from_units(value: f32) -> Self {
        Self(value)
    }

    /// Return the Refloat ATR speed-boost value.
    pub const fn as_units(self) -> f32 {
        self.0
    }
}

/// Refloat runtime ATR payload values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimeAtrPayload {
    accel_diff: RefloatRealtimeAtrAccelerationDiff,
    speed_boost: RefloatRealtimeAtrSpeedBoost,
}

impl RefloatRealtimeRuntimeAtrPayload {
    /// Build typed Refloat runtime ATR payload values.
    pub const fn new(
        accel_diff: RefloatRealtimeAtrAccelerationDiff,
        speed_boost: RefloatRealtimeAtrSpeedBoost,
    ) -> Self {
        Self {
            accel_diff,
            speed_boost,
        }
    }

    /// Return `atr.accel_diff`.
    pub const fn accel_diff(self) -> RefloatRealtimeAtrAccelerationDiff {
        self.accel_diff
    }

    /// Return `atr.speed_boost`.
    pub const fn speed_boost(self) -> RefloatRealtimeAtrSpeedBoost {
        self.speed_boost
    }
}

/// Refloat `booster.current` runtime realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeBoosterCurrent(MotorCurrent);

impl RefloatRealtimeBoosterCurrent {
    /// Build a typed Refloat booster-current value.
    pub const fn new(current: MotorCurrent) -> Self {
        Self(current)
    }

    /// Return the typed booster current without erasing it to a primitive.
    pub const fn current(self) -> MotorCurrent {
        self.0
    }
}

/// Refloat realtime payload values sent only while running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeRuntimePayload {
    setpoints: RefloatRealtimeRuntimeSetpoints,
    balance_current: RefloatRealtimeBalanceCurrent,
    atr: RefloatRealtimeRuntimeAtrPayload,
    booster_current: RefloatRealtimeBoosterCurrent,
}

impl RefloatRealtimeRuntimePayload {
    /// Build typed Refloat realtime values sent only while running.
    pub const fn new(
        setpoints: RefloatRealtimeRuntimeSetpoints,
        balance_current: RefloatRealtimeBalanceCurrent,
        atr: RefloatRealtimeRuntimeAtrPayload,
        booster_current: RefloatRealtimeBoosterCurrent,
    ) -> Self {
        Self {
            setpoints,
            balance_current,
            atr,
            booster_current,
        }
    }

    /// Return the source-backed item contract for this payload section.
    pub const fn item_contract(self) -> [RefloatRealtimeDataItem; 10] {
        REFLOAT_REALTIME_RUNTIME_ITEMS
    }

    /// Return grouped runtime setpoint values.
    pub const fn setpoints(self) -> RefloatRealtimeRuntimeSetpoints {
        self.setpoints
    }

    /// Return `balance_current`.
    pub const fn balance_current(self) -> RefloatRealtimeBalanceCurrent {
        self.balance_current
    }

    /// Return grouped ATR runtime values.
    pub const fn atr(self) -> RefloatRealtimeRuntimeAtrPayload {
        self.atr
    }

    /// Return `booster.current`.
    pub const fn booster_current(self) -> RefloatRealtimeBoosterCurrent {
        self.booster_current
    }
}

/// Refloat `charging.current` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeChargingCurrent(BatteryCurrent);

impl RefloatRealtimeChargingCurrent {
    /// Build a typed Refloat charging-current value.
    pub const fn new(current: BatteryCurrent) -> Self {
        Self(current)
    }

    /// Return the typed charging current without erasing it to a primitive.
    pub const fn current(self) -> BatteryCurrent {
        self.0
    }
}

/// Refloat `charging.voltage` realtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatRealtimeChargingVoltage(BatteryVoltage);

impl RefloatRealtimeChargingVoltage {
    /// Build a typed Refloat charging-voltage value.
    pub const fn new(voltage: BatteryVoltage) -> Self {
        Self(voltage)
    }

    /// Return the typed charging voltage without erasing it to a primitive.
    pub const fn voltage(self) -> BatteryVoltage {
        self.0
    }
}

/// Refloat realtime charging payload values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatRealtimeChargingPayload {
    current: RefloatRealtimeChargingCurrent,
    voltage: RefloatRealtimeChargingVoltage,
}

impl RefloatRealtimeChargingPayload {
    /// Build typed Refloat realtime charging values.
    pub const fn new(
        current: RefloatRealtimeChargingCurrent,
        voltage: RefloatRealtimeChargingVoltage,
    ) -> Self {
        Self { current, voltage }
    }

    /// Return `charging.current`.
    pub const fn current(self) -> RefloatRealtimeChargingCurrent {
        self.current
    }

    /// Return `charging.voltage`.
    pub const fn voltage(self) -> RefloatRealtimeChargingVoltage {
        self.voltage
    }
}

/// Refloat alert ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatAlertId {
    /// Firmware fault alert.
    FirmwareFault,
}

impl RefloatAlertId {
    /// Return the Refloat `v1.2.1` alert ID.
    pub const fn id(self) -> u8 {
        match self {
            Self::FirmwareFault => 1,
        }
    }

    const fn mask(self) -> u32 {
        1 << (self.id() - 1)
    }
}

/// Refloat active-alert mask appended to realtime data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RefloatRealtimeAlertMask(u32);

impl RefloatRealtimeAlertMask {
    /// Build an empty active-alert mask.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Return a copy with the alert marked active.
    pub const fn with_alert(self, alert: RefloatAlertId) -> Self {
        Self(self.0 | alert.mask())
    }

    /// Return whether the alert is active.
    pub const fn contains(self, alert: RefloatAlertId) -> bool {
        self.0 & alert.mask() != 0
    }

    /// Return the Refloat-compatible active-alert mask.
    pub const fn active_alert_mask_compat(self) -> u32 {
        self.0
    }
}

/// Refloat reserved realtime tail flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RefloatRealtimeReservedFlags(u32);

impl RefloatRealtimeReservedFlags {
    /// Build the currently empty Refloat realtime extra-flags field.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Return the Refloat-compatible extra-flags value.
    pub const fn extra_flags_compat(self) -> u32 {
        self.0
    }
}

/// Refloat realtime tail fields appended after conditional payload values.
///
/// Source map: upstream appends active-alert mask, reserved flags, and firmware
/// fault code at `third_party/refloat/src/main.c:1956-1958`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRealtimeTail {
    active_alerts: RefloatRealtimeAlertMask,
    reserved_flags: RefloatRealtimeReservedFlags,
    firmware_fault_code: FirmwareFaultWireCode,
}

impl RefloatRealtimeTail {
    /// Build typed Refloat realtime tail fields.
    pub const fn new(
        active_alerts: RefloatRealtimeAlertMask,
        reserved_flags: RefloatRealtimeReservedFlags,
        firmware_fault_code: FirmwareFaultWireCode,
    ) -> Self {
        Self {
            active_alerts,
            reserved_flags,
            firmware_fault_code,
        }
    }

    /// Return active alerts.
    pub const fn active_alerts(self) -> RefloatRealtimeAlertMask {
        self.active_alerts
    }

    /// Return reserved extra flags.
    pub const fn reserved_flags(self) -> RefloatRealtimeReservedFlags {
        self.reserved_flags
    }

    /// Return firmware fault code.
    pub const fn firmware_fault_code(self) -> FirmwareFaultWireCode {
        self.firmware_fault_code
    }
}

/// Refloat realtime-data header fields.
///
/// Source map: upstream `cmd_realtime_data` emits these header bytes at
/// `third_party/refloat/src/main.c:1912-1941`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRealtimeDataHeader {
    timestamp: TimestampTicks,
    ride_state: RefloatRideState,
    footpad_state: RefloatFootpadState,
    beep_reason: RefloatBeepReason,
    fatal_error: RefloatFatalErrorState,
    data_recorder: RefloatDataRecorderFlags,
}

impl RefloatRealtimeDataHeader {
    /// Build the typed realtime-data header state.
    pub const fn new(
        timestamp: TimestampTicks,
        ride_state: RefloatRideState,
        footpad_state: RefloatFootpadState,
        beep_reason: RefloatBeepReason,
    ) -> Self {
        Self {
            timestamp,
            ride_state,
            footpad_state,
            beep_reason,
            fatal_error: RefloatFatalErrorState::None,
            data_recorder: RefloatDataRecorderFlags::inactive(),
        }
    }

    /// Return this header with fatal-error state.
    pub const fn with_fatal_error(mut self, fatal_error: RefloatFatalErrorState) -> Self {
        self.fatal_error = fatal_error;
        self
    }

    /// Return this header with data-recorder flags.
    pub const fn with_data_recorder(mut self, data_recorder: RefloatDataRecorderFlags) -> Self {
        self.data_recorder = data_recorder;
        self
    }

    /// Return the typed VESC system timestamp.
    pub const fn timestamp(self) -> TimestampTicks {
        self.timestamp
    }

    /// Return the Refloat `v1.2.1` realtime data mask byte.
    pub const fn data_mask_compat(self) -> u8 {
        let runtime = match self.ride_state.run_state() {
            RefloatRunState::Running => 0x1,
            RefloatRunState::Disabled | RefloatRunState::Startup | RefloatRunState::Ready => 0,
        };
        let charging = match self.ride_state.charging() {
            RefloatChargingState::NotCharging => 0,
            RefloatChargingState::Charging => 0x2,
        };

        runtime | charging | 0x4
    }

    /// Return the Refloat `v1.2.1` realtime extra-flags byte.
    pub const fn extra_flags_compat(self) -> u8 {
        self.data_recorder.extra_flags_compat(self.fatal_error)
    }

    /// Return the Refloat `v1.2.1` realtime mode/run-state byte.
    pub const fn state_byte_compat(self) -> u8 {
        self.ride_state.mode().id() << 4 | self.ride_state.run_state().id()
    }

    /// Return the Refloat `v1.2.1` realtime footpad/ride-flags byte.
    pub const fn footpad_flags_compat(self) -> u8 {
        let charging = match self.ride_state.charging() {
            RefloatChargingState::NotCharging => 0,
            RefloatChargingState::Charging => 0x20,
        };
        let darkride = match self.ride_state.darkride() {
            RefloatDarkRideState::Upright => 0,
            RefloatDarkRideState::Active => 0x2,
        };
        let wheelslip = match self.ride_state.wheelslip() {
            RefloatWheelSlipState::None => 0,
            RefloatWheelSlipState::Detected => 0x1,
        };

        self.footpad_state.id() << 6 | charging | darkride | wheelslip
    }

    /// Return the Refloat `v1.2.1` realtime setpoint/stop byte.
    pub const fn stop_setpoint_byte_compat(self) -> u8 {
        self.ride_state.setpoint_adjustment().id() << 4 | self.ride_state.stop_condition().id()
    }

    /// Return the Refloat `v1.2.1` beep-reason byte.
    pub const fn beep_reason_compat(self) -> u8 {
        self.beep_reason.id()
    }
}
