//! BLE loopback proof-of-concept package payload.
//!
//! This crate is the linkable staticlib artifact (`libvesc_rust_poc.a`). Generic loader,
//! lifecycle, and firmware wrapper code lives in `vesc-package`.

#![cfg_attr(not(test), no_std)]

pub mod extensions;
pub mod init;

pub use init::package_lib_init;
pub use vesc_package::{
    ble_loopback, ffi, lbm, lifecycle, ProtocolFrame, WireCommand, WireVersion,
};

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::{extensions, init};
    use vesc_package::init as pkg_init;

    #[test]
    fn rust_add_stays_a_plain_integer_function() {
        assert_eq!(extensions::rust_add(1, 2), 3);
        assert_eq!(extensions::rust_add(-8, 11), 3);
    }

    #[test]
    fn package_lib_init_runs_the_device_loopback_entrypoint_path() {
        pkg_init::reset_init_call_count_for_tests();
        let mut info = super::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(init::package_lib_init(&mut info));
        assert_eq!(pkg_init::init_call_count_for_tests(), 1);
        assert!(info.stop_fun.is_some());
    }
}
