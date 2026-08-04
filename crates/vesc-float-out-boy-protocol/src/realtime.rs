//! Float Out Boy realtime protocol payload types.
//!
//! Source map: realtime ID lists and payloads mirror Float Out Boy `v1.2.1` at
//! `third_party/float-out-boy/src/rt_data.h:38-66` and `third_party/float-out-boy/src/main.c:1876-1960`.

use super::{
    FloatOutBoyBeepReason, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState, FloatOutBoyFootpadState,
    FloatOutBoyRideState, FloatOutBoyRunState, FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, DirectionalMotorCurrent, FirmwareFaultWireCode,
    MosfetTemperature, MotorCurrent, MotorTemperature, SignedRatio, TimestampTicks,
};

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
        pub const FLOAT_OUT_BOY_REALTIME_DATA_ITEMS:
            [FloatOutBoyRealtimeDataItem; [$(FloatOutBoyRealtimeDataItem::$always,)+].len()] =
            [$(FloatOutBoyRealtimeDataItem::$always,)+];

        /// Realtime-data items appended while running, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS:
            [FloatOutBoyRealtimeDataItem; [$(FloatOutBoyRealtimeDataItem::$runtime,)+].len()] =
            [$(FloatOutBoyRealtimeDataItem::$runtime,)+];

        /// Realtime-data items captured by the data recorder, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS:
            [FloatOutBoyRealtimeDataItem; [$(FloatOutBoyRealtimeDataItem::$recorded,)+].len()] =
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

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `motor.filt_current` realtime value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeFilteredMotorCurrent(DirectionalMotorCurrent);
    new(current);
    current;
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `imu.balance_pitch` realtime value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
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

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `remote.input` realtime value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeRemoteInput(SignedRatio);
    new(ratio);
    ratio;
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy realtime motor-current values that are always sent.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeMotorCurrents {
        motor: MotorCurrent => motor,
        directional: DirectionalMotorCurrent => directional,
        filtered: FloatOutBoyRealtimeFilteredMotorCurrent => filtered,
        battery: BatteryCurrent => battery,
    }
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy realtime motor-temperature values that are always sent.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeMotorTemperatures {
        mosfet: MosfetTemperature => mosfet,
        motor: MotorTemperature => motor,
    }
}

impl Default for FloatOutBoyRealtimeMotorTemperatures {
    fn default() -> Self {
        let zero = vescpkg_rs::Temperature::from_degrees_celsius(0.0);
        Self::new(MosfetTemperature::new(zero), MotorTemperature::new(zero))
    }
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy runtime setpoint angle value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeRuntimeSetpoint(AngleDegrees);
    new(angle);
    angle;
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy runtime setpoint values sent only while running.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct FloatOutBoyRealtimeRuntimeSetpoints {
        board: FloatOutBoyRealtimeRuntimeSetpoint => board => with_board,
        atr: FloatOutBoyRealtimeRuntimeSetpoint => atr,
        brake_tilt: FloatOutBoyRealtimeRuntimeSetpoint => brake_tilt,
        torque_tilt: FloatOutBoyRealtimeRuntimeSetpoint => torque_tilt,
        turn_tilt: FloatOutBoyRealtimeRuntimeSetpoint => turn_tilt,
        remote: FloatOutBoyRealtimeRuntimeSetpoint => remote,
    }
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `balance_current` runtime realtime value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeBalanceCurrent(MotorCurrent);
    new(current);
    current;
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `booster.current` runtime realtime value.
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeBoosterCurrent(MotorCurrent);
    new(current);
    current;
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `atr.accel_diff` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeAtrAccelerationDiff(f32);
    from_erpm_delta(value);
    as_erpm_delta;
}

vescpkg_rs::typed_newtype! {
    /// Float Out Boy `atr.speed_boost` runtime realtime value.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub struct FloatOutBoyRealtimeAtrSpeedBoost(f32);
    from_units(value);
    as_units;
}

vescpkg_rs::typed_fields! {
    /// Float Out Boy realtime tail fields appended after conditional payload values.
    ///
    /// Source map: upstream appends active-alert mask, reserved flags, and firmware
    /// fault code at `third_party/float-out-boy/src/main.c:1956-1958`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FloatOutBoyRealtimeTail {
        firmware_fault_active: bool => firmware_fault_active,
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
            data_recorder: FloatOutBoyDataRecorderFlags::empty(),
        }
    }

    vescpkg_rs::const_field_builders! {
        /// Return this header with fatal-error state.
        pub fn with_fatal_error(fatal_error: FloatOutBoyFatalErrorState) => fatal_error;
        /// Return this header with data-recorder flags.
        pub fn with_data_recorder(data_recorder: FloatOutBoyDataRecorderFlags) => data_recorder;
    }

    vescpkg_rs::const_field_getters! {
        /// Return the typed VESC system timestamp.
        pub fn timestamp -> TimestampTicks = timestamp;
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
