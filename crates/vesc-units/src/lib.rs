//! Reusable `no_std` measurement newtypes for VESC-related Rust code.
//!
//! This crate owns physical units only. VESC-specific meanings such as motor
//! current versus battery current belong in `vesc-types`; raw firmware ABI
//! values belong in `vescpkg-sys`; raw protocol byte conversion belongs in
//! `vesc-protocol`.
//!
//! The default build has no `std`, no `alloc`, and no `uom` dependency. Enable
//! the `uom` feature only when boundary conversion to third-party quantities is
//! useful.
//!
//! # Unit Boundary
//!
//! Use these local newtypes as the normal public API for physical measurements:
//!
//! ```
//! use vesc_units::{DutyCycle, Energy, Speed, Voltage};
//!
//! let pack_voltage = Voltage::from_volts(50.4);
//! let speed = Speed::from_kilometers_per_hour(36.0);
//! let stored = Energy::from_watt_hours(2.0);
//! let duty = DutyCycle::from_ratio(0.25).expect("in range");
//!
//! assert_eq!(pack_voltage.as_volts(), 50.4);
//! assert_eq!(speed.as_meters_per_second(), 10.0);
//! assert_eq!(stored.as_joules(), 7200.0);
//! assert_eq!(duty.as_ratio(), 0.25);
//! ```
//!
//! Raw primitive values are explicit boundary conversions, not the default way
//! to pass measurements around:
//!
//! ```
//! use vesc_units::Voltage;
//!
//! let voltage = Voltage::from_volts(57.0);
//! let abi_value: f32 = voltage.into();
//!
//! assert_eq!(abi_value, 57.0);
//! ```
//!
//! VESC-specific meanings belong in a separate domain layer. For example,
//! motor current and battery current should be distinct domain types even though
//! both can contain [`Current`]. [`Efficiency`] is only a generic
//! watt-hours-per-mile measurement here; controller-specific efficiency semantics
//! should live in `vesc-types`.
//!
//! # Optional `uom` Compatibility
//!
//! The `uom` feature adds conversion at interoperability boundaries while the
//! local newtypes remain the public representation in this core crate. A higher
//! level package facade may choose `uom` return types when that ergonomic layer
//! is ready; that policy does not change the embedded units boundary here.
//!
//! ```
//! #[cfg(feature = "uom")]
//! {
//!     use uom::si::electric_potential::volt;
//!     use uom::si::f32::ElectricPotential;
//!     use vesc_units::Voltage;
//!
//!     let quantity = ElectricPotential::from(Voltage::from_volts(12.0));
//!     assert_eq!(quantity.get::<volt>(), 12.0);
//!     assert_eq!(Voltage::from(quantity).as_volts(), 12.0);
//! }
//! ```

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_code)]

#[cfg(test)]
extern crate std;

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

macro_rules! scalar_unit {
    ($name:ident, $from:ident, $as:ident, $unit:literal) => {
        #[doc = concat!("Generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            #[doc = concat!("Create a value from ", $unit, ".")]
            pub const fn $from(value: f32) -> Self {
                Self(value)
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f32 {
                self.0
            }
        }

        impl From<$name> for f32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

macro_rules! bounded_unit {
    ($name:ident, $from:ident, $as:ident, $min:expr, $max:expr, $unit:literal) => {
        #[doc = concat!("Bounded generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Inclusive lower bound for this unit.
            pub const MIN: f32 = $min;

            /// Inclusive upper bound for this unit.
            pub const MAX: f32 = $max;

            #[doc = concat!("Create a checked value from ", $unit, ".")]
            pub const fn $from(value: f32) -> Result<Self, BoundedUnitError> {
                if value >= Self::MIN && value <= Self::MAX {
                    Ok(Self(value))
                } else {
                    Err(BoundedUnitError::new(value, Self::MIN, Self::MAX))
                }
            }

            #[doc = concat!("Clamp a primitive value into the valid ", $unit, " range.")]
            pub const fn clamped(value: f32) -> Self {
                if value != value || value < Self::MIN {
                    Self(Self::MIN)
                } else if value > Self::MAX {
                    Self(Self::MAX)
                } else {
                    Self(value)
                }
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f32 {
                self.0
            }
        }

        impl From<$name> for f32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

scalar_unit!(Voltage, from_volts, as_volts, "volts");
scalar_unit!(Current, from_amps, as_amps, "amps");
scalar_unit!(Power, from_watts, as_watts, "watts");
scalar_unit!(Energy, from_joules, as_joules, "joules");
scalar_unit!(AmpHours, from_amp_hours, as_amp_hours, "amp-hours");
scalar_unit!(WattHours, from_watt_hours, as_watt_hours, "watt-hours");
scalar_unit!(
    MechanicalRpm,
    from_revolutions_per_minute,
    as_revolutions_per_minute,
    "mechanical revolutions per minute"
);
scalar_unit!(
    ElectricalRpm,
    from_revolutions_per_minute,
    as_revolutions_per_minute,
    "electrical revolutions per minute"
);
scalar_unit!(Distance, from_meters, as_meters, "meters");
scalar_unit!(
    Speed,
    from_meters_per_second,
    as_meters_per_second,
    "meters per second"
);
scalar_unit!(
    TemperatureC,
    from_degrees_celsius,
    as_degrees_celsius,
    "degrees Celsius"
);
scalar_unit!(Latitude, from_degrees, as_degrees, "degrees latitude");
scalar_unit!(Longitude, from_degrees, as_degrees, "degrees longitude");
scalar_unit!(Altitude, from_meters, as_meters, "meters altitude");
scalar_unit!(GnssAccuracy, from_meters, as_meters, "meters accuracy");
scalar_unit!(
    Efficiency,
    from_watt_hours_per_mile,
    as_watt_hours_per_mile,
    "watt-hours per mile"
);

impl Energy {
    /// Create an energy value from watt-hours.
    pub const fn from_watt_hours(value: f32) -> Self {
        Self::from_joules(value * 3600.0)
    }

    /// Return this energy value in watt-hours.
    pub const fn as_watt_hours(self) -> f32 {
        self.as_joules() / 3600.0
    }
}

impl WattHours {
    /// Create a watt-hour value from joules.
    pub const fn from_joules(value: f32) -> Self {
        Self::from_watt_hours(value / 3600.0)
    }

    /// Return this watt-hour value in joules.
    pub const fn as_joules(self) -> f32 {
        self.as_watt_hours() * 3600.0
    }
}

impl Speed {
    /// Create a speed value from kilometers per hour.
    pub const fn from_kilometers_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value / 3.6)
    }

    /// Return this speed value in kilometers per hour.
    pub const fn as_kilometers_per_hour(self) -> f32 {
        self.as_meters_per_second() * 3.6
    }

    /// Create a speed value from miles per hour.
    pub const fn from_miles_per_hour(value: f32) -> Self {
        Self::from_meters_per_second(value * 0.447_04)
    }

    /// Return this speed value in miles per hour.
    pub const fn as_miles_per_hour(self) -> f32 {
        self.as_meters_per_second() / 0.447_04
    }
}

bounded_unit!(DutyCycle, from_ratio, as_ratio, 0.0, 1.0, "normalized duty");
bounded_unit!(Ratio, from_ratio, as_ratio, 0.0, 1.0, "normalized ratio");
bounded_unit!(Percent, from_percent, as_percent, 0.0, 100.0, "percent");
bounded_unit!(Pwm, from_ratio, as_ratio, 0.0, 1.0, "normalized PWM");

#[cfg(feature = "uom")]
mod uom_compat {
    use super::{
        Current, Distance, ElectricalRpm, Energy, MechanicalRpm, Power, Speed, TemperatureC,
        Voltage, WattHours,
    };
    use uom::si::angular_velocity::revolution_per_minute;
    use uom::si::electric_current::ampere;
    use uom::si::electric_potential::volt;
    use uom::si::energy::{joule, watt_hour};
    use uom::si::f32::{
        AngularVelocity, ElectricCurrent, ElectricPotential, Energy as UomEnergy, Length,
        Power as UomPower, ThermodynamicTemperature, Velocity,
    };
    use uom::si::length::meter;
    use uom::si::power::watt;
    use uom::si::thermodynamic_temperature::degree_celsius;
    use uom::si::velocity::meter_per_second;

    impl From<Voltage> for ElectricPotential {
        fn from(value: Voltage) -> Self {
            Self::new::<volt>(value.as_volts())
        }
    }

    impl From<ElectricPotential> for Voltage {
        fn from(value: ElectricPotential) -> Self {
            Self::from_volts(value.get::<volt>())
        }
    }

    impl From<Current> for ElectricCurrent {
        fn from(value: Current) -> Self {
            Self::new::<ampere>(value.as_amps())
        }
    }

    impl From<ElectricCurrent> for Current {
        fn from(value: ElectricCurrent) -> Self {
            Self::from_amps(value.get::<ampere>())
        }
    }

    impl From<Power> for UomPower {
        fn from(value: Power) -> Self {
            Self::new::<watt>(value.as_watts())
        }
    }

    impl From<UomPower> for Power {
        fn from(value: UomPower) -> Self {
            Self::from_watts(value.get::<watt>())
        }
    }

    impl From<Energy> for UomEnergy {
        fn from(value: Energy) -> Self {
            Self::new::<joule>(value.as_joules())
        }
    }

    impl From<UomEnergy> for Energy {
        fn from(value: UomEnergy) -> Self {
            Self::from_joules(value.get::<joule>())
        }
    }

    impl From<WattHours> for UomEnergy {
        fn from(value: WattHours) -> Self {
            Self::new::<watt_hour>(value.as_watt_hours())
        }
    }

    impl From<UomEnergy> for WattHours {
        fn from(value: UomEnergy) -> Self {
            Self::from_watt_hours(value.get::<watt_hour>())
        }
    }

    impl From<Distance> for Length {
        fn from(value: Distance) -> Self {
            Self::new::<meter>(value.as_meters())
        }
    }

    impl From<Length> for Distance {
        fn from(value: Length) -> Self {
            Self::from_meters(value.get::<meter>())
        }
    }

    impl From<Speed> for Velocity {
        fn from(value: Speed) -> Self {
            Self::new::<meter_per_second>(value.as_meters_per_second())
        }
    }

    impl From<Velocity> for Speed {
        fn from(value: Velocity) -> Self {
            Self::from_meters_per_second(value.get::<meter_per_second>())
        }
    }

    impl From<TemperatureC> for ThermodynamicTemperature {
        fn from(value: TemperatureC) -> Self {
            Self::new::<degree_celsius>(value.as_degrees_celsius())
        }
    }

    impl From<ThermodynamicTemperature> for TemperatureC {
        fn from(value: ThermodynamicTemperature) -> Self {
            Self::from_degrees_celsius(value.get::<degree_celsius>())
        }
    }

    impl From<MechanicalRpm> for AngularVelocity {
        fn from(value: MechanicalRpm) -> Self {
            Self::new::<revolution_per_minute>(value.as_revolutions_per_minute())
        }
    }

    impl From<AngularVelocity> for MechanicalRpm {
        fn from(value: AngularVelocity) -> Self {
            Self::from_revolutions_per_minute(value.get::<revolution_per_minute>())
        }
    }

    impl From<ElectricalRpm> for AngularVelocity {
        fn from(value: ElectricalRpm) -> Self {
            Self::new::<revolution_per_minute>(value.as_revolutions_per_minute())
        }
    }

    impl From<AngularVelocity> for ElectricalRpm {
        fn from(value: AngularVelocity) -> Self {
            Self::from_revolutions_per_minute(value.get::<revolution_per_minute>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AmpHours, Current, DutyCycle, ElectricalRpm, Energy, Percent, Power, Ratio, Speed,
        TemperatureC, Voltage, WattHours,
    };

    #[test]
    fn scalar_units_round_trip_through_named_accessors() {
        assert_eq!(Voltage::from_volts(50.5).as_volts(), 50.5);
        assert_eq!(Current::from_amps(12.25).as_amps(), 12.25);
        assert_eq!(Power::from_watts(600.0).as_watts(), 600.0);
        assert_eq!(Energy::from_joules(42.0).as_joules(), 42.0);
        assert_eq!(AmpHours::from_amp_hours(3.2).as_amp_hours(), 3.2);
        assert_eq!(WattHours::from_watt_hours(70.0).as_watt_hours(), 70.0);
        assert_eq!(
            ElectricalRpm::from_revolutions_per_minute(12_000.0).as_revolutions_per_minute(),
            12_000.0
        );
        assert_eq!(
            Speed::from_meters_per_second(4.5).as_meters_per_second(),
            4.5
        );
        assert_eq!(
            TemperatureC::from_degrees_celsius(23.0).as_degrees_celsius(),
            23.0
        );
    }

    #[test]
    fn local_unit_conversions_do_not_need_uom() {
        assert_eq!(Energy::from_watt_hours(2.0).as_joules(), 7200.0);
        assert_eq!(WattHours::from_joules(7200.0).as_watt_hours(), 2.0);
        assert_eq!(
            Speed::from_kilometers_per_hour(36.0).as_meters_per_second(),
            10.0
        );
        assert_eq!(Speed::from_miles_per_hour(60.0).as_miles_per_hour(), 60.0);
    }

    #[test]
    fn bounded_units_reject_out_of_range_values() {
        assert_eq!(DutyCycle::from_ratio(0.5).expect("valid").as_ratio(), 0.5);

        let low = DutyCycle::from_ratio(-0.1).expect_err("too low");
        assert_eq!(low.value(), -0.1);
        assert_eq!(low.min(), 0.0);
        assert_eq!(low.max(), 1.0);

        let high = Percent::from_percent(101.0).expect_err("too high");
        assert_eq!(high.value(), 101.0);
        assert_eq!(high.min(), 0.0);
        assert_eq!(high.max(), 100.0);
    }

    #[test]
    fn bounded_units_clamp_without_panicking() {
        assert_eq!(Ratio::clamped(-1.0).as_ratio(), 0.0);
        assert_eq!(Ratio::clamped(2.0).as_ratio(), 1.0);
        assert_eq!(Ratio::clamped(0.25).as_ratio(), 0.25);
        assert_eq!(Ratio::clamped(f32::NAN).as_ratio(), 0.0);
    }

    #[test]
    fn transparent_units_convert_into_primitive_boundary_values() {
        let volts: f32 = Voltage::from_volts(57.0).into();
        let duty: f32 = DutyCycle::from_ratio(0.75).expect("valid").into();

        assert_eq!(volts, 57.0);
        assert_eq!(duty, 0.75);
    }
}

#[cfg(all(test, feature = "uom"))]
mod uom_tests {
    use super::{
        Current, Distance, ElectricalRpm, Energy, MechanicalRpm, Power, Speed, TemperatureC,
        Voltage, WattHours,
    };
    use uom::si::angular_velocity::revolution_per_minute;
    use uom::si::electric_current::ampere;
    use uom::si::electric_potential::volt;
    use uom::si::energy::{joule, watt_hour};
    use uom::si::f32::{
        AngularVelocity, ElectricCurrent, ElectricPotential, Energy as UomEnergy, Length,
        Power as UomPower, ThermodynamicTemperature, Velocity,
    };
    use uom::si::length::meter;
    use uom::si::power::watt;
    use uom::si::thermodynamic_temperature::degree_celsius;
    use uom::si::velocity::meter_per_second;

    #[test]
    fn uom_conversions_round_trip_representative_units() {
        let volts = ElectricPotential::from(Voltage::from_volts(12.0));
        assert_eq!(Voltage::from(volts).as_volts(), 12.0);
        assert_eq!(volts.get::<volt>(), 12.0);

        let amps = ElectricCurrent::from(Current::from_amps(3.5));
        assert_eq!(Current::from(amps).as_amps(), 3.5);
        assert_eq!(amps.get::<ampere>(), 3.5);

        let watts = UomPower::from(Power::from_watts(42.0));
        assert_eq!(Power::from(watts).as_watts(), 42.0);
        assert_eq!(watts.get::<watt>(), 42.0);

        let joules = UomEnergy::from(Energy::from_joules(9.0));
        assert_eq!(Energy::from(joules).as_joules(), 9.0);
        assert_eq!(joules.get::<joule>(), 9.0);

        let watt_hours = UomEnergy::from(WattHours::from_watt_hours(11.0));
        assert_eq!(WattHours::from(watt_hours).as_watt_hours(), 11.0);
        assert_eq!(watt_hours.get::<watt_hour>(), 11.0);

        let meters = Length::from(Distance::from_meters(25.0));
        assert_eq!(Distance::from(meters).as_meters(), 25.0);
        assert_eq!(meters.get::<meter>(), 25.0);

        let speed = Velocity::from(Speed::from_meters_per_second(7.0));
        assert_eq!(Speed::from(speed).as_meters_per_second(), 7.0);
        assert_eq!(speed.get::<meter_per_second>(), 7.0);

        let temperature = ThermodynamicTemperature::from(TemperatureC::from_degrees_celsius(32.0));
        assert_eq!(TemperatureC::from(temperature).as_degrees_celsius(), 32.0);
        assert_eq!(temperature.get::<degree_celsius>(), 32.0);
    }

    #[test]
    fn uom_conversions_cover_rpm_like_units() {
        let mechanical = AngularVelocity::from(MechanicalRpm::from_revolutions_per_minute(3_000.0));
        let electrical =
            AngularVelocity::from(ElectricalRpm::from_revolutions_per_minute(12_000.0));

        assert_eq!(mechanical.get::<revolution_per_minute>(), 3_000.0);
        assert_eq!(electrical.get::<revolution_per_minute>(), 12_000.0);
        assert_eq!(
            MechanicalRpm::from(mechanical).as_revolutions_per_minute(),
            3_000.0
        );
        assert_eq!(
            ElectricalRpm::from(electrical).as_revolutions_per_minute(),
            12_000.0
        );
    }
}
