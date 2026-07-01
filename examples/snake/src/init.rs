//! Native loader entrypoints for the Snake example package.

use vescpkg_rs::{ffi, init as pkg_init};

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
static prog_ptr: u32 = 0;

/// Package loader entrypoint that installs the example stop hook and reports success.
#[cfg(not(test))]
#[inline(never)]
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    let _ = pkg_init::install_stop_hook(info);
    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };
    crate::app_data::register_snake_app_data_handler(info)
}

/// Test-build package loader entrypoint that mirrors the target init behavior.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    let _ = pkg_init::install_stop_hook(info);
    true
}

/// ARM package loader entrypoint placed in `.init_fun` for VESC firmware loading.
#[cfg(all(not(test), target_arch = "arm"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    package_lib_init(info)
}
