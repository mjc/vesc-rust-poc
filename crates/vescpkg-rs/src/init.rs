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
fn install_stop_hook(info: *mut ffi::LibInfo) -> bool {
    let Some(info) = crate::loader_info_mut(info) else {
        return false;
    };
    info.stop_fun = Some(crate::firmware::stop_handler_for_loader(info, stop_package));
    true
}

/// Safe startup context for package authors.
pub struct PackageStart {
    info: *mut ffi::LibInfo,
}

impl PackageStart {
    /// Build a startup context from the firmware ABI pointer.
    #[doc(hidden)]
    pub fn from_raw(info: *mut ffi::LibInfo) -> Self {
        Self { info }
    }

    /// Install the default package stop hook into loader metadata.
    pub fn install_stop_hook(&mut self) -> bool {
        install_stop_hook(self.info)
    }

    /// Borrow loader metadata when startup code needs loader-owned state.
    pub fn loader_info_mut(&mut self) -> Option<&mut ffi::LibInfo> {
        crate::loader_info_mut(self.info)
    }

    /// Clear package state and stop metadata after a startup failure.
    pub fn clear_loader_info(&mut self) {
        crate::clear_loader_info(self.info);
    }

    /// Store package state and a stop hook in loader metadata.
    pub fn install_loader_state<T>(
        &mut self,
        stop_handler: ffi::StopHandler,
        state: &mut T,
    ) -> bool {
        if self.loader_info_mut().is_none() {
            return false;
        }
        crate::install_loader_state(self.info, stop_handler, state)
    }

    /// Allocate package state in firmware memory and store it in loader metadata.
    pub fn allocate_loader_state<A, T>(
        &mut self,
        allocator: &crate::FirmwareAllocator<'_, A>,
        stop_handler: ffi::StopHandler,
        state: T,
    ) -> bool
    where
        A: crate::AllocBindings,
    {
        let Ok(mut allocation) = allocator.allocate_for::<T>(1) else {
            self.clear_loader_info();
            return false;
        };
        let state = allocation.write_first(state);

        if !self.install_loader_state(stop_handler, state) {
            self.clear_loader_info();
            return false;
        }

        let _ = allocation.into_raw();
        true
    }

    /// Register extension descriptors using loader metadata for this package image.
    pub fn register_extensions_with<B: crate::LbmBindings>(
        &mut self,
        lifecycle: &crate::PackageLifecycle<B>,
        descriptors: impl IntoIterator<Item = crate::ExtensionDescriptor>,
    ) -> bool {
        let Some(info) = self.loader_info_mut() else {
            return false;
        };
        lifecycle
            .register_extensions_from_image(ffi::NativeImage::from_info(info), descriptors)
            .is_ok()
    }

    /// Register extension descriptors with the live firmware bindings.
    #[cfg(all(not(test), target_arch = "arm"))]
    pub fn register_extensions(
        &mut self,
        descriptors: impl IntoIterator<Item = crate::ExtensionDescriptor>,
    ) -> bool {
        let lifecycle = crate::PackageLifecycle::new(crate::RealBindings);
        self.register_extensions_with(&lifecycle, descriptors)
    }
}

/// One startup phase for a firmware package.
pub type PackageStartStep = fn(&mut PackageStart) -> bool;

/// Run package startup phases in order, stopping at the first failure.
pub fn start_package(start: &mut PackageStart, steps: &[PackageStartStep]) -> bool {
    for step in steps {
        if !step(start) {
            return false;
        }
    }

    true
}

/// Define the VESC firmware entrypoints for a package start function.
#[macro_export]
macro_rules! package_start {
    ($start:path) => {
        #[cfg(all(not(test), target_arch = "arm"))]
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".program_ptr")]
        pub(crate) static prog_ptr: u32 = 0;

        /// Firmware loader entrypoint that runs the package start function.
        #[cfg(any(test, all(not(test), target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        pub extern "C" fn package_lib_init(info: *mut $crate::ffi::LibInfo) -> bool {
            let mut start = $crate::PackageStart::from_raw(info);
            $start(&mut start)
        }

        /// Host-linking loader shim for package crates.
        #[cfg(all(not(test), not(target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        pub extern "C" fn package_lib_init(info: *mut $crate::ffi::LibInfo) -> bool {
            let mut start = $crate::PackageStart::from_raw(info);
            let _ = start.install_stop_hook();
            true
        }

        /// ARM package initializer placed in the firmware init section.
        #[cfg(all(not(test), target_arch = "arm"))]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".init_fun")]
        pub extern "C" fn init(info: *mut $crate::ffi::LibInfo) -> bool {
            package_lib_init(info)
        }
    };
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
    use core::cell::Cell;
    use core::ffi::c_void;
    use core::mem::MaybeUninit;

    struct TestAllocBindings {
        malloc_calls: Cell<usize>,
        free_calls: Cell<usize>,
        last_requested_len: Cell<usize>,
        next_ptr: Cell<*mut c_void>,
    }

    impl TestAllocBindings {
        fn new(next_ptr: *mut c_void) -> Self {
            Self {
                malloc_calls: Cell::new(0),
                free_calls: Cell::new(0),
                last_requested_len: Cell::new(0),
                next_ptr: Cell::new(next_ptr),
            }
        }

        fn failing() -> Self {
            Self::new(core::ptr::null_mut())
        }
    }

    impl crate::AllocBindings for TestAllocBindings {
        unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
            self.malloc_calls.set(self.malloc_calls.get() + 1);
            self.last_requested_len.set(bytes);
            self.next_ptr.get()
        }

        unsafe fn free(&self, _ptr: *mut c_void) {
            self.free_calls.set(self.free_calls.get() + 1);
        }
    }

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

    #[test]
    fn start_package_runs_steps_until_failure() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);

        fn first(_start: &mut super::PackageStart) -> bool {
            CALLS.fetch_add(1, Ordering::SeqCst);
            true
        }

        fn second(_start: &mut super::PackageStart) -> bool {
            CALLS.fetch_add(10, Ordering::SeqCst);
            false
        }

        fn skipped(_start: &mut super::PackageStart) -> bool {
            CALLS.fetch_add(100, Ordering::SeqCst);
            true
        }

        CALLS.store(0, Ordering::SeqCst);
        let steps: [super::PackageStartStep; 3] = [first, second, skipped];
        let mut start = super::PackageStart::from_raw(core::ptr::null_mut());

        assert!(!super::start_package(&mut start, &steps));
        assert_eq!(CALLS.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn package_start_context_installs_loader_state_without_raw_pointer() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(start.install_loader_state(super::stop_package, &mut state));
        assert!(
            start
                .loader_info_mut()
                .is_some_and(|info| info.stop_fun.is_some())
        );
        let loaded =
            crate::loader_state_mut::<State>(start.loader_info_mut().expect("loader info"))
                .expect("loader state");
        assert_eq!(loaded.value, 42);

        start.clear_loader_info();
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_allocates_loader_state_in_firmware_memory() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(start.allocate_loader_state(&allocator, super::stop_package, State { value: 99 }));

        assert_eq!(bindings.malloc_calls.get(), 1);
        assert_eq!(
            bindings.last_requested_len.get(),
            core::mem::size_of::<State>()
        );
        assert_eq!(bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert!(info.stop_fun.is_some());
        let loaded = crate::loader_state_mut::<State>(&mut info).expect("loader state");
        assert_eq!(loaded.value, 99);
    }

    #[test]
    fn package_start_allocation_failure_clears_loader_metadata() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: Some(super::stop_package),
            arg: 0x1234_usize as *mut c_void,
            base_addr: 0,
        };
        let bindings = TestAllocBindings::failing();
        let allocator = crate::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(!start.allocate_loader_state(&allocator, super::stop_package, State { value: 7 }));

        assert_eq!(bindings.malloc_calls.get(), 1);
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_registers_extensions_from_loader_metadata() {
        use crate::test_support::{FakeBindings, stubs};

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let lifecycle = crate::PackageLifecycle::new(FakeBindings::new());
        let mut start = super::PackageStart::from_raw(&mut info);
        let descriptor =
            crate::ExtensionDescriptor::new(c"ext-start-probe", stubs::extension_handler);

        assert!(start.register_extensions_with(&lifecycle, [descriptor]));
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
        assert_eq!(
            lifecycle.bindings().last_name.get(),
            descriptor.name().as_ptr() as usize + 0x2000
        );
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            descriptor.handler() as usize + 0x2000
        );

        let mut rejected_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let rejecting_lifecycle = crate::PackageLifecycle::new(FakeBindings::rejecting());
        let mut rejecting_start = super::PackageStart::from_raw(&mut rejected_info);

        assert!(!rejecting_start.register_extensions_with(&rejecting_lifecycle, [descriptor]));
        assert_eq!(rejecting_lifecycle.bindings().add_calls.get(), 1);
    }
}
