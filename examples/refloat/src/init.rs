//! Native loader entrypoints for the Refloat package.

use vescpkg_rs::ffi;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
pub(crate) static prog_ptr: u32 = 0;

/// ARM package loader entrypoint for the Refloat payload.
///
/// C map: Refloat v1.2.1 `INIT_FUN` starts at
/// `/Users/mjc/projects/refloat/src/main.c:2415`.
#[cfg(all(not(test), target_arch = "arm"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    // Refloat v1.2.1 (0ef6e99d8701)
    // `/Users/mjc/projects/refloat/src/main.c:2419-2461` allocates `Data`,
    // runs `data_init`, installs stop/ARG, starts main+aux threads, then
    // registers IMU, custom config, app-data, and LispBM extensions.
    const REFLOAT_STARTUP: &[vescpkg_rs::PackageStartStep] = &[
        crate::package::install_refloat_package_state,
        crate::runtime::start_refloat_runtime_threads,
        crate::package::register_refloat_imu_callback,
        crate::package::register_refloat_app_data_callbacks,
        crate::extensions::register_refloat_loader_extensions,
    ];

    vescpkg_rs::start_package(info, REFLOAT_STARTUP)
}

/// Host non-test builds keep a generic stop-hook shim for host linking only.
///
/// This is not target Refloat parity: upstream installs `stop`/`Data *` during
/// ARM startup at `/Users/mjc/projects/refloat/src/main.c:2431-2432`, while the
/// host shim only keeps host linking alive.
#[cfg(all(not(test), not(target_arch = "arm")))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    let _ = vescpkg_rs::init::install_stop_hook(info);
    true
}

/// Test-build package loader entrypoint for the startup side-effect boundary.
///
/// C map: Refloat v1.2.1 `INIT_FUN` starts at
/// `/Users/mjc/projects/refloat/src/main.c:2415`.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    vescpkg_rs::start_package(info, &[])
}

/// ARM package loader entrypoint placed in `.init_fun` for VESC firmware loading.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    package_lib_init(info)
}

#[cfg(test)]
mod tests {
    use vescpkg_rs::ffi;

    #[test]
    fn package_lib_init_uses_generic_startup_entrypoint() {
        assert!(super::package_lib_init(
            core::ptr::null_mut::<ffi::LibInfo>()
        ));
    }
}
