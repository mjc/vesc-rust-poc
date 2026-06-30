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

#[cfg(test)]
mod tests {
    use super::{ProtocolFrame, WireCommand, WireVersion};

    #[test]
    fn device_side_can_use_the_shared_protocol_crate() {
        let frame = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[7, 8]);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Ping);
        assert_eq!(frame.payload(), &[7, 8]);
    }
}

#[cfg(all(test, feature = "test-support"))]
mod lifecycle_tests;
