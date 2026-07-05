//! BLE loopback VESC package payload.
//!
//! This crate is the linkable staticlib artifact (`libvesc_example_loopback.a`). Generic loader,
//! lifecycle, and firmware wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

pub mod extensions;

pub use vescpkg_rs::{ProtocolFrame, WireCommand, WireVersion, ble_loopback, ffi, lbm};

vescpkg_rs::package_start!(crate::start);

#[cfg(test)]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();
    true
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();

    let _ = vescpkg_rs::ble_loopback::register_loopback_app_data_handler();
    let _ = start.register_extensions(extensions::package_extension_descriptors());

    // Extension registration can run other firmware setup; register again so the
    // loopback handler remains the active app-data callback (refloat pattern).
    let _ = vescpkg_rs::ble_loopback::register_loopback_app_data_handler();

    true
}

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
