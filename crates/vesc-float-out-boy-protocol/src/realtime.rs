//! Float Out Boy realtime protocol payload types.
//!
//! Source map: realtime ID lists and payloads mirror the pinned Refloat cutoff at
//! `third_party/float-out-boy/src/rt_data.h:38-66` and `third_party/float-out-boy/src/main.c:1876-1960`.

use super::{
    FloatOutBoyAllDataPayloads, FloatOutBoyBeepReason, FloatOutBoyChargingState,
    FloatOutBoyDarkRideState, FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState,
    FloatOutBoyFootpadState, FloatOutBoyRideState, FloatOutBoyRunState, FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, DirectionalMotorCurrent, FirmwareFaultWireCode,
    MosfetTemperature, MotorCurrent, MotorTemperature, MotorTorque, SampleRate, SignedRatio,
    TimestampTicks, VescSeconds,
};
use vescpkg_rs::protocol_buffer::flag_if;

macro_rules! realtime_data_items {
    (
        project($payloads:ident, $live:ident;
            $base:ident, $motor:ident, $attitude:ident, $setpoints:ident, $temperatures:ident);
        always { $( $always:ident => $always_id:literal => $always_value:expr, )+ }
        runtime { $( $runtime:ident => $runtime_id:literal => $runtime_value:expr, )+ }
        recorded { $( $recorded:ident, )+ }
    ) => {
        /// Float Out Boy realtime-data item ID.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum FloatOutBoyRealtimeDataItem {
            $( #[doc = concat!("`", $always_id, "`.")] $always, )+
            $( #[doc = concat!("`", $runtime_id, "`.")] $runtime, )+
        }

        /// Realtime-data items sent in every packet, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_DATA_ITEMS: [FloatOutBoyRealtimeDataItem; 18] =
            [$(FloatOutBoyRealtimeDataItem::$always,)+];

        /// Realtime-data items appended while running, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS: [FloatOutBoyRealtimeDataItem; 11] =
            [$(FloatOutBoyRealtimeDataItem::$runtime,)+];

        /// Realtime-data items captured by the data recorder, in source order.
        pub const FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS: [FloatOutBoyRealtimeDataItem; 13] =
            [$(FloatOutBoyRealtimeDataItem::$recorded,)+];

        impl FloatOutBoyRealtimeDataItem {
            /// Return the pinned cutoff realtime-data string ID.
            #[must_use]
            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$always => $always_id,)+
                    $(Self::$runtime => $runtime_id,)+
                }
            }
        }

        /// Project one typed FOB realtime item to its protocol float value.
        #[must_use]
        pub fn realtime_value(
            $payloads: &FloatOutBoyAllDataPayloads,
            item: FloatOutBoyRealtimeDataItem,
            $live: FloatOutBoyRealtimeLiveValues,
        ) -> f32 {
            let $base = $payloads.base();
            let $motor = $base.motor();
            let $attitude = $base.attitude();
            let $setpoints = $base.setpoints();
            let $temperatures = $payloads.mode2().temperatures();
            match item {
                $(FloatOutBoyRealtimeDataItem::$always => $always_value,)+
                $(FloatOutBoyRealtimeDataItem::$runtime => $runtime_value,)+
            }
        }
    };
}

// C map: order, grouping, and IDs mirror `RT_DATA_ITEMS` / `RT_DATA_RUNTIME_ITEMS` in
// `third_party/float-out-boy/src/rt_data.h:38-66`. The recorded subset follows the port's
// current data-recorder model and remains intentionally separate from the upstream lists.
// Projections mirror `cmd_realtime_data` at `third_party/float-out-boy/src/main.c:1943-1948`;
// motor speed stays in the km/h expected by the VESC Tool consumer.
realtime_data_items! {
    project(payloads, live;
        base, motor, attitude, setpoints, temperatures);
    always {
        ControlDt => "control.dt" => live.control_period().period().as_seconds(),
        ControlFrequency => "control.freq" => live.control_frequency().frequency().as_hertz(),
        MotorSpeed => "speed" => motor.vehicle_speed().speed().as_kilometers_per_hour(),
        MotorErpm => "erpm" => motor.electrical_speed().rpm().as_revolutions_per_minute(),
        MotorCurrent => "current" => motor.motor_current().current().as_amps(),
        MotorDirectionalCurrent => "dir_current" => motor.directional_motor_current().current().as_amps(),
        MotorFilteredCurrent => "filt_current" => motor.filtered_motor_current().current().current().as_amps(),
        MotorDutyCycle => "duty_cycle" => motor.duty_cycle().ratio().as_ratio(),
        MotorBatteryVoltage => "batt_voltage" => motor.battery_voltage().voltage().as_volts(),
        MotorBatteryCurrent => "batt_current" => motor.battery_current().current().as_amps(),
        MotorMosfetTemperature => "mosfet_temp" => temperatures.mosfet().temperature().as_degrees_celsius(),
        MotorTemperature => "motor_temp" => temperatures.motor().temperature().as_degrees_celsius(),
        ImuPitch => "pitch" => crate::degrees(attitude.pitch().angle()),
        ImuBalancePitch => "balance_pitch" => crate::degrees(attitude.balance_pitch().angle()),
        ImuRoll => "roll" => crate::degrees(attitude.roll().angle()),
        FootpadAdc1 => "adc_left" => base.footpad().adc1_volts(),
        FootpadAdc2 => "adc_right" => base.footpad().adc2_volts(),
        RemoteInput => "remote.input" => live.remote_input().ratio().as_ratio(),
    }
    runtime {
        Setpoint => "setpoint" => setpoints.board().angle().as_degrees(),
        AtrSetpoint => "atr.setpoint" => setpoints.atr().angle().as_degrees(),
        BrakeTiltSetpoint => "brake_tilt.setpoint" => setpoints.brake_tilt().angle().as_degrees(),
        TorqueTiltSetpoint => "torque_tilt.setpoint" => setpoints.torque_tilt().angle().as_degrees(),
        TurnTiltSetpoint => "turn_tilt.setpoint" => setpoints.turn_tilt().angle().as_degrees(),
        RemoteSetpoint => "remote.setpoint" => setpoints.remote().angle().as_degrees(),
        BalanceCurrent => "balance_current" => base.balance_current().current().current().as_amps(),
        AtrAccelDiff => "atr.accel_diff" => live.atr_accel_diff().as_erpm_delta(),
        AtrSpeedBoost => "atr.speed_boost" => live.atr_speed_boost().as_units(),
        AtrTransitionBoost => "atr.transition_boost" => live.atr_transition_boost().factor(),
        BoosterTorque => "booster.torque" => base.booster_torque().torque().as_newton_meters(),
    }
    recorded {
        ControlDt,
        ControlFrequency,
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
        AtrTransitionBoost,
    }
}

vescpkg_rs::typed_newtypes! {
    attributes {
        #[derive(Debug, Default, Clone, Copy, PartialEq)]
        #[repr(transparent)]
    }
    /// Float Out Boy `motor.filt_current` realtime value.
    pub struct FloatOutBoyRealtimeFilteredMotorCurrent(DirectionalMotorCurrent) => new(current), current;
    /// Float Out Boy `imu.balance_pitch` realtime value.
    pub struct FloatOutBoyRealtimeBalancePitch(AngleRadians) => new(angle), angle;
    /// Float Out Boy `remote.input` realtime value.
    pub struct FloatOutBoyRealtimeRemoteInput(SignedRatio) => new(ratio), ratio;
    /// Float Out Boy runtime setpoint angle value.
    pub struct FloatOutBoyRealtimeRuntimeSetpoint(AngleDegrees) => new(angle), angle;
    /// Float Out Boy `balance_current` runtime realtime value.
    pub struct FloatOutBoyRealtimeBalanceCurrent(MotorCurrent) => new(current), current;
    /// Pinned Refloat cutoff `booster.torque` runtime realtime value.
    pub struct FloatOutBoyRealtimeBoosterTorque(MotorTorque) => new(torque), torque;
    /// Live cutoff ATR acceleration-difference value in ERPM delta units.
    pub struct FloatOutBoyRealtimeAtrAccelerationDiff(f32) => from_erpm_delta(value), as_erpm_delta;
    /// Live cutoff ATR speed-boost value.
    pub struct FloatOutBoyRealtimeAtrSpeedBoost(f32) => from_units(value), as_units;
    /// Measured cutoff control-loop period.
    pub struct FloatOutBoyRealtimeControlPeriod(VescSeconds) => new(period), period;
    /// Measured cutoff control-loop frequency.
    pub struct FloatOutBoyRealtimeControlFrequency(SampleRate) => new(frequency), frequency;
    /// Live cutoff ATR transition multiplier.
    pub struct FloatOutBoyRealtimeAtrTransitionBoost(f32) => from_factor(factor), factor;
}

vescpkg_rs::typed_field_groups! {
    attributes { #[derive(Debug, Clone, Copy, PartialEq)] }
    /// Float Out Boy realtime motor-current values that are always sent.
    #[derive(Default)]
    pub struct FloatOutBoyRealtimeMotorCurrents {
        motor: MotorCurrent => motor,
        directional: DirectionalMotorCurrent => directional,
        filtered: FloatOutBoyRealtimeFilteredMotorCurrent => filtered,
        battery: BatteryCurrent => battery,
    }
    /// Float Out Boy realtime motor-temperature values that are always sent.
    pub struct FloatOutBoyRealtimeMotorTemperatures {
        mosfet: MosfetTemperature => mosfet,
        motor: MotorTemperature => motor,
    }
    /// Float Out Boy runtime setpoint values sent only while running.
    #[derive(Default)]
    pub struct FloatOutBoyRealtimeRuntimeSetpoints {
        board: FloatOutBoyRealtimeRuntimeSetpoint => board => with_board,
        atr: FloatOutBoyRealtimeRuntimeSetpoint => atr,
        brake_tilt: FloatOutBoyRealtimeRuntimeSetpoint => brake_tilt,
        torque_tilt: FloatOutBoyRealtimeRuntimeSetpoint => torque_tilt,
        turn_tilt: FloatOutBoyRealtimeRuntimeSetpoint => turn_tilt,
        remote: FloatOutBoyRealtimeRuntimeSetpoint => remote,
    }
    /// Live values outside the cached all-data payload.
    pub struct FloatOutBoyRealtimeLiveValues {
        control_period: FloatOutBoyRealtimeControlPeriod => control_period,
        control_frequency: FloatOutBoyRealtimeControlFrequency => control_frequency,
        remote_input: FloatOutBoyRealtimeRemoteInput => remote_input,
        atr_accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff => atr_accel_diff,
        atr_speed_boost: FloatOutBoyRealtimeAtrSpeedBoost => atr_speed_boost,
        atr_transition_boost: FloatOutBoyRealtimeAtrTransitionBoost => atr_transition_boost,
    }
    /// Float Out Boy realtime tail fields appended after conditional payload values.
    ///
    /// Source map: upstream appends active-alert mask, reserved flags, and firmware
    /// fault code at `third_party/float-out-boy/src/main.c:1956-1958`.
    #[derive(Eq)]
    pub struct FloatOutBoyRealtimeTail {
        firmware_fault_active: bool => firmware_fault_active,
        firmware_fault_code: FirmwareFaultWireCode => firmware_fault_code,
    }
}

impl FloatOutBoyRealtimeBalancePitch {
    /// Return the balance pitch in degrees for Float Out Boy PID and booster math.
    #[must_use]
    pub fn angle_degrees(self) -> AngleDegrees {
        AngleDegrees::from(self.0)
    }
}

impl Default for FloatOutBoyRealtimeMotorTemperatures {
    fn default() -> Self {
        let zero = vescpkg_rs::Temperature::from_degrees_celsius(0.0);
        Self::new(MosfetTemperature::new(zero), MotorTemperature::new(zero))
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
        flag_if(
            matches!(self.ride_state.run_state(), FloatOutBoyRunState::Running),
            0x1,
        ) | flag_if(
            matches!(
                self.ride_state.charging(),
                FloatOutBoyChargingState::Charging
            ),
            0x2,
        ) | 0x4
    }

    /// Return the cutoff internal-realtime extra-flags byte.
    #[must_use]
    pub const fn extra_flags_compat(self) -> u8 {
        self.data_recorder.realtime_extra_flags_compat()
    }

    /// Return the cutoff internal-realtime packed state flags.
    #[must_use]
    pub fn state_flags_compat(self) -> u32 {
        let charging = match self.ride_state.charging() {
            FloatOutBoyChargingState::NotCharging => 0,
            FloatOutBoyChargingState::Charging => 1,
        };
        let fatal_error = match self.fatal_error {
            FloatOutBoyFatalErrorState::None => 0,
            FloatOutBoyFatalErrorState::Present => 1,
        };
        let darkride = match self.ride_state.darkride() {
            FloatOutBoyDarkRideState::Upright => 0,
            FloatOutBoyDarkRideState::Active => 1,
        };
        let wheelslip = match self.ride_state.wheelslip() {
            FloatOutBoyWheelSlipState::None => 0,
            FloatOutBoyWheelSlipState::Detected => 1,
        };
        u32::from(self.ride_state.mode().id()) << 28
            | u32::from(self.ride_state.run_state().id()) << 24
            | u32::from(self.footpad_state.id()) << 22
            | charging << 21
            | fatal_error << 20
            | darkride << 17
            | wheelslip << 16
            | u32::from(self.ride_state.setpoint_adjustment().id()) << 12
            | u32::from(self.ride_state.stop_condition().id()) << 8
            | u32::from(self.beep_reason.id())
    }
}
