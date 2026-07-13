//! BLE loopback VESC package payload.
//!
//! This crate is the package library linked into the Cargo-owned final ELF. Generic loader,
//! lifecycle, and firmware wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

mod app_data;
pub mod extensions;
pub mod package;

pub use package::package_lib_init;
pub use vescpkg_rs::{ProtocolFrame, WireCommand, WireVersion, ble_loopback, ffi, lbm, lifecycle};

#[cfg(test)]
mod tests {
    use super::extensions;

    #[test]
    fn rust_add_stays_a_plain_integer_function() {
        assert_eq!(extensions::rust_add(1, 2), 3);
        assert_eq!(extensions::rust_add(-8, 11), 3);
    }

    #[test]
    fn package_lib_init_runs_the_device_loopback_entrypoint_path() {
        let mut info = super::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(super::package_lib_init(&mut info));
        assert!(info.stop_fun.is_some());
    }
}
