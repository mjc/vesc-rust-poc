//! Native loader entrypoints for the BLE loopback proof-of-concept package.

use vesc_sdk::{ffi, init as pkg_init, lifecycle};

use crate::extensions;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
static prog_ptr: u32 = 0;

#[cfg(not(test))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    pkg_init::install_stop_hook(info)
}

#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    pkg_init::install_stop_hook(info)
}

#[cfg(all(not(test), target_arch = "arm"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    if !package_lib_init(info) {
        return false;
    }

    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };

    let lifecycle = ffi::PackageLifecycle::new(ffi::RealBindings);
    register_package_extensions(info, &lifecycle)
}

/// Register this package's extension table using the supplied binding set.
pub fn register_package_extensions<B: ffi::LbmBindings>(
    info: &ffi::LibInfo,
    lifecycle: &ffi::PackageLifecycle<B>,
) -> bool {
    let [descriptor] = extensions::package_extension_descriptors();
    lifecycle::register_extension_from_image(info, lifecycle, descriptor).is_ok()
}

#[cfg(all(test, feature = "test-support"))]
mod registration_tests {
    use super::register_package_extensions;
    use crate::extensions::package_extension_descriptors;
    use vesc_sdk::ffi::test_support::FakeBindings;
    use vesc_sdk::ffi::{self, PackageLifecycle};

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
