//! Firmware thread bindings for native package runtime code.
//!
//! Refloat v1.2.1 uses the VESC thread ABI declared in
//! `vesc_pkg_lib/vesc_c_if.h:376` and `382-384` for startup, stop, sleep, and
//! worker loops.

use core::ffi::{CStr, c_char, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::time::Duration;

use crate::bindings::AppDataBindings;
#[cfg(not(test))]
use crate::extension::LispValue;
#[cfg(not(test))]
use crate::lifecycle_core::AppDataHandlerRegistrationError;
use crate::lifecycle_core::AppDataSendError;
use crate::types::ThreadPriority;
use crate::units::TimestampTicks;

mod private {
    pub trait FirmwareThreads {}
}

/// Native package thread entrypoint shape.
pub(crate) type ThreadEntry = unsafe extern "C" fn(*mut c_void);

/// Typed firmware app-data capability available to package code.
pub struct FirmwareAppData {
    #[cfg(not(test))]
    api: AppDataApi<crate::bindings::RealBindings>,
}

impl FirmwareAppData {
    #[cfg(not(test))]
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            api: AppDataApi::new(crate::bindings::RealBindings),
        }
    }

    /// Return the current firmware system time in ticks.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn system_time_ticks(&self) -> TimestampTicks {
        self.api.system_time_ticks()
    }

    /// Send one app-data response.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn send(&self, bytes: &[u8]) -> Result<(), AppDataSendError> {
        self.api.send(bytes)
    }
}

/// Typed LispBM capability available to package code.
pub struct FirmwareLisp {
    #[cfg(not(test))]
    api: crate::lifecycle_core::LbmApi<crate::bindings::RealBindings>,
}

impl FirmwareLisp {
    #[cfg(not(test))]
    pub(crate) fn new() -> Self {
        Self {
            api: crate::lifecycle_core::LbmApi::new(crate::bindings::RealBindings),
        }
    }

    /// Decode a LispBM integer value.
    #[cfg(not(test))]
    pub fn decode_i32(&self, value: LispValue) -> i32 {
        crate::lifecycle_core::LbmApi::decode_i32(&self.api, value.raw())
    }

    /// Return LispBM's true value.
    #[cfg(not(test))]
    pub fn encode_true(&self) -> LispValue {
        LispValue::from_raw(crate::lifecycle_core::LbmApi::encode_true(&self.api))
    }

    /// Return LispBM's nil value.
    #[cfg(not(test))]
    pub fn encode_nil(&self) -> LispValue {
        LispValue::from_raw(crate::lifecycle_core::LbmApi::encode_nil(&self.api))
    }
}

/// Internal firmware app-data API built on a binding implementation.
pub(crate) struct AppDataApi<B> {
    bindings: B,
}

impl<B: AppDataBindings> AppDataApi<B> {
    /// Construct a new firmware app-data API wrapper.
    pub(crate) fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the current firmware system time in ticks.
    pub(crate) fn system_time_ticks(&self) -> TimestampTicks {
        TimestampTicks::from_ticks(self.bindings.system_time_ticks())
    }

    /// Send one app-data response.
    pub(crate) fn send(&self, bytes: &[u8]) -> Result<(), AppDataSendError> {
        self.bindings
            .send_app_data_bytes(bytes)
            .then_some(())
            .ok_or(AppDataSendError::PayloadTooLarge)
    }
}

/// Typed access to the firmware capabilities available to package threads.
pub struct Firmware {
    #[cfg(not(test))]
    threads: ThreadApi<RealThreadBindings>,
    #[cfg(not(test))]
    app_data: FirmwareAppData,
    #[cfg(not(test))]
    lisp: FirmwareLisp,
    #[cfg(not(test))]
    gpio: crate::Gpio,
    #[cfg(not(test))]
    imu: crate::imu::ImuApi<crate::imu::RealImuBindings>,
    #[cfg(not(test))]
    telemetry: crate::motor::MotorTelemetryApi<crate::motor::RealMotorTelemetryBindings>,
    #[cfg(not(test))]
    motor: crate::motor::MotorControlApi<crate::motor::RealMotorControlBindings>,
}

impl Firmware {
    /// Borrow firmware thread capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn threads(&self) -> &impl FirmwareThreads {
        &self.threads
    }

    /// Borrow firmware app-data capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn app_data(&self) -> &FirmwareAppData {
        &self.app_data
    }

    /// Clear package-owned IMU, app-data, and custom-config callbacks.
    ///
    /// C map: Refloat clears those callbacks in that order at
    /// `third_party/refloat/src/main.c:2401-2403`.
    #[cfg(not(test))]
    pub fn clear_package_callbacks(&self) -> Result<(), AppDataHandlerRegistrationError> {
        use crate::bindings::{AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings};

        let bindings = crate::bindings::RealBindings;
        bindings.clear_imu_read_callback_handler();
        // SAFETY: `Firmware` is only constructed after package startup has installed VESC_IF.
        let app_data_cleared = unsafe { bindings.clear_app_data_handler() };
        (bindings.clear_custom_config_callbacks() && app_data_cleared)
            .then_some(())
            .ok_or(AppDataHandlerRegistrationError::FirmwareRejected)
    }

    /// Borrow typed LispBM capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn lisp(&self) -> &FirmwareLisp {
        &self.lisp
    }

    /// Borrow firmware GPIO capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn gpio(&self) -> &crate::Gpio {
        &self.gpio
    }

    /// Borrow firmware IMU capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn imu(&self) -> &impl crate::Imu {
        &self.imu
    }

    /// Borrow firmware motor telemetry capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn telemetry(&self) -> &impl crate::MotorTelemetry {
        &self.telemetry
    }

    /// Borrow firmware motor-control capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn motor(&self) -> &impl crate::MotorOutput {
        &self.motor
    }

    /// Construct firmware capabilities backed by the live VESC package ABI.
    #[cfg(not(test))]
    pub fn new() -> Self {
        Self {
            threads: ThreadApi::new(RealThreadBindings),
            app_data: FirmwareAppData::new(),
            lisp: FirmwareLisp::new(),
            gpio: crate::Gpio::new(),
            imu: crate::imu::ImuApi::new(crate::imu::RealImuBindings),
            telemetry: crate::motor::MotorTelemetryApi::new(
                crate::motor::RealMotorTelemetryBindings,
            ),
            motor: crate::motor::MotorControlApi::from_firmware(
                crate::motor::RealMotorControlBindings,
            ),
        }
    }

    #[cfg(test)]
    fn test() -> Self {
        Self {}
    }
}

#[cfg(not(test))]
impl Default for Firmware {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime context passed to firmware package threads that do not need package state.
pub struct StatelessThreadContext {
    #[cfg(not(test))]
    threads: ThreadApi<RealThreadBindings>,
}

impl StatelessThreadContext {
    /// Borrow firmware thread capabilities without exposing the binding type.
    #[cfg(not(test))]
    pub fn threads(&self) -> &impl FirmwareThreads {
        &self.threads
    }

    /// Build a stateless thread context backed by the live VESC package ABI.
    #[cfg(not(test))]
    pub fn new() -> Self {
        Self {
            threads: ThreadApi::new(RealThreadBindings),
        }
    }

    #[cfg(test)]
    fn test() -> Self {
        Self {}
    }
}

#[cfg(not(test))]
impl Default for StatelessThreadContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime context passed to a typed firmware package thread.
pub struct ThreadContext<'a, S: 'static> {
    state: &'a mut S,
    firmware: Firmware,
}

impl<'a, S: 'static> ThreadContext<'a, S> {
    /// Build a thread context from explicit state and firmware capabilities.
    pub fn new(state: &'a mut S, firmware: Firmware) -> Self {
        Self { state, firmware }
    }

    /// Build a thread context backed by the live VESC package ABI.
    #[cfg(not(test))]
    fn from_entry(state: &'a mut S) -> Self {
        Self::new(state, Firmware::new())
    }

    #[cfg(test)]
    fn test(state: &'a mut S) -> Self {
        Self::new(state, Firmware::test())
    }

    /// Return mutable package state.
    pub fn state(&mut self) -> &mut S {
        self.state
    }

    /// Return firmware capabilities for this package thread.
    pub fn firmware(&self) -> &Firmware {
        &self.firmware
    }

    /// Split the context into package state and firmware capabilities.
    pub fn into_parts(self) -> (&'a mut S, Firmware) {
        (self.state, self.firmware)
    }
}

/// Rust implementation for a firmware package thread.
pub trait FirmwareThread {
    /// Package state type passed as the thread argument.
    type State: 'static;

    /// Run the thread body.
    fn run(ctx: ThreadContext<'_, Self::State>);
}

/// Rust implementation for a firmware package thread that does not need package state.
pub trait StatelessFirmwareThread {
    /// Run the thread body.
    fn run(ctx: StatelessThreadContext);
}

/// Firmware ABI trampoline for a typed package thread.
///
/// # Safety
///
/// `arg` must be null or point to a valid `T::State` value with exclusive access for the duration
/// of this call.
pub(crate) unsafe extern "C" fn firmware_thread_entry<T: FirmwareThread>(arg: *mut c_void) {
    let Some(state) = (unsafe { crate::arg_mut::<T::State>(arg) }) else {
        return;
    };
    #[cfg(test)]
    T::run(ThreadContext::test(state));
    #[cfg(not(test))]
    T::run(ThreadContext::from_entry(state));
}

/// Firmware ABI trampoline for a typed package thread without package state.
///
/// # Safety
///
/// `arg` is ignored. The firmware must still call this with the native package
/// thread ABI.
pub(crate) unsafe extern "C" fn stateless_firmware_thread_entry<T: StatelessFirmwareThread>(
    _arg: *mut c_void,
) {
    #[cfg(test)]
    T::run(StatelessThreadContext::test());
    #[cfg(not(test))]
    T::run(StatelessThreadContext::new());
}

/// Firmware-owned native package thread handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadHandle(NonNull<c_void>);

impl ThreadHandle {
    pub(crate) unsafe fn from_firmware(thread: *mut c_void) -> Option<Self> {
        NonNull::new(thread).map(Self)
    }

    /// Return the raw firmware thread handle.
    pub(crate) const fn as_ptr(self) -> *mut c_void {
        self.0.as_ptr()
    }
}

/// A static name assigned to a firmware thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadName(&'static [u8]);

impl ThreadName {
    /// Build a name from the terminated storage generated by `thread_name!`.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_terminated(name: &'static str) -> Option<Self> {
        match CStr::from_bytes_with_nul(name.as_bytes()) {
            Ok(_) => Some(Self(name.as_bytes())),
            Err(_) => None,
        }
    }

    /// Return the name without its private ABI terminator.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        let bytes = &self.0[..self.0.len() - 1];
        // SAFETY: the macro support hook accepts only valid C strings built
        // from UTF-8 Rust string literals.
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }

    pub(crate) const fn as_cstr(self) -> &'static CStr {
        // SAFETY: the macro support hook validates the terminating NUL byte.
        unsafe { CStr::from_bytes_with_nul_unchecked(self.0) }
    }
}

/// Failure returned when firmware rejects a thread operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadError {
    /// The requested thread priority is not supported by this firmware.
    PriorityUnsupported,
}

/// Create a checked static firmware thread name from a Rust string literal.
#[macro_export]
macro_rules! thread_name {
    ($name:literal) => {
        const {
            match $crate::ThreadName::__from_terminated(concat!($name, "\0")) {
                Some(name) => name,
                None => panic!("thread name literal must not contain NUL"),
            }
        }
    };
}

/// Stack size passed to a firmware thread in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadStackSize(usize);

impl ThreadStackSize {
    /// Build a stack size from its firmware byte count.
    #[must_use]
    pub const fn from_bytes(bytes: usize) -> Self {
        Self(bytes)
    }

    pub(crate) const fn bytes(self) -> usize {
        self.0
    }
}

/// Typed firmware thread spawn settings.
#[derive(Debug)]
pub struct ThreadSpec<S: 'static> {
    entry: ThreadEntry,
    stack_size: ThreadStackSize,
    name: ThreadName,
    _state: PhantomData<fn(&mut S)>,
}

impl<S: 'static> Copy for ThreadSpec<S> {}

impl<S: 'static> Clone for ThreadSpec<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static> ThreadSpec<S> {
    /// Build spawn settings for a typed firmware thread.
    pub fn new<T>(stack_size: ThreadStackSize, name: ThreadName) -> Self
    where
        T: FirmwareThread<State = S>,
    {
        Self::from_entry(firmware_thread_entry::<T>, stack_size, name)
    }

    /// Build spawn settings for a typed firmware thread that ignores package state.
    pub fn stateless<T>(stack_size: ThreadStackSize, name: ThreadName) -> Self
    where
        T: StatelessFirmwareThread,
    {
        Self::from_entry(stateless_firmware_thread_entry::<T>, stack_size, name)
    }

    /// Build spawn settings from a raw firmware thread entrypoint.
    pub(crate) const fn from_entry(
        entry: ThreadEntry,
        stack_size: ThreadStackSize,
        name: ThreadName,
    ) -> Self {
        Self {
            entry,
            stack_size,
            name,
            _state: PhantomData,
        }
    }
}

/// Pair of package-owned firmware thread handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadPair {
    first: Option<ThreadHandle>,
    second: Option<ThreadHandle>,
}

/// Typed firmware thread-pair spawn settings.
#[derive(Debug, Clone, Copy)]
pub struct ThreadPairSpec<S: 'static> {
    first: ThreadSpec<S>,
    second: ThreadSpec<()>,
}

impl<S: 'static> ThreadPairSpec<S> {
    /// Build a pair with one stateful thread and one stateless thread.
    pub const fn new(first: ThreadSpec<S>, second: ThreadSpec<()>) -> Self {
        Self { first, second }
    }

    /// Return the first thread spec.
    pub const fn first(self) -> ThreadSpec<S> {
        self.first
    }

    /// Return the second thread spec.
    pub const fn second(self) -> ThreadSpec<()> {
        self.second
    }
}

impl ThreadPair {
    /// Return an empty thread-handle set.
    pub const fn empty() -> Self {
        Self {
            first: None,
            second: None,
        }
    }

    /// Build a complete thread-handle pair.
    pub const fn new(first: ThreadHandle, second: ThreadHandle) -> Self {
        Self {
            first: Some(first),
            second: Some(second),
        }
    }

    const fn with_first(first: ThreadHandle) -> Self {
        Self {
            first: Some(first),
            second: None,
        }
    }

    /// Return the first thread handle.
    pub const fn first(self) -> Option<ThreadHandle> {
        self.first
    }

    /// Return the second thread handle.
    pub const fn second(self) -> Option<ThreadHandle> {
        self.second
    }

    /// Request thread termination from second to first.
    pub fn terminate_reverse(self, threads: &impl FirmwareThreads) {
        if let Some(second) = self.second {
            threads.request_terminate(second);
        }
        if let Some(first) = self.first {
            threads.request_terminate(first);
        }
    }
}

/// Typed firmware thread operations available to package code.
pub trait FirmwareThreads: private::FirmwareThreads {
    /// Spawn a stateful thread followed by a stateless thread.
    ///
    /// # Safety
    ///
    /// `state` must remain valid until the first thread exits or is terminated.
    unsafe fn spawn_thread_pair_with_state<S>(
        &self,
        pair: ThreadPairSpec<S>,
        state: &mut S,
    ) -> Option<ThreadPair>;

    /// Ask a firmware thread to terminate.
    fn request_terminate(&self, thread: ThreadHandle);

    /// Return whether the current package thread should terminate.
    fn should_terminate(&self) -> bool;

    /// Sleep the current package thread for a duration.
    fn sleep_for(&self, duration: Duration);

    /// Set the current package thread priority when supported by firmware.
    fn set_priority(&self, priority: ThreadPriority) -> Result<(), ThreadError>;
}

impl Default for ThreadPair {
    fn default() -> Self {
        Self::empty()
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
    fn request_terminate(&self, thread: ThreadHandle);

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

    fn request_terminate(&self, thread: ThreadHandle) {
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
        unsafe { crate::ffi::vesc_spawn(entry, stack_bytes, name, arg) }
    }

    fn request_terminate(&self, thread: ThreadHandle) {
        unsafe { crate::ffi::vesc_request_terminate(thread.as_ptr()) };
    }

    fn should_terminate(&self) -> bool {
        unsafe { crate::ffi::vesc_should_terminate() }
    }

    fn sleep_us(&self, micros: u32) {
        unsafe { crate::ffi::vesc_sleep_us(micros) };
    }

    fn set_priority(&self, priority: ThreadPriority) -> bool {
        unsafe { crate::ffi::vesc_thread_set_priority(priority.as_i8().into()) }
    }
}

/// High-level firmware thread API built on a binding implementation.
pub struct ThreadApi<B> {
    bindings: B,
}

impl<B: ThreadBindings> private::FirmwareThreads for ThreadApi<B> {}

impl<B: ThreadBindings> FirmwareThreads for ThreadApi<B> {
    unsafe fn spawn_thread_pair_with_state<S>(
        &self,
        pair: ThreadPairSpec<S>,
        state: &mut S,
    ) -> Option<ThreadPair> {
        unsafe { self.spawn_thread_pair_with_state(pair, state) }
    }

    fn request_terminate(&self, thread: ThreadHandle) {
        self.request_terminate(thread);
    }

    fn should_terminate(&self) -> bool {
        self.should_terminate()
    }

    fn sleep_for(&self, duration: Duration) {
        self.sleep_for(duration);
    }

    fn set_priority(&self, priority: ThreadPriority) -> Result<(), ThreadError> {
        self.set_priority(priority)
    }
}

impl<B: ThreadBindings> ThreadApi<B> {
    /// Construct a new firmware thread API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Spawn firmware threads in order while preserving the first on a second-spawn failure.
    ///
    /// C map: Refloat passes its position-independent thread and string addresses
    /// directly to spawn at third_party/refloat/src/main.c:2438-2444.
    ///
    /// # Safety
    ///
    /// State is passed only to the first firmware thread. It must remain valid
    /// until that thread exits or is terminated.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) unsafe fn spawn_thread_pair_with_state<S>(
        &self,
        pair: ThreadPairSpec<S>,
        state: &mut S,
    ) -> Option<ThreadPair> {
        let ThreadPairSpec { first, second } = pair;
        let arg = core::ptr::from_mut(state).cast::<c_void>();
        let spawn = |entry, stack_size: ThreadStackSize, name: ThreadName, arg| {
            // C map: lispif_spawn consumes the runtime entry, name, and
            // argument addresses unchanged at
            // third_party/vesc/lispBM/lispif_c_lib.c:98-125.
            let thread = unsafe {
                self.bindings
                    .spawn(entry, stack_size.bytes(), name.as_cstr().as_ptr(), arg)
            };
            unsafe { ThreadHandle::from_firmware(thread) }
        };

        let first_handle = spawn(first.entry, first.stack_size, first.name, arg)?;
        let Some(second_handle) = spawn(
            second.entry,
            second.stack_size,
            second.name,
            core::ptr::null_mut(),
        ) else {
            return Some(ThreadPair::with_first(first_handle));
        };

        Some(ThreadPair::new(first_handle, second_handle))
    }

    /// Ask a firmware thread to terminate.
    pub fn request_terminate(&self, thread: ThreadHandle) {
        self.bindings.request_terminate(thread);
    }

    /// Return whether the current package thread should terminate.
    pub fn should_terminate(&self) -> bool {
        self.bindings.should_terminate()
    }

    /// Sleep the current package thread for a duration.
    ///
    /// Refloat's main runtime thread sleeps with `VESC_IF->sleep_us` at
    /// `src/main.c:1080`.
    pub fn sleep_for(&self, duration: Duration) {
        let micros = duration.as_micros().min(u128::from(u32::MAX)) as u32;
        self.bindings.sleep_us(micros);
    }

    /// Set the current package thread priority when supported by firmware.
    ///
    /// Refloat lowers `aux_thd` priority with optional
    /// `VESC_IF->thread_set_priority(-1)` at `src/main.c:1133-1135`; the ABI
    /// slot is declared at `vesc_pkg_lib/vesc_c_if.h:670`.
    pub fn set_priority(&self, priority: ThreadPriority) -> Result<(), ThreadError> {
        self.bindings
            .set_priority(priority)
            .then_some(())
            .ok_or(ThreadError::PriorityUnsupported)
    }
}

#[cfg(test)]
/// Thread test support.
pub mod test_support {
    use super::{ThreadBindings, ThreadEntry, ThreadHandle};
    use crate::types::ThreadPriority;
    use core::cell::Cell;
    use core::ffi::{c_char, c_void};

    /// Private fake binding implementation used by SDK tests.
    pub(crate) struct FakeThreadBindings {
        /// Number of spawn calls observed.
        pub(crate) spawn_calls: Cell<usize>,
        /// Number of terminate calls observed.
        pub(crate) terminate_calls: Cell<usize>,
        /// Number of should-terminate calls observed.
        pub(crate) should_terminate_calls: Cell<usize>,
        /// Number of sleep calls observed.
        pub(crate) sleep_calls: Cell<usize>,
        /// Number of priority calls observed.
        pub(crate) priority_calls: Cell<usize>,
        /// Spawn stack sizes by call order.
        pub(crate) spawn_stacks: Cell<[usize; 2]>,
        /// Spawn names by call order.
        pub(crate) spawn_names: Cell<[*const c_char; 2]>,
        /// Spawn args by call order.
        pub(crate) spawn_args: Cell<[usize; 2]>,
        /// Spawn entries by call order.
        pub(crate) spawn_entries: Cell<[usize; 2]>,
        /// Terminated thread handles by call order.
        pub(crate) terminated_threads: Cell<[usize; 2]>,
        /// Sleep durations by call order.
        pub(crate) sleep_micros: Cell<[u32; 2]>,
        /// Thread priorities by call order.
        pub(crate) priorities: Cell<[i8; 2]>,
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

        fn request_terminate(&self, thread: ThreadHandle) {
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
            priorities[index] = priority.as_i8();
            self.priorities.set(priorities);
            self.priority_result.get()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppDataApi, FirmwareThread, StatelessFirmwareThread, StatelessThreadContext, ThreadApi,
        ThreadContext, ThreadHandle, ThreadPairSpec, ThreadSpec, ThreadStackSize,
        firmware_thread_entry, stateless_firmware_thread_entry,
    };
    use core::ffi::CStr;
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::boxed::Box;

    use crate::thread::test_support::FakeThreadBindings;
    use crate::units::TimestampTicks;

    #[test]
    fn semantic_app_data_capability_hides_binding_shape() {
        let bindings = crate::test_support::FakeAppDataBindings::new();
        let app_data = AppDataApi::new(&bindings);

        assert_eq!(app_data.system_time_ticks(), TimestampTicks::from_ticks(0));
        assert_eq!(app_data.send(&[1, 2, 3]), Ok(()));
        assert_eq!(bindings.send_calls.get(), 1);
        assert_eq!(bindings.last_len.get(), 3);
    }

    #[test]
    fn semantic_thread_capability_hides_binding_shape() {
        let bindings = FakeThreadBindings::new();
        let threads = ThreadApi::new(&bindings);

        fn accepts_semantic_threads(threads: &impl super::FirmwareThreads) -> bool {
            threads.should_terminate()
        }

        assert!(!accepts_semantic_threads(&threads));
    }

    static RUN_CALLS: AtomicUsize = AtomicUsize::new(0);
    static OBSERVED_STATE: AtomicU32 = AtomicU32::new(0);

    struct RecordingThread;
    struct RecordingStatelessThread;

    impl FirmwareThread for RecordingThread {
        type State = u32;

        fn run(mut ctx: ThreadContext<'_, Self::State>) {
            RUN_CALLS.fetch_add(1, Ordering::SeqCst);
            *ctx.state() += 1;
            OBSERVED_STATE.store(*ctx.state(), Ordering::SeqCst);
        }
    }

    impl StatelessFirmwareThread for RecordingStatelessThread {
        fn run(_ctx: StatelessThreadContext) {
            RUN_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn firmware_thread_entry_returns_without_state() {
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);

        unsafe { firmware_thread_entry::<RecordingThread>(core::ptr::null_mut()) };

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn firmware_thread_entry_passes_typed_state_through_context() {
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);
        let state = Box::leak(Box::new(41_u32));

        unsafe {
            firmware_thread_entry::<RecordingThread>(core::ptr::from_mut(state).cast::<c_void>());
        }

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(*state, 42);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 42);
    }

    #[test]
    fn stateless_firmware_thread_entry_ignores_raw_arg() {
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);

        unsafe {
            stateless_firmware_thread_entry::<RecordingStatelessThread>(core::ptr::null_mut())
        };

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn stateless_firmware_thread_entry_ignores_nonnull_raw_arg() {
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);

        unsafe {
            stateless_firmware_thread_entry::<RecordingStatelessThread>(
                core::ptr::without_provenance_mut::<c_void>(0x1234),
            );
        }

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn stateless_thread_context_can_run_without_firmware_context() {
        RUN_CALLS.store(0, Ordering::SeqCst);

        RecordingStatelessThread::run(StatelessThreadContext::test());

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn thread_pair_spec_passes_state_only_to_the_stateful_thread() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0x2000]);
        let threads = ThreadApi::new(&bindings);
        let first_name = crate::thread_name!("first");
        let second_name = crate::thread_name!("second");
        let pair = ThreadPairSpec::new(
            ThreadSpec::from_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadStackSize::from_bytes(128),
                first_name,
            ),
            ThreadSpec::from_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadStackSize::from_bytes(256),
                second_name,
            ),
        );
        let entries = [pair.first().entry as usize, pair.second().entry as usize];
        let mut state = 7_u32;
        let state_arg = core::ptr::from_mut(&mut state).cast::<c_void>() as usize;

        let handles = unsafe { threads.spawn_thread_pair_with_state(pair, &mut state) };

        assert_eq!(
            handles.map(|pair| (pair.first(), pair.second())),
            Some((
                unsafe { ThreadHandle::from_firmware(0x1000 as *mut c_void) },
                unsafe { ThreadHandle::from_firmware(0x2000 as *mut c_void) },
            ))
        );
        assert_eq!(bindings.spawn_calls.get(), 2);
        assert_eq!(bindings.spawn_entries.get(), entries);
        assert_eq!(bindings.spawn_stacks.get(), [128, 256]);
        assert_eq!(
            bindings
                .spawn_names
                .get()
                .map(|name| unsafe { CStr::from_ptr(name) }),
            [first_name.as_cstr(), second_name.as_cstr()]
        );
        assert_eq!(bindings.spawn_args.get(), [state_arg, 0]);
        assert_eq!(bindings.terminate_calls.get(), 0);
    }

    #[test]
    fn thread_pair_spec_preserves_first_when_second_spawn_fails() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0]);
        let threads = ThreadApi::new(&bindings);
        let pair = ThreadPairSpec::new(
            ThreadSpec::from_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadStackSize::from_bytes(128),
                crate::thread_name!("first"),
            ),
            ThreadSpec::from_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadStackSize::from_bytes(256),
                crate::thread_name!("second"),
            ),
        );
        let mut state = 7_u32;

        let handles = unsafe { threads.spawn_thread_pair_with_state(pair, &mut state) };

        assert_eq!(
            handles.map(|pair| (pair.first(), pair.second())),
            Some((
                unsafe { ThreadHandle::from_firmware(0x1000 as *mut c_void) },
                None,
            ))
        );
        assert_eq!(bindings.spawn_calls.get(), 2);
        assert_eq!(bindings.terminate_calls.get(), 0);
    }

    #[test]
    fn thread_name_exposes_rust_text_without_abi_terminator() {
        let name = crate::thread_name!("Refloat Main");

        assert_eq!(name.as_str(), "Refloat Main");
        assert_eq!(
            super::ThreadName::__from_terminated("Refloat Main\0").map(super::ThreadName::as_str),
            Some("Refloat Main")
        );
        assert!(super::ThreadName::__from_terminated("Refloat Main").is_none());
        assert!(super::ThreadName::__from_terminated("Refloat\0 Main\0").is_none());
    }
}
