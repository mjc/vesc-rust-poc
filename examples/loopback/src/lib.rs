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

pub use vescpkg_rs::{ProtocolFrame, WireCommand, WireVersion, ble_loopback, ffi, lbm, lifecycle};

vescpkg_rs::package_start!(crate::start);

#[cfg(test)]
pub(crate) fn start(info: *mut ffi::LibInfo) -> bool {
    let _ = vescpkg_rs::init::install_stop_hook(info);
    true
}

#[cfg(all(not(test), target_arch = "arm"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub(crate) fn start(info: *mut ffi::LibInfo) -> bool {
    let _ = vescpkg_rs::init::install_stop_hook(info);

    let Some(info) = (unsafe { info.as_ref() }) else {
        return true;
    };

    let _ = vescpkg_rs::ble_loopback::register_loopback_app_data_handler();

    let lifecycle = ffi::PackageLifecycle::new(ffi::RealBindings);
    let _ = register_package_extensions(info, &lifecycle);

    // Extension registration can run other firmware setup; register again so the
    // loopback handler remains the active app-data callback (refloat pattern).
    let _ = vescpkg_rs::ble_loopback::register_loopback_app_data_handler();

    true
}

/// Register this package's extension table using the supplied binding set.
pub fn register_package_extensions<B: ffi::LbmBindings>(
    info: &ffi::LibInfo,
    lifecycle: &ffi::PackageLifecycle<B>,
) -> bool {
    let [descriptor] = extensions::package_extension_descriptors();
    // VESC passes loader metadata for this native package before registration.
    lifecycle::register_extension_from_image(info, lifecycle, descriptor).is_ok()
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

#[cfg(all(test, feature = "test-support"))]
mod registration_tests {
    use super::register_package_extensions;
    use crate::extensions::package_extension_descriptors;
    use vescpkg_rs::ffi::test_support::FakeBindings;
    use vescpkg_rs::ffi::{self, PackageLifecycle};

    #[test]
    fn register_package_extensions_propagates_firmware_rejection() {
        let lifecycle = PackageLifecycle::new(FakeBindings::rejecting());
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let [descriptor] = package_extension_descriptors();

        assert!(!register_package_extensions(&info, &lifecycle));
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
        assert_eq!(descriptor.name(), package_extension_descriptors()[0].name());
    }
}
