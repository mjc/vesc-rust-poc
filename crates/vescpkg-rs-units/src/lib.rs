//! Reusable `no_std` measurement newtypes for VESC-related Rust code.
//!
//! This crate owns physical units only. VESC-specific meanings such as motor
//! current versus battery current belong in `vescpkg-rs-types`; raw firmware ABI
//! values belong in `vescpkg-rs-sys`; raw protocol byte conversion belongs in
//! `vesc-protocol`.
//!
//! This is an unofficial Rust crate for VESC package experiments; it is not an
//! official VESC project or endorsed package API.
//!
//! The default build has no `std` and no `alloc`; third-party unit facades are
//! intentionally out of scope for this core embedded layer.
//!
//! # Unit Boundary
//!
//! Use these local newtypes as the normal public API for physical measurements:
//!
//! ```
//! use vescpkg_rs_units::prelude::*;
//!
//! let pack_voltage = Voltage::from_volts(50.4);
//! let speed = Speed::from_kilometers_per_hour(36.0);
//! let stored = Energy::from_watt_hours(2.0);
//! let ratio = Ratio::from_ratio(0.25).expect("in range");
//!
//! assert_eq!(pack_voltage.as_volts(), 50.4);
//! assert_eq!(speed.as_meters_per_second(), 10.0);
//! assert_eq!(stored.as_watt_hours(), 2.0);
//! assert_eq!(ratio.as_ratio(), 0.25);
//! ```
//!
//! Raw primitive values are explicit boundary conversions, not the default way
//! to pass measurements around:
//!
//! ```
//! use vescpkg_rs_units::Voltage;
//!
//! let voltage = Voltage::from_volts(57.0);
//! let abi_value: f32 = voltage.as_volts();
//!
//! assert_eq!(abi_value, 57.0);
//! ```
//!
//! Implicit primitive erasure is intentionally not available:
//!
//! ```compile_fail
//! use vescpkg_rs_units::Voltage;
//!
//! let _: f32 = Voltage::from_volts(57.0).into();
//! ```
//!
//! ```compile_fail
//! use vescpkg_rs_units::Ratio;
//!
//! let ratio = Ratio::from_ratio(0.75).expect("valid");
//! let _: f32 = ratio.into();
//! ```
//!
//! VESC-specific meanings belong in a separate domain layer. For example,
//! motor current and battery current should be distinct domain types even though
//! both can contain [`Current`].

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_code)]

#[cfg(test)]
extern crate std;

mod macros;
pub(crate) use macros::{bounded_unit, scalar_int_unit, scalar_unit, scalar_unit_f64};

pub mod battery;
pub mod electrical;
pub mod gnss;
pub mod motion;
pub mod ratio;
pub mod temperature;
pub mod time;

pub use battery::{AmpHours, Charge, DistancePerEnergy, Energy, EnergyPerDistance, WattHours};
pub use electrical::{Current, FluxLinkage, Inductance, Power, Resistance, Voltage};
pub use gnss::{Height, Latitude, Longitude};
pub use motion::{
    AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, Distance, OdometerMeters, Rpm,
    Speed, TachometerSteps,
};
pub use ratio::{Percent, Ratio, SignedRatio};
pub use temperature::Temperature;
pub use time::{
    AbiSeconds, Frequency, SYSTEM_TICK_RATE_HZ, SampleRate, SystemInstant, SystemTicks,
    TimestampTicks,
};

/// Common package-author imports for typed unit calculations.
pub mod prelude {
    pub use crate::{
        AbiSeconds, AccelerationG, AmpHours, AngleDegrees, AngleRadians, AngularVelocity,
        BoundedUnitError, Charge, Current, Distance, DistancePerEnergy, Energy, EnergyPerDistance,
        FluxLinkage, Frequency, Height, Inductance, Latitude, Longitude, OdometerMeters, Percent,
        Power, Ratio, Resistance, Rpm, SYSTEM_TICK_RATE_HZ, SampleRate, SignedRatio, Speed,
        SystemInstant, SystemTicks, TachometerSteps, Temperature, TimestampTicks, Voltage,
        WattHours,
    };
}

#[cfg(test)]
mod tests;

/// Error returned when a bounded unit rejects an out-of-range value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedUnitError {
    value: f32,
    min: f32,
    max: f32,
}

impl BoundedUnitError {
    /// Create a bounded-unit error.
    pub const fn new(value: f32, min: f32, max: f32) -> Self {
        Self { value, min, max }
    }

    /// Return the rejected value.
    pub const fn value(self) -> f32 {
        self.value
    }

    /// Return the inclusive lower bound.
    pub const fn min(self) -> f32 {
        self.min
    }

    /// Return the inclusive upper bound.
    pub const fn max(self) -> f32 {
        self.max
    }
}
