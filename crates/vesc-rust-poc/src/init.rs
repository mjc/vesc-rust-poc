//! Native loader entrypoints for the BLE loopback proof-of-concept package.

use vesc_package::{ffi, init as pkg_init, lifecycle};

use crate::extensions;

#[cfg(test)]
use core::cell::Cell;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[no_mangle]
#[link_section = ".program_ptr"]
static prog_ptr: u32 = 0;

#[cfg(not(test))]
#[inline(never)]
#[no_mangle]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    pkg_init::install_stop_hook(info)
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    init_for_tests(info)
}

#[cfg(all(not(test), target_arch = "arm"))]
#[no_mangle]
#[link_section = ".init_fun"]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    if !package_lib_init(info) {
        return false;
    }

    register_package_extensions(info)
}

/// Register this package's extension table using the compact init path.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn register_package_extensions(info: *mut ffi::LibInfo) -> bool {
    if info.is_null() {
        return false;
    }

    let [descriptor] = extensions::package_extension_descriptors();
    unsafe { lifecycle::register_extension_from_image_real(&*info, descriptor).is_ok() }
}

#[cfg(test)]
thread_local! {
    static INIT_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub fn init_for_tests(info: *mut ffi::LibInfo) -> bool {
    let _ = pkg_init::install_stop_hook(info);
    INIT_CALLS.with(|calls| calls.set(calls.get() + 1));
    true
}

#[cfg(test)]
pub fn reset_init_call_count_for_tests() {
    INIT_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
pub fn init_call_count_for_tests() -> usize {
    INIT_CALLS.with(|calls| calls.get())
}

#[cfg(all(test, feature = "test-support"))]
mod registration_tests {
    use super::register_package_extensions;
    use crate::extensions::package_extension_descriptors;
    use vesc_package::ffi::test_support::FakeBindings;
    use vesc_package::ffi::{self, PackageLifecycle};
    use vesc_package::lifecycle::register_extension_from_image;

    #[test]
    fn register_package_extensions_propagates_firmware_rejection() {
        let bindings = FakeBindings::rejecting();
        let lifecycle = PackageLifecycle::new(bindings);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let [descriptor] = package_extension_descriptors();

        assert!(!register_package_extensions(core::ptr::null_mut()));
        assert!(
            register_extension_from_image(&info, &lifecycle, descriptor).is_err(),
            "registration failure should propagate to init callers"
        );
        let _ = &mut info;
    }
}

#[cfg(test)]
mod tests {
    use super::{init_for_tests, reset_init_call_count_for_tests};
    use vesc_package::ffi;

    #[test]
    fn package_init_records_device_initialization() {
        reset_init_call_count_for_tests();

        assert!(init_for_tests(core::ptr::null_mut()));
        assert_eq!(super::init_call_count_for_tests(), 1);
    }

    #[test]
    fn package_init_installs_a_stop_hook() {
        reset_init_call_count_for_tests();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(init_for_tests(&mut info));
        assert!(info.stop_fun.is_some());
    }
}
