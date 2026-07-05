//! Native loader entrypoints for the Refloat package.

use vescpkg_rs::ffi;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
static prog_ptr: u32 = 0;

/// ARM package loader entrypoint that installs Refloat app-data state and handler.
#[cfg(all(not(test), target_arch = "arm"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    crate::app_data::install_refloat_app_data(info)
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

/// ARM package loader entrypoint placed in `.init_fun` for VESC firmware loading.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    package_lib_init(info)
}
