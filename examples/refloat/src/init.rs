//! Native loader entrypoints for the Refloat package.

use vescpkg_rs::ffi;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
pub(crate) static prog_ptr: u32 = 0;

/// ARM package loader entrypoint for the containment Refloat payload.
#[cfg(all(not(test), target_arch = "arm"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    // Refloat v1.2.1 (0ef6e99d8701) `src/main.c:2456-2459` registers custom
    // config, app-data, then loader extensions. This candidate keeps only the
    // loader extensions while app-data side effects are isolated on hardware.
    refloat_package_start(|| unsafe { crate::extensions::register_refloat_loader_extensions(info) })
}

/// Host non-test builds install only the stop hook.
#[cfg(all(not(test), not(target_arch = "arm")))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    let _ = vescpkg_rs::init::install_stop_hook(info);
    true
}

/// Test-build package loader entrypoint that mirrors the target init behavior.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    let _ = vescpkg_rs::init::install_stop_hook(info);
    true
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn refloat_package_start(register_loader_extensions: impl FnOnce() -> bool) -> bool {
    register_loader_extensions()
}

/// ARM package loader entrypoint placed in `.init_fun` for VESC firmware loading.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    let _ = package_lib_init(info);
    true
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    #[test]
    fn refloat_startup_registers_only_loader_extensions() {
        let extension_calls = Cell::new(0);

        let result = super::refloat_package_start(|| {
            extension_calls.set(extension_calls.get() + 1);
            true
        });

        assert!(result);
        assert_eq!(extension_calls.get(), 1);
    }

    #[test]
    fn refloat_startup_fails_when_loader_extensions_fail() {
        assert!(!super::refloat_package_start(|| false));
    }
}
