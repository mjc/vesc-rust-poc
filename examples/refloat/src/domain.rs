//! Refloat-specific ride-domain types.
//!
//! These types compose the reusable `vescpkg-rs` package-author units and
//! semantic wrappers into Refloat concepts. Raw firmware/app-data primitives
//! should stay at explicit boundary conversions.

use vescpkg_rs::prelude::{
    AdcDecodedLevel, BatteryCurrent, BatteryVoltage, DirectionalMotorCurrent, DutyCycle,
    ElectricalSpeed, ImuAngularRate, ImuPitch, ImuRoll, ImuYaw, MotorCurrent, VehicleSpeed,
};

/// Refloat footpad sensor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FootpadSensorState {
    /// No footpad sensor is active.
    None,
    /// Left footpad sensor is active.
    Left,
    /// Right footpad sensor is active.
    Right,
    /// Both footpad sensors are active.
    Both,
}

impl FootpadSensorState {
    /// Return the Refloat app-data switch compatibility value.
    pub const fn switch_compat(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left | Self::Right => 1,
            Self::Both => 2,
        }
    }
}

/// Refloat footpad ADC sample and decoded state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootpadSensorSample {
    adc1: AdcDecodedLevel,
    adc2: AdcDecodedLevel,
    state: FootpadSensorState,
}

impl FootpadSensorSample {
    /// Build a footpad sensor sample from typed ADC levels and decoded state.
    pub const fn new(
        adc1: AdcDecodedLevel,
        adc2: AdcDecodedLevel,
        state: FootpadSensorState,
    ) -> Self {
        Self { adc1, adc2, state }
    }

    /// Return the typed ADC1 level.
    pub const fn adc1(self) -> AdcDecodedLevel {
        self.adc1
    }

    /// Return the typed ADC2 level.
    pub const fn adc2(self) -> AdcDecodedLevel {
        self.adc2
    }

    /// Return the decoded footpad sensor state.
    pub const fn state(self) -> FootpadSensorState {
        self.state
    }
}

/// Refloat top-level run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatRunState {
    /// Package is disabled.
    Disabled,
    /// Package is starting up.
    Startup,
    /// Package is ready but not actively balancing.
    Ready,
    /// Package is actively running.
    Running,
}

/// Refloat runtime mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatMode {
    /// Normal ride mode.
    Normal,
    /// Hand-test mode.
    HandTest,
    /// Flywheel mode.
    Flywheel,
}

/// Refloat stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatStopCondition {
    /// No stop condition is active.
    None,
    /// Pitch angle fault.
    Pitch,
    /// Roll angle fault.
    Roll,
    /// Half-switch fault.
    SwitchHalf,
    /// Full-switch fault.
    SwitchFull,
    /// Reverse-stop fault.
    ReverseStop,
    /// Quickstop fault.
    QuickStop,
}

/// Refloat setpoint adjustment or pushback reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatSetpointAdjustment {
    /// No adjustment.
    None,
    /// Centering adjustment.
    Centering,
    /// Reverse-stop adjustment.
    ReverseStop,
    /// Pushback from speed limit.
    PushbackSpeed,
    /// Pushback from duty limit.
    PushbackDuty,
    /// Pushback from error state.
    PushbackError,
    /// Pushback from high voltage.
    PushbackHighVoltage,
    /// Pushback from low voltage.
    PushbackLowVoltage,
    /// Pushback from temperature.
    PushbackTemperature,
}

impl RefloatSetpointAdjustment {
    const fn is_float_state_tiltback(self) -> bool {
        matches!(
            self,
            Self::PushbackError
                | Self::PushbackHighVoltage
                | Self::PushbackLowVoltage
                | Self::PushbackTemperature
        )
    }
}

/// Refloat charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatChargingState {
    /// Not charging.
    NotCharging,
    /// Charging is active.
    Charging,
}

/// Refloat wheel-slip state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatWheelSlipState {
    /// No wheel slip detected.
    None,
    /// Wheel slip detected.
    Detected,
}

/// Refloat darkride/upside-down state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefloatDarkRideState {
    /// Board is upright.
    Upright,
    /// Darkride/upside-down state is active.
    Active,
}

/// Refloat ride state as typed package-domain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRideState {
    run_state: RefloatRunState,
    mode: RefloatMode,
    setpoint_adjustment: RefloatSetpointAdjustment,
    stop_condition: RefloatStopCondition,
    charging: RefloatChargingState,
    wheelslip: RefloatWheelSlipState,
    darkride: RefloatDarkRideState,
}

impl RefloatRideState {
    /// Build a Refloat ride state from required enum-shaped state.
    pub const fn new(
        run_state: RefloatRunState,
        mode: RefloatMode,
        setpoint_adjustment: RefloatSetpointAdjustment,
        stop_condition: RefloatStopCondition,
    ) -> Self {
        Self {
            run_state,
            mode,
            setpoint_adjustment,
            stop_condition,
            charging: RefloatChargingState::NotCharging,
            wheelslip: RefloatWheelSlipState::None,
            darkride: RefloatDarkRideState::Upright,
        }
    }

    /// Return this state with the requested charging state.
    pub const fn with_charging(mut self, charging: RefloatChargingState) -> Self {
        self.charging = charging;
        self
    }

    /// Return this state with the requested wheel-slip state.
    pub const fn with_wheelslip(mut self, wheelslip: RefloatWheelSlipState) -> Self {
        self.wheelslip = wheelslip;
        self
    }

    /// Return this state with the requested darkride/upside-down state.
    pub const fn with_darkride(mut self, darkride: RefloatDarkRideState) -> Self {
        self.darkride = darkride;
        self
    }

    /// Return the Refloat app-data Float State compatibility value.
    pub const fn float_state_compat(self) -> u8 {
        if matches!(self.charging, RefloatChargingState::Charging) {
            return 14;
        }

        match self.run_state {
            RefloatRunState::Disabled => 15,
            RefloatRunState::Startup => 0,
            RefloatRunState::Ready => self.ready_float_state_compat(),
            RefloatRunState::Running => self.running_float_state_compat(),
        }
    }

    /// Return the Refloat app-data setpoint-adjustment compatibility value.
    pub const fn setpoint_adjustment_compat(self) -> u8 {
        match self.setpoint_adjustment {
            RefloatSetpointAdjustment::Centering => 0,
            RefloatSetpointAdjustment::ReverseStop => 1,
            RefloatSetpointAdjustment::None => 2,
            RefloatSetpointAdjustment::PushbackDuty => 3,
            RefloatSetpointAdjustment::PushbackHighVoltage => 4,
            RefloatSetpointAdjustment::PushbackLowVoltage => 5,
            RefloatSetpointAdjustment::PushbackTemperature => 6,
            RefloatSetpointAdjustment::PushbackSpeed => 7,
            RefloatSetpointAdjustment::PushbackError => 8,
        }
    }

    const fn ready_float_state_compat(self) -> u8 {
        match self.stop_condition {
            RefloatStopCondition::None => 11,
            RefloatStopCondition::Pitch => 6,
            RefloatStopCondition::Roll => 7,
            RefloatStopCondition::SwitchHalf => 8,
            RefloatStopCondition::SwitchFull => 9,
            RefloatStopCondition::ReverseStop => 12,
            RefloatStopCondition::QuickStop => 13,
        }
    }

    const fn running_float_state_compat(self) -> u8 {
        if self.setpoint_adjustment.is_float_state_tiltback() {
            2
        } else if matches!(self.wheelslip, RefloatWheelSlipState::Detected) {
            3
        } else if matches!(self.darkride, RefloatDarkRideState::Active) {
            4
        } else if matches!(self.mode, RefloatMode::Flywheel) {
            5
        } else {
            1
        }
    }
}

/// Refloat IMU sample used by ride logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatImuSample {
    pitch: ImuPitch,
    roll: ImuRoll,
    yaw: ImuYaw,
    angular_rate: ImuAngularRate,
}

impl RefloatImuSample {
    /// Build a typed Refloat IMU sample.
    pub const fn new(
        pitch: ImuPitch,
        roll: ImuRoll,
        yaw: ImuYaw,
        angular_rate: ImuAngularRate,
    ) -> Self {
        Self {
            pitch,
            roll,
            yaw,
            angular_rate,
        }
    }

    /// Return typed pitch.
    pub const fn pitch(self) -> ImuPitch {
        self.pitch
    }

    /// Return typed roll.
    pub const fn roll(self) -> ImuRoll {
        self.roll
    }

    /// Return typed yaw.
    pub const fn yaw(self) -> ImuYaw {
        self.yaw
    }

    /// Return typed angular-rate axes.
    pub const fn angular_rate(self) -> ImuAngularRate {
        self.angular_rate
    }
}

/// Refloat motor telemetry sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatMotorTelemetry {
    electrical_speed: ElectricalSpeed,
    vehicle_speed: VehicleSpeed,
    motor_current: DirectionalMotorCurrent,
    battery_current: BatteryCurrent,
    duty_cycle: DutyCycle,
    battery_voltage: BatteryVoltage,
}

impl RefloatMotorTelemetry {
    /// Build a typed Refloat motor telemetry sample.
    pub const fn new(
        electrical_speed: ElectricalSpeed,
        vehicle_speed: VehicleSpeed,
        motor_current: DirectionalMotorCurrent,
        battery_current: BatteryCurrent,
        duty_cycle: DutyCycle,
        battery_voltage: BatteryVoltage,
    ) -> Self {
        Self {
            electrical_speed,
            vehicle_speed,
            motor_current,
            battery_current,
            duty_cycle,
            battery_voltage,
        }
    }

    /// Return typed electrical speed.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return typed vehicle speed.
    pub const fn vehicle_speed(self) -> VehicleSpeed {
        self.vehicle_speed
    }

    /// Return typed directional motor current.
    pub const fn motor_current(self) -> DirectionalMotorCurrent {
        self.motor_current
    }

    /// Return typed battery current.
    pub const fn battery_current(self) -> BatteryCurrent {
        self.battery_current
    }

    /// Return typed duty cycle.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }

    /// Return typed battery voltage.
    pub const fn battery_voltage(self) -> BatteryVoltage {
        self.battery_voltage
    }
}

/// Refloat motor-current request.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct RefloatMotorCommand {
    requested_current: MotorCurrent,
}

impl RefloatMotorCommand {
    /// Build a motor command from typed requested current.
    pub const fn new(requested_current: MotorCurrent) -> Self {
        Self { requested_current }
    }

    /// Return the typed requested current.
    pub const fn requested_current(self) -> MotorCurrent {
        self.requested_current
    }
}
