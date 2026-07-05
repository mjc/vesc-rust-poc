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
    //
    // Rust installs its compact Refloat state, starts its runtime threads,
    // and preserves the registration tail ordering: loader metadata at
    // `/Users/mjc/projects/refloat/src/main.c:2431-2432`, thread spawn at
    // `/Users/mjc/projects/refloat/src/main.c:2439-2449`, then registration at
    // `/Users/mjc/projects/refloat/src/main.c:2455-2459`.
    refloat_package_start(
        || crate::app_data::install_refloat_app_data_state(info),
        || crate::runtime::start_refloat_runtime_threads(info),
        || crate::app_data::register_refloat_app_data_callbacks(info),
        || unsafe { crate::extensions::register_refloat_loader_extensions(info) },
    )
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
pub extern "C" fn package_lib_init(_info: *mut ffi::LibInfo) -> bool {
    // Upstream Refloat v1.2.1 installs `stop`/`Data *` at
    // `/Users/mjc/projects/refloat/src/main.c:2431-2432`, starts runtime
    // threads at `/Users/mjc/projects/refloat/src/main.c:2439-2449`, then
    // registers IMU/custom config/app-data/LispBM at
    // `/Users/mjc/projects/refloat/src/main.c:2455-2459`.
    refloat_package_start(|| true, || true, || true, || true)
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn refloat_package_start(
    install_state: impl FnOnce() -> bool,
    start_runtime_threads: impl FnOnce() -> bool,
    register_app_data_callbacks: impl FnOnce() -> bool,
    register_loader_extensions: impl FnOnce() -> bool,
) -> bool {
    install_state()
        && start_runtime_threads()
        && register_app_data_callbacks()
        && register_loader_extensions()
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
    use core::cell::Cell;

    #[test]
    fn refloat_startup_registers_state_before_app_data_before_loader_extensions() {
        let state_calls = Cell::new(0);
        let thread_calls = Cell::new(0);
        let app_data_calls = Cell::new(0);
        let extension_calls = Cell::new(0);

        let result = super::refloat_package_start(
            || {
                state_calls.set(state_calls.get() + 1);
                assert_eq!(thread_calls.get(), 0);
                assert_eq!(app_data_calls.get(), 0);
                assert_eq!(extension_calls.get(), 0);
                true
            },
            || {
                thread_calls.set(thread_calls.get() + 1);
                assert_eq!(state_calls.get(), 1);
                assert_eq!(app_data_calls.get(), 0);
                assert_eq!(extension_calls.get(), 0);
                true
            },
            || {
                app_data_calls.set(app_data_calls.get() + 1);
                assert_eq!(state_calls.get(), 1);
                assert_eq!(thread_calls.get(), 1);
                assert_eq!(extension_calls.get(), 0);
                true
            },
            || {
                extension_calls.set(extension_calls.get() + 1);
                assert_eq!(state_calls.get(), 1);
                assert_eq!(thread_calls.get(), 1);
                assert_eq!(app_data_calls.get(), 1);
                true
            },
        );

        assert!(result);
        assert_eq!(state_calls.get(), 1);
        assert_eq!(thread_calls.get(), 1);
        assert_eq!(app_data_calls.get(), 1);
        assert_eq!(extension_calls.get(), 1);
    }

    #[test]
    fn refloat_startup_fails_when_state_install_fails() {
        assert!(!super::refloat_package_start(
            || false,
            || true,
            || true,
            || true
        ));
    }

    #[test]
    fn refloat_startup_fails_when_runtime_thread_start_fails() {
        assert!(!super::refloat_package_start(
            || true,
            || false,
            || true,
            || true
        ));
    }

    #[test]
    fn refloat_startup_fails_when_app_data_callbacks_fail() {
        assert!(!super::refloat_package_start(
            || true,
            || true,
            || false,
            || true
        ));
    }

    #[test]
    fn refloat_startup_fails_when_loader_extensions_fail() {
        assert!(!super::refloat_package_start(
            || true,
            || true,
            || true,
            || false
        ));
    }
}
