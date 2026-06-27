//! Native VESC package loader helpers shared across package payloads.

use crate::ffi;

#[cfg(test)]
use core::cell::Cell;

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
    #[cfg(not(test))]
    {
        let _ = ffi::LoopbackLifecycle::new(ffi::RealBindings).clear_app_data_handler();
    }

    #[cfg(test)]
    {
        STOP_CALLS.with(|calls| calls.set(calls.get() + 1));
    }
}

/// Install the package stop hook into loader metadata.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn install_stop_hook(info: *mut ffi::LibInfo) -> bool {
    if info.is_null() {
        return false;
    }

    if let Some(info) = unsafe { info.as_mut() } {
        info.stop_fun = Some(stop_package);
    }

    true
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    init_for_tests(info)
}

#[cfg(test)]
thread_local! {
    static INIT_CALLS: Cell<usize> = const { Cell::new(0) };
    static STOP_CALLS: Cell<usize> = const { Cell::new(0) };
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

#[cfg(test)]
mod tests {
    use super::{init_for_tests, install_stop_hook, reset_init_call_count_for_tests};
    use crate::ffi;

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

        let stop_fun = info.stop_fun.expect("stop hook");
        unsafe {
            stop_fun(info.arg);
        }
        assert_eq!(super::stop_call_count_for_tests(), 1);
    }

    #[test]
    fn install_stop_hook_rejects_null_loader_metadata() {
        assert!(!install_stop_hook(core::ptr::null_mut()));
    }
}
