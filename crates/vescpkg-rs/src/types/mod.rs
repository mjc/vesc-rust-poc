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

pub mod battery;
pub mod motor;
pub mod ratio;
pub mod temperature;

pub use battery::{AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, InputVoltage};
pub use motor::{
    BrakeCurrent, DCurrent, HandbrakeCurrent, MotorCurrent, OpenLoopCurrent, PhaseCurrent, QCurrent,
};
pub use ratio::{BrakeCurrentRelative, CurrentRelative, DutyCycle};
pub use temperature::{FetTemperature, MotorTemperature};
