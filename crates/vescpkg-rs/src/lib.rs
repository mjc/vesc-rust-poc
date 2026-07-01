//! Target-side SDK for Rust VESC packages.
//!
//! Link this crate into native VESC package code. It wraps `vescpkg-rs-sys` with
//! lifecycle, LispBM extension, app-data, GPIO, and protocol helpers.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

#[cfg(test)]
extern crate std;

mod bindings;
mod extension;
mod lifecycle_core;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Safe and unsafe raw ABI re-exports for SDK consumers that need them.
pub mod ffi {
    pub use crate::bindings::*;
    pub use crate::extension::*;
    pub use crate::lifecycle_core::*;
    #[cfg(any(test, feature = "test-support"))]
    pub use crate::test_support;
    pub use vescpkg_rs_sys::*;
}

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};
pub use vescpkg_rs_units as units;

pub use bindings::{AppDataBindings, LbmBindings};
pub use extension::{ExtensionDescriptor, ExtensionNameError, RegisterError};
pub use lifecycle_core::{LbmApi, LoopbackLifecycle, PackageLifecycle};

#[cfg(not(test))]
pub use bindings::RealBindings;

/// BLE loopback helpers and package-side packet handlers.
pub mod ble_loopback;
/// GPIO bindings and convenience wrappers for package code.
pub mod gpio;
/// Device package entrypoint and loader-hook helpers.
pub mod init;

#[cfg(not(test))]
pub use gpio::RealGpioBindings;
pub use gpio::{GpioApi, GpioBindings};
/// LispBM value encoding helpers and raw device-side integer packing.
pub mod lbm;
/// Higher-level lifecycle helpers for package startup and runtime behavior.
pub mod lifecycle;
/// VESC-domain semantic wrappers over generic embedded units.
pub mod types;

#[cfg(test)]
mod tests {
    use super::{ProtocolFrame, WireCommand, WireVersion};
    use crate::types::{
        AudioVoltage, AveragePower, BatteryCurrent, BatteryVoltage, CanControllerId, DVoltage,
        DirectionalMotorCurrent, FocMotorFluxLinkage, FocMotorInductance, FocMotorResistance,
        GearRatio, GnssLatitude, GnssLongitude, GnssSpeed, MechanicalSpeed, MotorCurrent,
        MotorPoleCount, PeakPower, QVoltage, ThreadPriority, TotalMotorCurrent, TripDistance,
        VehicleSpeed, WattHoursDischarged, WheelDiameter,
    };
    use vescpkg_rs_units::{
        Current, Distance, Energy, FluxLinkage, Inductance, Latitude, Longitude, MechanicalRpm,
        Power, Resistance, Speed, Voltage,
    };

    #[test]
    fn device_side_can_use_the_shared_protocol_crate() {
        let frame = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[7, 8]);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Ping);
        assert_eq!(frame.payload(), &[7, 8]);
    }

    #[test]
    fn semantic_current_types_are_not_interchangeable() {
        let motor = MotorCurrent::new(Current::from_amps(10.0));
        let battery = BatteryCurrent::new(Current::from_amps(6.0));

        assert_eq!(motor.current().as_amps(), 10.0);
        assert_eq!(battery.current().as_amps(), 6.0);
    }

    #[test]
    fn semantic_voltage_energy_and_aggregate_current_types_wrap_units() {
        let total = TotalMotorCurrent::new(Current::from_amps(18.0));
        let directional = DirectionalMotorCurrent::new(Current::from_amps(-2.0));
        let battery_voltage = BatteryVoltage::new(Voltage::from_volts(50.4));
        let discharged = WattHoursDischarged::new(Energy::from_watt_hours(42.0));
        let d_voltage = DVoltage::new(Voltage::from_volts(1.25));
        let q_voltage = QVoltage::new(Voltage::from_volts(2.5));
        let audio_voltage = AudioVoltage::new(Voltage::from_volts(0.75));
        let average_power = AveragePower::new(Power::from_watts(420.0));
        let peak_power = PeakPower::new(Power::from_watts(900.0));

        assert_eq!(total.current().as_amps(), 18.0);
        assert_eq!(directional.current().as_amps(), -2.0);
        assert_eq!(battery_voltage.voltage().as_volts(), 50.4);
        assert_eq!(discharged.energy().as_watt_hours(), 42.0);
        assert_eq!(d_voltage.voltage().as_volts(), 1.25);
        assert_eq!(q_voltage.voltage().as_volts(), 2.5);
        assert_eq!(audio_voltage.voltage().as_volts(), 0.75);
        assert_eq!(average_power.power().as_watts(), 420.0);
        assert_eq!(peak_power.power().as_watts(), 900.0);
    }

    #[test]
    fn semantic_motion_and_gnss_types_wrap_units() {
        let speed = VehicleSpeed::new(Speed::from_meters_per_second(4.0));
        let trip = TripDistance::new(Distance::from_meters(123.0));
        let mechanical = MechanicalSpeed::new(MechanicalRpm::from_revolutions_per_minute(3000.0));
        let latitude = GnssLatitude::new(Latitude::from_degrees(40.015));
        let longitude = GnssLongitude::new(Longitude::from_degrees(-105.2705));
        let gnss_speed = GnssSpeed::new(Speed::from_meters_per_second(3.5));

        assert_eq!(speed.speed().as_meters_per_second(), 4.0);
        assert_eq!(trip.distance().as_meters(), 123.0);
        assert_eq!(mechanical.rpm().as_revolutions_per_minute(), 3000.0);
        assert_eq!(latitude.latitude().as_degrees(), 40.015);
        assert_eq!(longitude.longitude().as_degrees(), -105.2705);
        assert_eq!(gnss_speed.speed().as_meters_per_second(), 3.5);
    }

    #[test]
    fn semantic_config_types_wrap_units_and_checked_scalars() {
        let poles = MotorPoleCount::try_new(14).expect("valid pole count");
        let cells = crate::types::BatteryCellCount::try_new(12).expect("valid cell count");
        let gear_ratio = GearRatio::try_new(2.6).expect("valid gear ratio");
        let wheel = WheelDiameter::new(Distance::from_meters(0.165));
        let motor_r = FocMotorResistance::new(Resistance::from_ohms(0.03));
        let motor_l = FocMotorInductance::new(Inductance::from_henries(0.000_012));
        let flux = FocMotorFluxLinkage::new(FluxLinkage::from_webers(0.004));

        assert_eq!(poles.get(), 14);
        assert_eq!(cells.get(), 12);
        assert_eq!(gear_ratio.get(), 2.6);
        assert_eq!(wheel.distance().as_meters(), 0.165);
        assert_eq!(motor_r.resistance().as_ohms(), 0.03);
        assert_eq!(motor_l.inductance().as_henries(), 0.000_012);
        assert_eq!(flux.flux_linkage().as_webers(), 0.004);
        assert!(MotorPoleCount::try_new(0).is_err());
        assert!(crate::types::BatteryCellCount::try_new(0).is_err());
        assert!(GearRatio::try_new(0.0).is_err());
    }

    #[test]
    fn semantic_raw_tokens_require_explicit_checked_construction() {
        let controller = CanControllerId::new(42);
        let priority = ThreadPriority::try_new(5).expect("valid priority");

        assert_eq!(controller.get(), 42);
        assert_eq!(priority.get(), 5);
        assert!(ThreadPriority::try_new(6).is_err());
        assert!(ThreadPriority::try_new(-6).is_err());
    }
}

#[cfg(all(test, feature = "test-support"))]
mod lifecycle_tests;
