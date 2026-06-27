//! Native VESC package loader wiring: stop hook and LispBM extension registration.

use crate::ffi;
use crate::package_lifecycle;

#[cfg(test)]
use core::cell::Cell;

const FIRMWARE_BINDINGS: ffi::RealBindings = ffi::RealBindings;

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[no_mangle]
#[link_section = ".program_ptr"]
static prog_ptr: u32 = 0;

#[cfg(all(not(test), target_arch = "arm"))]
#[no_mangle]
#[link_section = ".init_fun"]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    if !crate::package_lib_init(info) {
        return false;
    }

    let _registered = register_probe_extension(info);
    true
}

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
    #[cfg(not(test))]
    {
        let _ = ffi::LoopbackLifecycle::new(FIRMWARE_BINDINGS).clear_app_data_handler();
    }

    #[cfg(test)]
    {
        STOP_CALLS.with(|calls| calls.set(calls.get() + 1));
    }
}

/// Install the package stop hook into loader metadata.
pub fn install_stop_hook(info: *mut ffi::LibInfo) -> bool {
    if info.is_null() {
        return false;
    }

    if let Some(info) = unsafe { info.as_mut() } {
        info.stop_fun = Some(stop_package);
    }

    true
}

/// Register the device probe extension using the compact single-descriptor path.
///
/// The loader keeps this as one `register_extension_from_image` call so `.init_fun`
/// stays within the device-proven codegen budget.
pub fn register_probe_extension(info: *mut ffi::LibInfo) -> bool {
    if info.is_null() {
        return false;
    }

    unsafe {
        let image = ffi::NativeImage::from_info(&*info);
        let lifecycle = ffi::PackageLifecycle::new(ffi::RealBindings);
        lifecycle
            .register_extension_from_image(image, package_lifecycle::rust_probe_diag_descriptor())
            .is_ok()
    }
}

#[cfg(test)]
thread_local! {
    static INIT_CALLS: Cell<usize> = Cell::new(0);
    static STOP_CALLS: Cell<usize> = Cell::new(0);
}

#[cfg(test)]
pub fn init_for_tests(info: *mut ffi::LibInfo) -> bool {
    let _ = install_stop_hook(info);
    INIT_CALLS.with(|calls| calls.set(calls.get() + 1));
    true
}

#[cfg(test)]
pub fn reset_init_call_count_for_tests() {
    INIT_CALLS.with(|calls| calls.set(0));
    STOP_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
pub fn init_call_count_for_tests() -> usize {
    INIT_CALLS.with(|calls| calls.get())
}

#[cfg(test)]
pub fn stop_call_count_for_tests() -> usize {
    STOP_CALLS.with(|calls| calls.get())
}
