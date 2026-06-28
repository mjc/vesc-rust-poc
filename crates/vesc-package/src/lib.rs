//! Safe wrapper around `vesc-ffi` for VESC native package development.
//!
//! Owns binding traits, lifecycle helpers, loader init, and device-side runtime code.
//! Raw firmware ABI types live in `vesc-ffi`; this crate builds the safe layer on top.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

mod bindings;
mod extension;
mod lifecycle_core;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod ffi {
    //! Raw ABI plus the safe package runtime surface re-exported for callers.
    pub use crate::bindings::*;
    pub use crate::extension::*;
    pub use crate::lifecycle_core::*;
    #[cfg(any(test, feature = "test-support"))]
    pub use crate::test_support;
    pub use vesc_ffi::*;
}

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

pub use bindings::{AppDataBindings, LbmBindings};
pub use extension::{ExtensionDescriptor, ExtensionNameError, RegisterError};
pub use lifecycle_core::{LbmApi, LoopbackLifecycle, PackageLifecycle};

#[cfg(not(test))]
pub use bindings::RealBindings;

pub mod ble_loopback;
pub mod init;
pub mod lbm;
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
