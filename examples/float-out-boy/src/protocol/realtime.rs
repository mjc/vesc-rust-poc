//! Realtime protocol payload types owned by Float Out Boy.
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

/// Numeric width selected by command 33 control bit zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyRealtimePrecision {
    /// Encode selected numeric fields with the 16-bit float codec.
    Float16,
    /// Encode selected numeric fields with the 32-bit float codec.
    Float32,
}

vescpkg_rs::typed_newtypes! {
    attributes { #[derive(Debug, Clone, Copy, PartialEq, Eq)] }
    /// Opaque command 33 control byte.
    pub struct FloatOutBoyRealtimeControlFlags(u8) => from_wire(wire_value), wire_value;
    /// First command 33 selection mask.
    pub struct FloatOutBoyRealtimeMask1(u32) => from_wire(wire_value), wire_value;
    /// Second command 33 selection mask.
    pub struct FloatOutBoyRealtimeMask2(u32) => from_wire(wire_value), wire_value;
}

impl FloatOutBoyRealtimeControlFlags {
    /// Return the selected numeric precision.
    #[must_use]
    pub const fn precision(self) -> FloatOutBoyRealtimePrecision {
        if self.wire_value() & 1 == 0 {
            FloatOutBoyRealtimePrecision::Float16
        } else {
            FloatOutBoyRealtimePrecision::Float32
        }
    }
}

impl FloatOutBoyRealtimeMask1 {
    pub(crate) const fn selects(self, bit: u32) -> bool {
        self.wire_value() & bit != 0
    }
}

impl FloatOutBoyRealtimeMask2 {
    const GNSS_FIELDS: u32 = 0x0000_7e00;

    pub(crate) const fn selects(self, bit: u32) -> bool {
        self.wire_value() & bit != 0
    }

    /// Return whether any selected field needs a GNSS snapshot.
    #[must_use]
    pub const fn selects_gnss(self) -> bool {
        self.wire_value() & Self::GNSS_FIELDS != 0
    }
}

/// Parsed command 33 payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyRealtimeSelectedRequest {
    control_flags: FloatOutBoyRealtimeControlFlags,
    mask1: FloatOutBoyRealtimeMask1,
    mask2: FloatOutBoyRealtimeMask2,
}

impl FloatOutBoyRealtimeSelectedRequest {
    /// Parse the cutoff command 33 request shape.
    ///
    /// A partial second mask is ignored; all four bytes must be present.
    #[must_use]
    pub fn parse(payload: &[u8]) -> Option<Self> {
        let [flags, a, b, c, d, ..] = payload else {
            return None;
        };
        let mask2 = payload
            .get(5..9)
            .and_then(|bytes| <&[u8; 4]>::try_from(bytes).ok())
            .map_or(0, |bytes| u32::from_be_bytes(*bytes));
        Some(Self {
            control_flags: FloatOutBoyRealtimeControlFlags::from_wire(*flags),
            mask1: FloatOutBoyRealtimeMask1::from_wire(u32::from_be_bytes([*a, *b, *c, *d])),
            mask2: FloatOutBoyRealtimeMask2::from_wire(mask2),
        })
    }

    vescpkg_rs::const_field_getters! {
        /// Return the opaque control byte.
        pub fn control_flags -> FloatOutBoyRealtimeControlFlags = control_flags;
        /// Return selection mask one.
        pub fn mask1 -> FloatOutBoyRealtimeMask1 = mask1;
        /// Return selection mask two.
        pub fn mask2 -> FloatOutBoyRealtimeMask2 = mask2;
    }
}

macro_rules! realtime_data_items {
    (
        project($payloads:ident, $live:ident);
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
    project(payloads, live);
    always {
        ControlDt => "control.dt" => live.control_period().period().as_seconds(),
        ControlFrequency => "control.freq" => live.control_frequency().frequency().as_hertz(),
        MotorSpeed => "speed" => payloads.vehicle_speed().speed().as_kilometers_per_hour(),
        MotorErpm => "erpm" => payloads.electrical_speed().rpm().as_revolutions_per_minute(),
        MotorCurrent => "current" => payloads.motor_current().current().as_amps(),
        MotorDirectionalCurrent => "dir_current" => payloads.directional_motor_current().current().as_amps(),
        MotorFilteredCurrent => "filt_current" => payloads.filtered_motor_current().current().current().as_amps(),
        MotorDutyCycle => "duty_cycle" => payloads.duty_cycle().ratio().as_ratio(),
        MotorBatteryVoltage => "batt_voltage" => payloads.motor_battery_voltage().voltage().as_volts(),
        MotorBatteryCurrent => "batt_current" => payloads.battery_current().current().as_amps(),
        MotorMosfetTemperature => "mosfet_temp" => payloads.temperatures().mosfet().temperature().as_degrees_celsius(),
        MotorTemperature => "motor_temp" => payloads.temperatures().motor().temperature().as_degrees_celsius(),
        ImuPitch => "pitch" => super::degrees(payloads.pitch().angle()),
        ImuBalancePitch => "balance_pitch" => super::degrees(payloads.balance_pitch().angle()),
        ImuRoll => "roll" => super::degrees(payloads.roll().angle()),
        FootpadAdc1 => "adc_left" => payloads.footpad().left_voltage().as_volts(),
        FootpadAdc2 => "adc_right" => payloads.footpad().right_voltage().as_volts(),
        RemoteInput => "remote.input" => live.remote_input().ratio().as_ratio(),
    }
    runtime {
        Setpoint => "setpoint" => payloads.setpoints().board().angle().as_degrees(),
        AtrSetpoint => "atr.setpoint" => payloads.setpoints().atr().angle().as_degrees(),
        BrakeTiltSetpoint => "brake_tilt.setpoint" => payloads.setpoints().brake_tilt().angle().as_degrees(),
        TorqueTiltSetpoint => "torque_tilt.setpoint" => payloads.setpoints().torque_tilt().angle().as_degrees(),
        TurnTiltSetpoint => "turn_tilt.setpoint" => payloads.setpoints().turn_tilt().angle().as_degrees(),
        RemoteSetpoint => "remote.setpoint" => payloads.setpoints().remote().angle().as_degrees(),
        BalanceCurrent => "balance_current" => payloads.balance_current().current().current().as_amps(),
        AtrAccelDiff => "atr.accel_diff" => live.atr_accel_diff().as_erpm_delta(),
        AtrSpeedBoost => "atr.speed_boost" => live.atr_speed_boost().as_units(),
        AtrTransitionBoost => "atr.transition_boost" => live.atr_transition_boost().factor(),
        BoosterTorque => "booster.torque" => payloads.booster_torque().torque().as_newton_meters(),
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

#[cfg(test)]
mod selected_request_tests {
    use super::{FloatOutBoyRealtimePrecision, FloatOutBoyRealtimeSelectedRequest};

    #[test]
    fn rejects_truncated_required_header() {
        for len in 0..5 {
            assert!(FloatOutBoyRealtimeSelectedRequest::parse(&[0; 4][..len]).is_none());
        }
    }

    #[test]
    fn reads_required_fields_and_ignores_partial_second_mask() {
        let request =
            FloatOutBoyRealtimeSelectedRequest::parse(&[0x80, 1, 2, 3, 4, 0xaa]).expect("request");
        assert_eq!(request.control_flags().wire_value(), 0x80);
        assert_eq!(
            request.control_flags().precision(),
            FloatOutBoyRealtimePrecision::Float16
        );
        assert_eq!(request.mask1().wire_value(), 0x0102_0304);
        assert_eq!(request.mask2().wire_value(), 0);
    }

    #[test]
    fn reads_complete_second_mask_and_ignores_trailing_bytes() {
        let request = FloatOutBoyRealtimeSelectedRequest::parse(&[
            1, 0, 0, 0, 0, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        ])
        .expect("request");
        assert_eq!(
            request.control_flags().precision(),
            FloatOutBoyRealtimePrecision::Float32
        );
        assert_eq!(request.mask2().wire_value(), 0xaabb_ccdd);
    }

    #[test]
    fn detects_gnss_selection() {
        let no_gnss = FloatOutBoyRealtimeSelectedRequest::parse(&[0, 0, 0, 0, 0, 0, 0, 1, 0])
            .expect("request");
        let gnss = FloatOutBoyRealtimeSelectedRequest::parse(&[0, 0, 0, 0, 0, 0, 0, 2, 0])
            .expect("request");
        assert!(!no_gnss.mask2().selects_gnss());
        assert!(gnss.mask2().selects_gnss());
    }
}
