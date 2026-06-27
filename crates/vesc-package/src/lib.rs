//! Safe wrapper around `vesc-ffi` for VESC native package development.
//!
//! Owns loader helpers, generic extension registration, and device-side runtime code.

#![cfg_attr(not(test), no_std)]

pub mod ffi {
    //! Raw firmware ABI re-exported for advanced callers and tests.
    pub use vesc_ffi::*;
}

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

pub mod ble_loopback;
pub mod init;
pub mod lbm;
pub mod lifecycle;

#[cfg(test)]
mod tests {
    use super::{ffi, init, ProtocolFrame, WireCommand, WireVersion};

    #[test]
    fn device_side_can_use_the_shared_protocol_crate() {
        let frame = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[7, 8]);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Ping);
        assert_eq!(frame.payload(), &[7, 8]);
    }

    #[test]
    fn package_lib_init_runs_the_device_loopback_entrypoint_path() {
        init::reset_init_call_count_for_tests();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(init::package_lib_init(&mut info));

        assert_eq!(init::init_call_count_for_tests(), 1);
        assert!(info.stop_fun.is_some());
    }
}
