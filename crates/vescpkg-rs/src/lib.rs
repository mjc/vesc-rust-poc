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
        BatteryCurrent, BatteryVoltage, DirectionalMotorCurrent, MotorCurrent, TotalMotorCurrent,
        WattHoursDischarged,
    };
    use vescpkg_rs_units::{Current, Energy, Voltage};

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

        assert_eq!(total.current().as_amps(), 18.0);
        assert_eq!(directional.current().as_amps(), -2.0);
        assert_eq!(battery_voltage.voltage().as_volts(), 50.4);
        assert_eq!(discharged.energy().as_watt_hours(), 42.0);
    }
}

#[cfg(all(test, feature = "test-support"))]
mod lifecycle_tests;
