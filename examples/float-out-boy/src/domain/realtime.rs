//! Float Out Boy realtime semantic domain types.
//!
//! Source map: realtime ID lists and payloads mirror Float Out Boy `v1.2.1` at
//! `third_party/float-out-boy/src/rt_data.h:38-66` and `third_party/float-out-boy/src/main.c:1876-1960`.

use super::{
    FloatOutBoyBeepReason, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState, FloatOutBoyFootpadSample,
    FloatOutBoyFootpadState, FloatOutBoyRideState, FloatOutBoyRunState, FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, BatteryVoltage, DirectionalMotorCurrent, DutyCycle,
    ElectricalSpeed, FirmwareFaultWireCode, ImuPitch, ImuRoll, MosfetTemperature, MotorCurrent,
    MotorTemperature, SignedRatio, TimestampTicks, VehicleSpeed,
};

/// Float Out Boy realtime-data item group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyRealtimeDataItemGroup {
    /// Always sent in realtime data.
    Always,
    /// Sent only while the board is running.
    Runtime,
}

/// Float Out Boy data-recorder policy for a realtime-data item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyRealtimeDataRecordPolicy {
    /// Send in realtime data only.
    SendOnly,
    /// Send in realtime data and record in the data recorder.
    Record,
}

macro_rules! realtime_data_items {
    (
        always { $( $always:ident => $always_id:literal, )+ }
        runtime { $( $runtime:ident => $runtime_id:literal, )+ }
        recorded { $( $recorded:ident, )+ }
    ) => {
        /// Float Out Boy realtime-data item ID.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum FloatOutBoyRealtimeDataItem {
            $( #[doc = concat!("`", $always_id, "`.")] $always, )+
            $( #[doc = concat!("`", $runtime_id, "`.")] $runtime, )+
        }

        /// Realtime-data items sent in every packet, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_DATA_ITEMS: [FloatOutBoyRealtimeDataItem; 16] =
            [$(FloatOutBoyRealtimeDataItem::$always,)+];

        /// Realtime-data items appended while running, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS: [FloatOutBoyRealtimeDataItem; 10] =
            [$(FloatOutBoyRealtimeDataItem::$runtime,)+];

        /// Realtime-data items captured by the data recorder, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS: [FloatOutBoyRealtimeDataItem; 10] =
            [$(FloatOutBoyRealtimeDataItem::$recorded,)+];

        impl FloatOutBoyRealtimeDataItem {
            /// Return the Float Out Boy `v1.2.1` realtime-data string ID.
            #[must_use]
            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$always => $always_id,)+
                    $(Self::$runtime => $runtime_id,)+
                }
            }

            /// Return the Float Out Boy `v1.2.1` realtime-data group.
            #[must_use]
            pub const fn group(self) -> FloatOutBoyRealtimeDataItemGroup {
                if matches!(self, $(Self::$runtime)|+) {
                    FloatOutBoyRealtimeDataItemGroup::Runtime
                } else {
                    FloatOutBoyRealtimeDataItemGroup::Always
                }
            }

            /// Return the Float Out Boy `v1.2.1` data-recorder policy.
            #[must_use]
            pub const fn record_policy(self) -> FloatOutBoyRealtimeDataRecordPolicy {
                if matches!(self, $(Self::$recorded)|+) {
                    FloatOutBoyRealtimeDataRecordPolicy::Record
                } else {
                    FloatOutBoyRealtimeDataRecordPolicy::SendOnly
                }
            }
        }
    };
}

// C map: order, grouping, and IDs mirror `RT_DATA_ITEMS` / `RT_DATA_RUNTIME_ITEMS` in
// `third_party/float-out-boy/src/rt_data.h:38-66`. The recorded subset follows the port's
// current data-recorder model and remains intentionally separate from the upstream lists.
realtime_data_items! {
    always {
        MotorSpeed => "motor.speed",
        MotorErpm => "motor.erpm",
        MotorCurrent => "motor.current",
        MotorDirectionalCurrent => "motor.dir_current",
        MotorFilteredCurrent => "motor.filt_current",
        MotorDutyCycle => "motor.duty_cycle",
        MotorBatteryVoltage => "motor.batt_voltage",
        MotorBatteryCurrent => "motor.batt_current",
        MotorMosfetTemperature => "motor.mosfet_temp",
        MotorTemperature => "motor.motor_temp",
        ImuPitch => "imu.pitch",
        ImuBalancePitch => "imu.balance_pitch",
        ImuRoll => "imu.roll",
        FootpadAdc1 => "footpad.adc1",
        FootpadAdc2 => "footpad.adc2",
        RemoteInput => "remote.input",
    }
    runtime {
        Setpoint => "setpoint",
        AtrSetpoint => "atr.setpoint",
        BrakeTiltSetpoint => "brake_tilt.setpoint",
        TorqueTiltSetpoint => "torque_tilt.setpoint",
        TurnTiltSetpoint => "turn_tilt.setpoint",
        RemoteSetpoint => "remote.setpoint",
        BalanceCurrent => "balance_current",
        AtrAccelDiff => "atr.accel_diff",
        AtrSpeedBoost => "atr.speed_boost",
        BoosterCurrent => "booster.current",
    }
    recorded {
        MotorErpm,
        MotorDirectionalCurrent,
        MotorDutyCycle,
        MotorBatteryVoltage,
        ImuPitch,
        ImuBalancePitch,
        Setpoint,
        AtrSetpoint,
        TorqueTiltSetpoint,
        BalanceCurrent,
    }
}

typed_newtype! {
    /// Float Out Boy `motor.filt_current` realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeFilteredMotorCurrent(DirectionalMotorCurrent);
    new(current);
    current;
}

typed_newtype! {
    /// Float Out Boy `imu.balance_pitch` realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeBalancePitch(AngleRadians);
    new(angle);
    angle;
}

impl FloatOutBoyRealtimeBalancePitch {
    /// Return the balance pitch in degrees for Float Out Boy PID and booster math.
    #[must_use]
    pub fn angle_degrees(self) -> AngleDegrees {
        AngleDegrees::from(self.0)
    }
}

typed_newtype! {
    /// Float Out Boy `remote.input` realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeRemoteInput(SignedRatio);
    new(ratio);
    ratio;
}

typed_fields! {
    /// Float Out Boy realtime motor-current values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeMotorCurrents {
        motor: MotorCurrent => motor,
        directional: DirectionalMotorCurrent => directional,
        filtered: FloatOutBoyRealtimeFilteredMotorCurrent => filtered,
        battery: BatteryCurrent => battery,
    }
}

typed_fields! {
    /// Float Out Boy realtime motor-temperature values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeMotorTemperatures {
        mosfet: MosfetTemperature => mosfet,
        motor: MotorTemperature => motor,
    }
}

typed_fields! {
    /// Float Out Boy realtime motor payload values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeMotorPayload {
        speed: VehicleSpeed => speed,
        electrical_speed: ElectricalSpeed => electrical_speed,
        currents: FloatOutBoyRealtimeMotorCurrents => currents,
        duty_cycle: DutyCycle => duty_cycle,
        battery_voltage: BatteryVoltage => battery_voltage,
        temperatures: FloatOutBoyRealtimeMotorTemperatures => temperatures,
    }
}

typed_fields! {
    /// Float Out Boy realtime IMU payload values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeImuPayload {
        pitch: ImuPitch => pitch,
        balance_pitch: FloatOutBoyRealtimeBalancePitch => balance_pitch,
        roll: ImuRoll => roll,
    }
}

typed_fields! {
    /// Float Out Boy realtime payload values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeAlwaysPayload {
        motor: FloatOutBoyRealtimeMotorPayload => motor,
        imu: FloatOutBoyRealtimeImuPayload => imu,
        footpad: FloatOutBoyFootpadSample => footpad,
        remote_input: FloatOutBoyRealtimeRemoteInput => remote_input,
    }
}

impl FloatOutBoyRealtimeAlwaysPayload {
    /// Return the source-backed item contract for this payload section.
    #[must_use]
    pub const fn item_contract(self) -> [FloatOutBoyRealtimeDataItem; 16] {
        FLOAT_OUT_BOY_REALTIME_DATA_ITEMS
    }
}

typed_newtype! {
    /// Float Out Boy runtime setpoint angle value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeRuntimeSetpoint(AngleDegrees);
    new(angle);
    angle;
}

typed_fields! {
    /// Float Out Boy runtime setpoint values sent only while running.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeRuntimeSetpoints {
        board: FloatOutBoyRealtimeRuntimeSetpoint => board,
        atr: FloatOutBoyRealtimeRuntimeSetpoint => atr,
        brake_tilt: FloatOutBoyRealtimeRuntimeSetpoint => brake_tilt,
        torque_tilt: FloatOutBoyRealtimeRuntimeSetpoint => torque_tilt,
        turn_tilt: FloatOutBoyRealtimeRuntimeSetpoint => turn_tilt,
        remote: FloatOutBoyRealtimeRuntimeSetpoint => remote,
    }
}

impl FloatOutBoyRealtimeRuntimeSetpoints {
    /// Return these runtime setpoints with a new board target.
    #[must_use]
    pub const fn with_board(mut self, board: FloatOutBoyRealtimeRuntimeSetpoint) -> Self {
        self.board = board;
        self
    }
}

typed_newtype! {
    /// Float Out Boy `balance_current` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeBalanceCurrent(MotorCurrent);
    new(current);
    current;
}

typed_newtype! {
    /// Float Out Boy `atr.accel_diff` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeAtrAccelerationDiff(f32);
    from_erpm_delta(value);
    as_erpm_delta;
}

typed_newtype! {
    /// Float Out Boy `atr.speed_boost` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeAtrSpeedBoost(f32);
    from_units(value);
    as_units;
}

typed_fields! {
    /// Float Out Boy runtime ATR payload values.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeRuntimeAtrPayload {
        accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff => accel_diff,
        speed_boost: FloatOutBoyRealtimeAtrSpeedBoost => speed_boost,
    }
}

typed_newtype! {
    /// Float Out Boy `booster.current` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeBoosterCurrent(MotorCurrent);
    new(current);
    current;
}

typed_fields! {
    /// Float Out Boy realtime payload values sent only while running.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeRuntimePayload {
        setpoints: FloatOutBoyRealtimeRuntimeSetpoints => setpoints,
        balance_current: FloatOutBoyRealtimeBalanceCurrent => balance_current,
        atr: FloatOutBoyRealtimeRuntimeAtrPayload => atr,
        booster_current: FloatOutBoyRealtimeBoosterCurrent => booster_current,
    }
}

impl FloatOutBoyRealtimeRuntimePayload {
    /// Return the source-backed item contract for this payload section.
    #[must_use]
    pub const fn item_contract(self) -> [FloatOutBoyRealtimeDataItem; 10] {
        FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS
    }
}

typed_newtype! {
    /// Float Out Boy `charging.current` realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeChargingCurrent(BatteryCurrent);
    new(current);
    current;
}

typed_newtype! {
    /// Float Out Boy `charging.voltage` realtime value.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeChargingVoltage(BatteryVoltage);
    new(voltage);
    voltage;
}

typed_fields! {
    /// Float Out Boy realtime charging payload values.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeChargingPayload {
        current: FloatOutBoyRealtimeChargingCurrent => current,
        voltage: FloatOutBoyRealtimeChargingVoltage => voltage,
    }
}

/// Float Out Boy alert ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatOutBoyAlertId {
    /// Firmware fault alert.
    FirmwareFault,
}

impl FloatOutBoyAlertId {
    /// Return the Float Out Boy `v1.2.1` alert ID.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::FirmwareFault => 1,
        }
    }

    const fn mask(self) -> u32 {
        match self {
            Self::FirmwareFault => 1,
        }
    }
}

/// Float Out Boy active-alert mask appended to realtime data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FloatOutBoyRealtimeAlertMask(u32);

impl FloatOutBoyRealtimeAlertMask {
    /// Build an empty active-alert mask.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Return a copy with the alert marked active.
    #[must_use]
    pub const fn with_alert(self, alert: FloatOutBoyAlertId) -> Self {
        Self(self.0 | alert.mask())
    }

    /// Return whether the alert is active.
    #[must_use]
    pub const fn contains(self, alert: FloatOutBoyAlertId) -> bool {
        self.0 & alert.mask() != 0
    }

    /// Return the Float Out Boy-compatible active-alert mask.
    #[must_use]
    pub const fn active_alert_mask_compat(self) -> u32 {
        self.0
    }
}

/// Float Out Boy reserved realtime tail flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FloatOutBoyRealtimeReservedFlags(u32);

impl FloatOutBoyRealtimeReservedFlags {
    /// Build the currently empty Float Out Boy realtime extra-flags field.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// Return the Float Out Boy-compatible extra-flags value.
    #[must_use]
    pub const fn extra_flags_compat(self) -> u32 {
        self.0
    }
}

typed_fields! {
    /// Float Out Boy realtime tail fields appended after conditional payload values.
    ///
    /// Source map: upstream appends active-alert mask, reserved flags, and firmware
    /// fault code at `third_party/float-out-boy/src/main.c:1956-1958`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FloatOutBoyRealtimeTail {
        active_alerts: FloatOutBoyRealtimeAlertMask => active_alerts,
        reserved_flags: FloatOutBoyRealtimeReservedFlags => reserved_flags,
        firmware_fault_code: FirmwareFaultWireCode => firmware_fault_code,
    }
}

/// Float Out Boy realtime-data header fields.
///
/// Source map: upstream `cmd_realtime_data` emits these header bytes at
/// `third_party/float-out-boy/src/main.c:1912-1941`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyRealtimeDataHeader {
    timestamp: TimestampTicks,
    ride_state: FloatOutBoyRideState,
    footpad_state: FloatOutBoyFootpadState,
    beep_reason: FloatOutBoyBeepReason,
    fatal_error: FloatOutBoyFatalErrorState,
    data_recorder: FloatOutBoyDataRecorderFlags,
}

impl FloatOutBoyRealtimeDataHeader {
    /// Build the typed realtime-data header state.
    #[must_use]
    pub const fn new(
        timestamp: TimestampTicks,
        ride_state: FloatOutBoyRideState,
        footpad_state: FloatOutBoyFootpadState,
        beep_reason: FloatOutBoyBeepReason,
    ) -> Self {
        Self {
            timestamp,
            ride_state,
            footpad_state,
            beep_reason,
            fatal_error: FloatOutBoyFatalErrorState::None,
            data_recorder: FloatOutBoyDataRecorderFlags::inactive(),
        }
    }

    /// Return this header with fatal-error state.
    #[must_use]
    pub const fn with_fatal_error(mut self, fatal_error: FloatOutBoyFatalErrorState) -> Self {
        self.fatal_error = fatal_error;
        self
    }

    /// Return this header with data-recorder flags.
    #[must_use]
    pub const fn with_data_recorder(mut self, data_recorder: FloatOutBoyDataRecorderFlags) -> Self {
        self.data_recorder = data_recorder;
        self
    }

    /// Return the typed VESC system timestamp.
    #[must_use]
    pub const fn timestamp(self) -> TimestampTicks {
        self.timestamp
    }

    /// Return the Float Out Boy `v1.2.1` realtime data mask byte.
    #[must_use]
    pub const fn data_mask_compat(self) -> u8 {
        let runtime = match self.ride_state.run_state() {
            FloatOutBoyRunState::Running => 0x1,
            FloatOutBoyRunState::Disabled
            | FloatOutBoyRunState::Startup
            | FloatOutBoyRunState::Ready => 0,
        };
        let charging = match self.ride_state.charging() {
            FloatOutBoyChargingState::NotCharging => 0,
            FloatOutBoyChargingState::Charging => 0x2,
        };

        runtime | charging | 0x4
    }

    /// Return the Float Out Boy `v1.2.1` realtime extra-flags byte.
    #[must_use]
    pub const fn extra_flags_compat(self) -> u8 {
        self.data_recorder.extra_flags_compat(self.fatal_error)
    }

    /// Return the Float Out Boy `v1.2.1` realtime mode/run-state byte.
    #[must_use]
    pub const fn state_byte_compat(self) -> u8 {
        self.ride_state.mode().id() << 4 | self.ride_state.run_state().id()
    }

    /// Return the Float Out Boy `v1.2.1` realtime footpad/ride-flags byte.
    #[must_use]
    pub const fn footpad_flags_compat(self) -> u8 {
        let charging = match self.ride_state.charging() {
            FloatOutBoyChargingState::NotCharging => 0,
            FloatOutBoyChargingState::Charging => 0x20,
        };
        let darkride = match self.ride_state.darkride() {
            FloatOutBoyDarkRideState::Upright => 0,
            FloatOutBoyDarkRideState::Active => 0x2,
        };
        let wheelslip = match self.ride_state.wheelslip() {
            FloatOutBoyWheelSlipState::None => 0,
            FloatOutBoyWheelSlipState::Detected => 0x1,
        };

        self.footpad_state.id() << 6 | charging | darkride | wheelslip
    }

    /// Return the Float Out Boy `v1.2.1` realtime setpoint/stop byte.
    #[must_use]
    pub const fn stop_setpoint_byte_compat(self) -> u8 {
        self.ride_state.setpoint_adjustment().id() << 4 | self.ride_state.stop_condition().id()
    }

    /// Return the Float Out Boy `v1.2.1` beep-reason byte.
    #[must_use]
    pub const fn beep_reason_compat(self) -> u8 {
        self.beep_reason.id()
    }
}
