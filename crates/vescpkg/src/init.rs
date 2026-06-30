//! Native VESC package loader helpers shared across package payloads.

use crate::ffi;

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        crate::ble_loopback::clear_loopback_app_data_handler();
    }

    #[cfg(test)]
    {
        record_stop_call_for_tests();
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

#[cfg(any(test, feature = "test-support"))]
mod test_state {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static INIT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static STOP_CALLS: AtomicUsize = AtomicUsize::new(0);

    pub fn record_init_call() {
        INIT_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn record_stop_call() {
        STOP_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    pub fn reset() {
        INIT_CALLS.store(0, Ordering::SeqCst);
        STOP_CALLS.store(0, Ordering::SeqCst);
    }

    pub fn init_calls() -> usize {
        INIT_CALLS.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn stop_calls() -> usize {
        STOP_CALLS.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
fn record_stop_call_for_tests() {
    test_state::record_stop_call();
}

/// Test helper that mirrors the device `package_lib_init` stop-hook path.
#[cfg(any(test, feature = "test-support"))]
pub fn init_for_tests(info: *mut ffi::LibInfo) -> bool {
    let _ = install_stop_hook(info);
    test_state::record_init_call();
    true
}

/// Resets the package init call counter used by tests.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_init_call_count_for_tests() {
    test_state::reset();
}

/// Returns how many times the package init entrypoint has been called in tests.
#[cfg(any(test, feature = "test-support"))]
pub fn init_call_count_for_tests() -> usize {
    test_state::init_calls()
}

/// Returns how many times the package stop hook has been called in tests.
#[cfg(test)]
pub fn stop_call_count_for_tests() -> usize {
    test_state::stop_calls()
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
