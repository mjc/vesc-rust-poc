//! VESC-domain semantic types over generic embedded units.
//!
//! Units such as amps and volts are generic. This module adds the VESC meaning:
//! motor current is not battery current, input voltage is not an arbitrary
//! voltage, and duty cycle is a controller command ratio.
//!
//! ```compile_fail
//! use vescpkg_rs::types::{BatteryCurrent, MotorCurrent};
//! use vescpkg_rs::units::Current;
//!
//! fn set_motor_current(_: MotorCurrent) {}
//!
//! let battery = BatteryCurrent::new(Current::from_amps(10.0));
//! set_motor_current(battery);
//! ```
//!
//! Raw token wrappers also require explicit extraction:
//!
//! ```compile_fail
//! use vescpkg_rs::types::CanControllerId;
//!
//! let id = CanControllerId::new(42);
//! let _: u8 = id.into();
//! ```

pub mod adc;
pub mod battery;
pub mod can;
pub mod config;
pub mod gnss;
pub mod imu;
pub mod io;
pub mod motion;
pub mod motor;
pub mod power;
pub mod ratio;
pub mod temperature;
pub mod time;

pub use adc::{AdcDecodedLevel, AdcVoltage};
pub use battery::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, BatteryVoltage, CellVoltage,
    InputCurrent, InputVoltage, WattHoursCharged, WattHoursDischarged, WattHoursRemaining,
};
pub use can::{CanControllerId, CanExtendedId, CanPayloadLen, CanPayloadLenError, CanStandardId};
pub use config::{
    BatteryCellCount, BatteryCellCountError, FocMotorFluxLinkage, FocMotorInductance,
    FocMotorResistance, GearRatio, GearRatioError, MotorPoleCount, MotorPoleCountError,
    WheelDiameter,
};
pub use gnss::{GnssAltitude, GnssLatitude, GnssLongitude, GnssSpeed};
pub use imu::{ImuAcceleration, ImuAngularRate, ImuPitch, ImuQuaternion, ImuRoll, ImuYaw};
pub use io::{
    BaudRate, BaudRateError, PacketLength, PacketLengthError, ThreadPriority, ThreadPriorityError,
};
pub use motion::{
    AbsoluteTachometerSteps, ElectricalSpeed, MechanicalSpeed, TachometerSteps, TripDistance,
    VehicleSpeed,
};
pub use motor::{
    AudioVoltage, BrakeCurrent, DCurrent, DVoltage, DirectionalMotorCurrent, HandbrakeCurrent,
    MotorCurrent, OpenLoopCurrent, PhaseCurrent, QCurrent, QVoltage, TotalMotorCurrent,
};
pub use power::{AveragePower, PeakPower};
pub use ratio::{
    BrakeCurrentRelative, CurrentRelative, DutyCycle, HandbrakeRelative, JoystickX, JoystickY,
    PpmInput,
};
pub use temperature::{
    FetTemperature, MosfetTemperature, MotorTemperature, TemperatureLimitEnd, TemperatureLimitStart,
};
pub use time::{PpmAge, RemoteAge, SystemDuration, SystemTimestamp, TimeoutDuration};
