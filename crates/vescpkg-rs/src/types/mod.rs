//! VESC-domain semantic types over generic embedded units.
//!
//! Units such as amps and volts are generic. This module adds the VESC meaning:
//! motor current is not battery current, input voltage is not an arbitrary
//! voltage, and duty cycle is a controller command ratio.
//!
//! ```compile_fail
//! use vescpkg_rs::{BatteryCurrent, Current, MotorCurrent};
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
//! use vescpkg_rs::CanControllerId;
//!
//! let id = CanControllerId::new(42);
//! let _: u8 = id.into();
//! ```

mod adc;
mod battery;
mod can;
mod config;
mod control;
mod gnss;
mod imu;
mod io;
pub(crate) mod loader;
mod motion;
mod motor;
mod power;
mod ratio;
mod temperature;
mod time;
mod version;

pub use adc::{AdcDecodedLevel, AdcVoltage, BrakeLeverLevel, BrakeSwitch};
pub use battery::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, BatteryVoltage, CellVoltage,
    InputCurrent, InputVoltage, WattHoursCharged, WattHoursDischarged, WattHoursRemaining,
};
pub use can::{CanControllerId, CanExtendedId, CanPayloadLen, CanPayloadLenError, CanStandardId};
pub use config::{
    BatteryCellCount, BatteryCellCountError, CustomConfigAngleCurrentGainField,
    CustomConfigAngleField, CustomConfigAngularVelocityField, CustomConfigDurationField,
    CustomConfigEditor, CustomConfigElectricalSpeedField, CustomConfigEnumField,
    CustomConfigFlagField, CustomConfigFrequencyField, CustomConfigImage,
    CustomConfigIntegralCurrentGainField, CustomConfigMahonyPitchGainField,
    CustomConfigMahonyRollGainField, CustomConfigMotorCurrentField, CustomConfigPidScaleField,
    CustomConfigRateCurrentGainField, CustomConfigRatioField, CustomConfigResetField,
    CustomConfigSampleRateField, CustomConfigScaledVoltageField, CustomConfigSecondsField,
    CustomConfigVoltageField, FocMotorFluxLinkage, FocMotorInductance, FocMotorResistance,
    GearRatio, GearRatioError, MotorPoleCount, MotorPoleCountError, WheelDiameter,
};
pub use control::{
    AngleCurrentGain, IntegralCurrentGain, MahonyPitchGain, MahonyRollGain, PidScale,
    RateCurrentGain,
};
pub use gnss::{GnssAccuracy, GnssAltitude, GnssHdop, GnssLatitude, GnssLongitude, GnssSpeed};
pub use imu::{
    ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRate,
    ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw, ImuAttitude, ImuMagneticField,
    ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuOrientation, ImuPitch,
    ImuQuaternion, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ, ImuReadSample,
    ImuRoll, ImuSamplePeriod, ImuYaw,
};
pub use io::{
    BaudRate, BaudRateError, PacketLength, PacketLengthError, ThreadPriority, ThreadPriorityError,
};
#[cfg(any(test, feature = "test-support"))]
pub use loader::PackageArgument;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) use loader::PackageArgument;
pub use motion::{
    AbsoluteTachometerSteps, ElectricalSpeed, MechanicalSpeed, OpenLoopPhase, PidPosition,
    TachometerSteps, TripDistance, VehicleSpeed,
};
pub use motor::{
    AudioChannel, AudioChannelError, AudioDuration, AudioFrequency, AudioSampleRate, AudioVoltage,
    BrakeCurrent, DCurrent, DVoltage, DirectionalMotorCurrent, FirmwareFaultCode,
    FirmwareFaultWireCode, HandbrakeCurrent, MotorCurrent, MotorCurrentLimit, OpenLoopCurrent,
    PhaseCurrent, QCurrent, QVoltage, TotalMotorCurrent,
};
pub use power::{AveragePower, PeakPower};
pub use ratio::{
    BrakeCurrentRelative, CurrentRelative, DutyCycle, HandbrakeRelative, JoystickX, JoystickY,
    PpmInput, Pwm,
};
pub use temperature::{
    FetTemperature, MosfetTemperature, MotorTemperature, TemperatureLimitEnd, TemperatureLimitStart,
};
pub use time::{CurrentOffDelay, PpmAge, RemoteAge, SystemDuration, TimeoutDuration};
pub use version::FirmwareVersion;
