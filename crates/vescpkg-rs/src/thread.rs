//! Firmware thread bindings for native package runtime code.
//!
//! Refloat v1.2.1 uses the VESC thread ABI declared in
//! `vesc_pkg_lib/vesc_c_if.h:376` and `382-384` for startup, stop, sleep, and
//! worker loops.

use core::ffi::{CStr, c_char, c_void};
use core::ptr::NonNull;

use crate::types::ThreadPriority;

/// Native package thread entrypoint shape.
pub type ThreadEntry = unsafe extern "C" fn(*mut c_void);

/// Rust implementation for a firmware package thread.
pub trait FirmwareThread {
    /// Package state type passed as the thread argument.
    type State: 'static;

    /// Run the thread body.
    fn run(state: Option<&'static mut Self::State>);
}

/// Firmware ABI trampoline for a typed package thread.
///
/// # Safety
///
/// `arg` must be null or point to a valid `T::State` value with exclusive access for the duration
/// of this call.
pub unsafe extern "C" fn firmware_thread_entry<T: FirmwareThread>(arg: *mut c_void) {
    T::run(crate::arg_mut::<T::State>(arg));
}

/// Firmware-owned native package thread handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareThreadHandle(NonNull<c_void>);

impl FirmwareThreadHandle {
    /// Build a thread handle from a firmware-returned pointer.
    ///
    /// # Safety
    ///
    /// `thread` must be null or a handle returned by the VESC `spawn` ABI slot.
    pub unsafe fn from_raw(thread: *mut c_void) -> Option<Self> {
        NonNull::new(thread).map(Self)
    }

    /// Return the raw firmware thread handle.
    pub const fn as_ptr(self) -> *mut c_void {
        self.0.as_ptr()
    }
}

/// Binding surface for VESC native package thread functions.
pub trait ThreadBindings {
    /// Spawn a firmware thread.
    ///
    /// # Safety
    ///
    /// `entry`, `name`, and `arg` must remain valid for the duration required
    /// by the firmware `spawn` call. `arg` must point to state that outlives
    /// the spawned thread.
    unsafe fn spawn(
        &self,
        entry: ThreadEntry,
        stack_bytes: usize,
        name: *const c_char,
        arg: *mut c_void,
    ) -> *mut c_void;

    /// Ask a firmware thread to terminate.
    fn request_terminate(&self, thread: FirmwareThreadHandle);

    /// Return whether the current package thread should terminate.
    fn should_terminate(&self) -> bool;

    /// Sleep the current package thread for a number of microseconds.
    fn sleep_us(&self, micros: u32);

    /// Set the current package thread priority when firmware exposes the slot.
    fn set_priority(&self, priority: ThreadPriority) -> bool;
}

impl<B: ThreadBindings + ?Sized> ThreadBindings for &B {
    unsafe fn spawn(
        &self,
        entry: ThreadEntry,
        stack_bytes: usize,
        name: *const c_char,
        arg: *mut c_void,
    ) -> *mut c_void {
        unsafe { (*self).spawn(entry, stack_bytes, name, arg) }
    }

    fn request_terminate(&self, thread: FirmwareThreadHandle) {
        (*self).request_terminate(thread);
    }

    fn should_terminate(&self) -> bool {
        (*self).should_terminate()
    }

    fn sleep_us(&self, micros: u32) {
        (*self).sleep_us(micros);
    }

    fn set_priority(&self, priority: ThreadPriority) -> bool {
        (*self).set_priority(priority)
    }
}

#[cfg(not(test))]
/// Thread binding implementation that forwards to the live firmware ABI.
pub struct RealThreadBindings;

#[cfg(not(test))]
impl ThreadBindings for RealThreadBindings {
    unsafe fn spawn(
        &self,
        entry: ThreadEntry,
        stack_bytes: usize,
        name: *const c_char,
        arg: *mut c_void,
    ) -> *mut c_void {
        unsafe { vescpkg_rs_sys::raw::vesc_spawn(entry, stack_bytes, name, arg) }
    }

    fn request_terminate(&self, thread: FirmwareThreadHandle) {
        unsafe { vescpkg_rs_sys::raw::vesc_request_terminate(thread.as_ptr()) };
    }

    fn should_terminate(&self) -> bool {
        unsafe { vescpkg_rs_sys::raw::vesc_should_terminate() }
    }

    fn sleep_us(&self, micros: u32) {
        unsafe { vescpkg_rs_sys::raw::vesc_sleep_us(micros) };
    }

    fn set_priority(&self, priority: ThreadPriority) -> bool {
        unsafe { vescpkg_rs_sys::raw::vesc_thread_set_priority(priority.get().into()) }
    }
}

/// High-level firmware thread API built on a binding implementation.
pub struct ThreadApi<B> {
    bindings: B,
}

impl<B: ThreadBindings> ThreadApi<B> {
    /// Construct a new firmware thread API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the wrapped thread bindings.
    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// Spawn a firmware package thread.
    ///
    /// Refloat starts its main and auxiliary worker threads at
    /// `src/main.c:2439-2445`.
    ///
    /// # Safety
    ///
    /// `entry` must be a valid package-thread entrypoint. `name` must be a
    /// valid NUL-terminated C string that remains valid for the firmware
    /// `spawn` call. `arg` must point to state that lives until the spawned
    /// thread exits or is terminated.
    pub unsafe fn spawn(
        &self,
        entry: ThreadEntry,
        stack_bytes: usize,
        name: &CStr,
        arg: *mut c_void,
    ) -> Option<FirmwareThreadHandle> {
        let thread = unsafe { self.bindings.spawn(entry, stack_bytes, name.as_ptr(), arg) };
        unsafe { FirmwareThreadHandle::from_raw(thread) }
    }

    /// Spawn a firmware package thread with typed package state as its argument.
    pub fn spawn_with_state<T>(
        &self,
        entry: ThreadEntry,
        stack_bytes: usize,
        name: &CStr,
        state: &mut T,
    ) -> Option<FirmwareThreadHandle> {
        let arg = core::ptr::from_mut(state).cast::<c_void>();
        unsafe { self.spawn(entry, stack_bytes, name, arg) }
    }

    /// Ask a firmware thread to terminate.
    pub fn request_terminate(&self, thread: FirmwareThreadHandle) {
        self.bindings.request_terminate(thread);
    }

    /// Return whether the current package thread should terminate.
    pub fn should_terminate(&self) -> bool {
        self.bindings.should_terminate()
    }

    /// Sleep the current package thread for a number of microseconds.
    ///
    /// Refloat's main runtime thread sleeps with `VESC_IF->sleep_us` at
    /// `src/main.c:1080`.
    pub fn sleep_us(&self, micros: u32) {
        self.bindings.sleep_us(micros);
    }

    /// Set the current package thread priority when supported by firmware.
    ///
    /// Refloat lowers `aux_thd` priority with optional
    /// `VESC_IF->thread_set_priority(-1)` at `src/main.c:1133-1135`; the ABI
    /// slot is declared at `vesc_pkg_lib/vesc_c_if.h:670`.
    pub fn set_priority(&self, priority: ThreadPriority) -> bool {
        self.bindings.set_priority(priority)
    }
}

#[cfg(any(test, feature = "test-support"))]
/// Thread fake binding helpers exported for tests.
pub mod test_support {
    use super::{FirmwareThreadHandle, ThreadBindings, ThreadEntry};
    use crate::types::ThreadPriority;
    use core::cell::Cell;
    use core::ffi::{c_char, c_void};

    /// Fake thread binding implementation used by package tests.
    pub struct FakeThreadBindings {
        /// Number of spawn calls observed.
        pub spawn_calls: Cell<usize>,
        /// Number of terminate calls observed.
        pub terminate_calls: Cell<usize>,
        /// Number of should-terminate calls observed.
        pub should_terminate_calls: Cell<usize>,
        /// Number of sleep calls observed.
        pub sleep_calls: Cell<usize>,
        /// Number of priority calls observed.
        pub priority_calls: Cell<usize>,
        /// Spawn stack sizes by call order.
        pub spawn_stacks: Cell<[usize; 2]>,
        /// Spawn names by call order.
        pub spawn_names: Cell<[*const c_char; 2]>,
        /// Spawn args by call order.
        pub spawn_args: Cell<[usize; 2]>,
        /// Spawn entries by call order.
        pub spawn_entries: Cell<[usize; 2]>,
        /// Terminated thread handles by call order.
        pub terminated_threads: Cell<[usize; 2]>,
        /// Sleep durations by call order.
        pub sleep_micros: Cell<[u32; 2]>,
        /// Thread priorities by call order.
        pub priorities: Cell<[i8; 2]>,
        spawn_results: Cell<[usize; 2]>,
        should_terminate_result: Cell<bool>,
        should_terminate_after_calls: Cell<usize>,
        priority_result: Cell<bool>,
    }

    impl Default for FakeThreadBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeThreadBindings {
        /// Creates fake thread bindings returning two non-null handles.
        pub fn new() -> Self {
            Self::with_spawn_results([1, 2])
        }

        /// Creates fake thread bindings returning explicit raw spawn handles.
        pub fn with_spawn_results(spawn_results: [usize; 2]) -> Self {
            Self {
                spawn_calls: Cell::new(0),
                terminate_calls: Cell::new(0),
                should_terminate_calls: Cell::new(0),
                sleep_calls: Cell::new(0),
                priority_calls: Cell::new(0),
                spawn_stacks: Cell::new([0, 0]),
                spawn_names: Cell::new([core::ptr::null(), core::ptr::null()]),
                spawn_args: Cell::new([0, 0]),
                spawn_entries: Cell::new([0, 0]),
                terminated_threads: Cell::new([0, 0]),
                sleep_micros: Cell::new([0, 0]),
                priorities: Cell::new([0, 0]),
                spawn_results: Cell::new(spawn_results),
                should_terminate_result: Cell::new(false),
                should_terminate_after_calls: Cell::new(usize::MAX),
                priority_result: Cell::new(true),
            }
        }

        /// Creates fake thread bindings returning `should_terminate`.
        pub fn with_should_terminate(should_terminate: bool) -> Self {
            let bindings = Self::new();
            bindings.should_terminate_result.set(should_terminate);
            bindings
        }

        /// Creates fake thread bindings that terminate after `calls`.
        pub fn with_should_terminate_after_calls(calls: usize) -> Self {
            let bindings = Self::new();
            bindings.should_terminate_after_calls.set(calls);
            bindings
        }
    }

    impl ThreadBindings for FakeThreadBindings {
        unsafe fn spawn(
            &self,
            entry: ThreadEntry,
            stack_bytes: usize,
            name: *const c_char,
            arg: *mut c_void,
        ) -> *mut c_void {
            let call = self.spawn_calls.get();
            self.spawn_calls.set(call + 1);
            let index = call.min(1);

            let mut stacks = self.spawn_stacks.get();
            stacks[index] = stack_bytes;
            self.spawn_stacks.set(stacks);

            let mut names = self.spawn_names.get();
            names[index] = name;
            self.spawn_names.set(names);

            let mut args = self.spawn_args.get();
            args[index] = arg as usize;
            self.spawn_args.set(args);

            let mut entries = self.spawn_entries.get();
            entries[index] = entry as usize;
            self.spawn_entries.set(entries);

            self.spawn_results.get()[index] as *mut c_void
        }

        fn request_terminate(&self, thread: FirmwareThreadHandle) {
            let call = self.terminate_calls.get();
            self.terminate_calls.set(call + 1);
            let index = call.min(1);
            let mut threads = self.terminated_threads.get();
            threads[index] = thread.as_ptr() as usize;
            self.terminated_threads.set(threads);
        }

        fn should_terminate(&self) -> bool {
            let calls = self.should_terminate_calls.get() + 1;
            self.should_terminate_calls.set(calls);
            self.should_terminate_result.get() || calls >= self.should_terminate_after_calls.get()
        }

        fn sleep_us(&self, micros: u32) {
            let call = self.sleep_calls.get();
            self.sleep_calls.set(call + 1);
            let index = call.min(1);
            let mut sleeps = self.sleep_micros.get();
            sleeps[index] = micros;
            self.sleep_micros.set(sleeps);
        }

        fn set_priority(&self, priority: ThreadPriority) -> bool {
            let call = self.priority_calls.get();
            self.priority_calls.set(call + 1);
            let index = call.min(1);
            let mut priorities = self.priorities.get();
            priorities[index] = priority.get();
            self.priorities.set(priorities);
            self.priority_result.get()
        }
    }
}
