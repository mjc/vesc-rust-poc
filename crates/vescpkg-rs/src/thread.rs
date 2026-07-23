//! Firmware thread bindings for native package runtime code.
//!
//! Float Out Boy v1.2.1 uses the VESC thread ABI declared in
//! `vesc_pkg_lib/vesc_c_if.h:376` and `382-384` for startup, stop, sleep, and
//! worker loops.

use core::ffi::{CStr, c_char, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::time::Duration;

use crate::PackageRuntimeState;
use crate::bindings::AppDataBindings;
use crate::lifecycle_core::AppDataSendError;
use crate::types::ThreadPriority;
use crate::units::TimestampTicks;
#[cfg(not(test))]
use crate::units::VescSeconds;

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

/// Opaque high-resolution firmware timer instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimerInstant(u32);

impl TimerInstant {
    /// Construct an instant from the firmware timer's raw counter.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[cfg(not(test))]
    const fn raw(self) -> u32 {
        self.0
    }
}

impl FirmwareAppData {
    #[cfg(not(test))]
    pub(crate) fn new() -> Self {
        Self {
            api: AppDataApi::new(crate::bindings::RealBindings),
        }
    }

    /// Send one app-data response.
    ///
    /// # Errors
    ///
    /// Returns [`AppDataSendError`] when the payload is too large or firmware rejects it.
    #[cfg(not(test))]
    pub fn send(&self, bytes: &[u8]) -> Result<(), AppDataSendError> {
        self.api.send(bytes)
    }
}

/// Firmware monotonic clock capability available to package code.
pub struct FirmwareClock {
    #[cfg(not(test))]
    api: AppDataApi<crate::bindings::RealBindings>,
}

impl FirmwareClock {
    #[cfg(not(test))]
    pub(crate) fn new() -> Self {
        Self {
            api: AppDataApi::new(crate::bindings::RealBindings),
        }
    }

    /// Return the current firmware system timestamp.
    #[cfg(not(test))]
    #[must_use]
    pub fn now(&self) -> TimestampTicks {
        self.api.system_timestamp()
    }

    /// Return firmware uptime in the native floating-point seconds domain.
    #[cfg(not(test))]
    #[must_use]
    pub fn uptime(&self) -> VescSeconds {
        self.api.system_uptime()
    }

    /// Ask firmware for the age of a timestamp, including its rollover rules.
    #[cfg(not(test))]
    #[must_use]
    pub fn age(&self, timestamp: TimestampTicks) -> VescSeconds {
        self.api.timestamp_age(timestamp)
    }

    /// Return the current high-resolution timer instant.
    #[cfg(not(test))]
    #[must_use]
    pub fn timer_now(&self) -> TimerInstant {
        self.api.timer_now()
    }

    /// Return high-resolution elapsed time since `earlier`.
    #[cfg(not(test))]
    #[must_use]
    pub fn timer_elapsed_since(&self, earlier: TimerInstant) -> VescSeconds {
        self.api.timer_elapsed_since(earlier)
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
    pub(crate) fn system_timestamp(&self) -> TimestampTicks {
        TimestampTicks::from_ticks(self.bindings.system_time_ticks())
    }

    /// Return firmware uptime in floating-point seconds.
    #[cfg(not(test))]
    fn system_uptime(&self) -> VescSeconds {
        VescSeconds::from_seconds(self.bindings.system_time_seconds())
    }

    /// Return firmware-computed age for a system timestamp.
    #[cfg(not(test))]
    fn timestamp_age(&self, timestamp: TimestampTicks) -> VescSeconds {
        VescSeconds::from_seconds(self.bindings.timestamp_age_seconds(timestamp.as_ticks()))
    }

    /// Return the current high-resolution timer instant.
    #[cfg(not(test))]
    fn timer_now(&self) -> TimerInstant {
        TimerInstant::from_raw(self.bindings.timer_time_now())
    }

    /// Return high-resolution elapsed time since a timer instant.
    #[cfg(not(test))]
    fn timer_elapsed_since(&self, earlier: TimerInstant) -> VescSeconds {
        VescSeconds::from_seconds(self.bindings.timer_seconds_elapsed_since(earlier.raw()))
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
    clock: FirmwareClock,
    #[cfg(not(test))]
    nvm: crate::Nvm,
    #[cfg(not(test))]
    gpio: crate::Gpio,
    #[cfg(not(test))]
    input: crate::ControllerInput,
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
    #[must_use]
    pub fn threads(&self) -> &impl FirmwareThreads {
        &self.threads
    }

    /// Borrow firmware app-data capabilities without exposing the binding type.
    #[cfg(not(test))]
    #[must_use]
    pub fn app_data(&self) -> &FirmwareAppData {
        &self.app_data
    }

    /// Borrow the firmware monotonic clock.
    #[cfg(not(test))]
    #[must_use]
    pub fn clock(&self) -> &FirmwareClock {
        &self.clock
    }

    /// Borrow the firmware byte-addressed NVM capability.
    #[cfg(not(test))]
    #[must_use]
    pub fn nvm(&self) -> &crate::Nvm {
        &self.nvm
    }

    /// Borrow firmware GPIO capabilities without exposing the binding type.
    #[cfg(not(test))]
    #[must_use]
    pub fn gpio(&self) -> &crate::Gpio {
        &self.gpio
    }

    /// Borrow typed PPM and UART controller inputs.
    #[cfg(not(test))]
    #[must_use]
    pub fn input(&self) -> &crate::ControllerInput {
        &self.input
    }

    /// Borrow firmware IMU capabilities without exposing the binding type.
    #[cfg(not(test))]
    #[must_use]
    pub fn imu(&self) -> &impl crate::Imu {
        &self.imu
    }

    /// Borrow firmware motor telemetry capabilities without exposing the binding type.
    #[cfg(not(test))]
    #[must_use]
    pub fn telemetry(&self) -> &impl crate::MotorTelemetry {
        &self.telemetry
    }

    /// Borrow firmware motor-control capabilities without exposing the binding type.
    #[cfg(not(test))]
    #[must_use]
    pub fn motor(&self) -> &impl crate::MotorOutput {
        &self.motor
    }

    /// Construct firmware capabilities backed by the live VESC package ABI.
    #[cfg(not(test))]
    #[must_use]
    pub fn new() -> Self {
        Self {
            threads: ThreadApi::new(RealThreadBindings),
            app_data: FirmwareAppData::new(),
            clock: FirmwareClock::new(),
            nvm: crate::Nvm::new(),
            gpio: crate::Gpio::new(),
            input: crate::ControllerInput::new(),
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
    #[must_use]
    pub fn threads(&self) -> &impl FirmwareThreads {
        &self.threads
    }

    /// Build a stateless thread context backed by the live VESC package ABI.
    #[cfg(not(test))]
    #[must_use]
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
pub struct ThreadContext<S: PackageRuntimeState> {
    state: NonNull<S>,
    firmware: Firmware,
}

impl<S: PackageRuntimeState> ThreadContext<S> {
    fn new(state: NonNull<S>, firmware: Firmware) -> Self {
        Self { state, firmware }
    }

    /// Build a thread context backed by the live VESC package ABI.
    #[cfg(not(test))]
    fn from_entry(state: NonNull<S>) -> Self {
        Self::new(state, Firmware::new())
    }

    #[cfg(test)]
    fn test(state: NonNull<S>) -> Self {
        Self::new(state, Firmware::test())
    }

    /// Run a closure with exclusive package-state access.
    #[must_use]
    pub fn with_state_mut<R>(
        &self,
        operation: impl for<'state> FnOnce(&'state mut S) -> R,
    ) -> Option<R> {
        let expected = crate::runtime::ExpectedState::Exact(self.state);
        #[cfg(not(target_arch = "arm"))]
        {
            S::runtime_store().with_expected_mut(expected, operation)
        }
        #[cfg(target_arch = "arm")]
        {
            crate::PackageStateStore::<S>::with_expected_mut(expected, operation)
        }
    }

    /// Return firmware capabilities for this package thread.
    #[must_use]
    pub fn firmware(&self) -> &Firmware {
        &self.firmware
    }
}

/// Rust implementation for a firmware package thread.
pub trait FirmwareThread {
    /// Package state type passed as the thread argument.
    type State: PackageRuntimeState;

    /// Run the thread body.
    fn run(ctx: ThreadContext<Self::State>);
}

/// Rust implementation for a firmware package thread that does not need package state.
pub trait StatelessFirmwareThread {
    /// Run the thread body.
    fn run(ctx: StatelessThreadContext);
}

const RETURNED_THREAD_POLL_INTERVAL: Duration = Duration::from_millis(1);
// `US2ST` in VESC's ChibiOS uses 32-bit arithmetic at 10 kHz.
const VESC_MAX_SAFE_SLEEP_MICROS: u32 = (u32::MAX - 999_999) / 10_000;

fn wait_for_firmware_termination(threads: &impl FirmwareThreads) {
    while !threads.should_terminate() {
        threads.sleep_for(RETURNED_THREAD_POLL_INTERVAL);
    }
}

fn run_firmware_thread_until_terminated<T: FirmwareThread>(
    context: ThreadContext<T::State>,
    threads: &impl FirmwareThreads,
) {
    T::run(context);
    wait_for_firmware_termination(threads);
}

fn run_stateless_firmware_thread_until_terminated<T: StatelessFirmwareThread>(
    context: StatelessThreadContext,
    threads: &impl FirmwareThreads,
) {
    T::run(context);
    wait_for_firmware_termination(threads);
}

/// Firmware ABI trampoline for a typed package thread.
///
/// # Safety
///
/// `arg` must point to the live package state installed in `T::State::runtime_store()`.
pub(crate) unsafe extern "C" fn firmware_thread_entry<T: FirmwareThread>(arg: *mut c_void) {
    let Some(state) = NonNull::new(arg.cast::<T::State>()) else {
        return;
    };
    #[cfg(test)]
    T::run(ThreadContext::test(state));
    #[cfg(not(test))]
    run_firmware_thread_until_terminated::<T>(
        ThreadContext::from_entry(state),
        &ThreadApi::new(RealThreadBindings),
    );
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
    run_stateless_firmware_thread_until_terminated::<T>(
        StatelessThreadContext::new(),
        &ThreadApi::new(RealThreadBindings),
    );
}

/// Firmware-owned native package thread handle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ThreadHandle(NonNull<c_void>);

// SAFETY: this is an opaque firmware thread identity. Moving ownership of the
// handle does not move or access the firmware thread itself.
unsafe impl Send for ThreadHandle {}

impl ThreadHandle {
    pub(crate) unsafe fn from_firmware(thread: *mut c_void) -> Option<Self> {
        NonNull::new(thread).map(Self)
    }

    /// Return the raw firmware thread handle.
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
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

    /// Supply a type-correct value for the unreachable invalid macro branch.
    #[doc(hidden)]
    #[must_use]
    pub const fn __invalid() -> Self {
        Self(b"invalid-thread\0")
    }

    /// Return the name without its private ABI terminator.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        let bytes = self.0.strip_suffix(&[0]).unwrap_or(self.0);
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
#[non_exhaustive]
pub enum ThreadError {
    /// The requested thread priority is not supported by this firmware.
    PriorityUnsupported,
}

impl core::fmt::Display for ThreadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("firmware does not support the requested thread priority")
    }
}

impl core::error::Error for ThreadError {}

/// Create a checked static firmware thread name from a Rust string literal.
#[macro_export]
macro_rules! thread_name {
    ($name:literal) => {
        const {
            const NAME: Option<$crate::ThreadName> =
                $crate::ThreadName::__from_terminated(concat!($name, "\0"));
            // A mismatched array length makes an embedded NUL a compile error
            // without placing a panic path in the package binary.
            const _: [(); 1] = [(); NAME.is_some() as usize];
            match NAME {
                Some(name) => name,
                None => $crate::ThreadName::__invalid(),
            }
        }
    };
}

/// `ChibiOS` working-area size passed to a firmware thread in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadWorkingAreaSize(usize);

/// Invalid `ChibiOS` thread working-area size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThreadWorkingAreaSizeError {
    /// The working area cannot hold `ChibiOS`'s required thread metadata and stack.
    TooSmall,
    /// The working-area byte count does not satisfy `ChibiOS` alignment.
    Misaligned,
}

impl core::fmt::Display for ThreadWorkingAreaSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::TooSmall => "thread working area must be at least 416 bytes",
            Self::Misaligned => "thread working area must be a multiple of 8 bytes",
        })
    }
}

impl core::error::Error for ThreadWorkingAreaSizeError {}

impl ThreadWorkingAreaSize {
    const MIN_BYTES: usize = 416;
    const WORKING_AREA_ALIGNMENT_BYTES: usize = 8;

    /// Build a working-area size from its firmware byte count.
    ///
    /// # Errors
    ///
    /// Returns an error when `bytes` is too small or incorrectly aligned.
    pub const fn try_from_bytes(bytes: usize) -> Result<Self, ThreadWorkingAreaSizeError> {
        if bytes < Self::MIN_BYTES {
            return Err(ThreadWorkingAreaSizeError::TooSmall);
        }
        if !bytes.is_multiple_of(Self::WORKING_AREA_ALIGNMENT_BYTES) {
            return Err(ThreadWorkingAreaSizeError::Misaligned);
        }
        Ok(Self(bytes))
    }

    pub(crate) const fn bytes(self) -> usize {
        self.0
    }
}

/// Typed firmware thread spawn settings.
#[derive(Debug)]
pub struct ThreadSpec<S: 'static> {
    entry: ThreadEntry,
    stack_size: ThreadWorkingAreaSize,
    name: ThreadName,
    argument: ThreadArgument,
    _state: PhantomData<fn(&mut S)>,
}

#[derive(Debug, Clone, Copy)]
enum ThreadArgument {
    PackageState,
    None,
}

impl<S: 'static> Copy for ThreadSpec<S> {}

impl<S: 'static> Clone for ThreadSpec<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static> ThreadSpec<S> {
    /// Build spawn settings for a typed firmware thread.
    pub fn new<T>(stack_size: ThreadWorkingAreaSize, name: ThreadName) -> Self
    where
        T: FirmwareThread<State = S>,
    {
        Self::from_entry(
            firmware_thread_entry::<T>,
            stack_size,
            name,
            ThreadArgument::PackageState,
        )
    }

    /// Build spawn settings for a typed firmware thread that ignores package state.
    pub fn stateless<T>(stack_size: ThreadWorkingAreaSize, name: ThreadName) -> Self
    where
        T: StatelessFirmwareThread,
    {
        Self::from_entry(
            stateless_firmware_thread_entry::<T>,
            stack_size,
            name,
            ThreadArgument::None,
        )
    }

    /// Build spawn settings from a raw firmware thread entrypoint.
    const fn from_entry(
        entry: ThreadEntry,
        stack_size: ThreadWorkingAreaSize,
        name: ThreadName,
        argument: ThreadArgument,
    ) -> Self {
        Self {
            entry,
            stack_size,
            name,
            argument,
            _state: PhantomData,
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_stateful_entry(
        entry: ThreadEntry,
        stack_size: ThreadWorkingAreaSize,
        name: ThreadName,
    ) -> Self {
        Self::from_entry(entry, stack_size, name, ThreadArgument::PackageState)
    }

    #[cfg(test)]
    pub(crate) const fn from_stateless_entry(
        entry: ThreadEntry,
        stack_size: ThreadWorkingAreaSize,
        name: ThreadName,
    ) -> Self {
        Self::from_entry(entry, stack_size, name, ThreadArgument::None)
    }
}

const MAX_PACKAGE_THREADS: usize = 20;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ThreadGroup {
    handles: [Option<ThreadHandle>; MAX_PACKAGE_THREADS],
}

impl ThreadGroup {
    fn new() -> Self {
        Self {
            handles: [const { None }; MAX_PACKAGE_THREADS],
        }
    }

    #[cfg(test)]
    pub(crate) fn from_handles<const N: usize>(handles: [ThreadHandle; N]) -> Self {
        assert!(N <= MAX_PACKAGE_THREADS);
        let mut group = Self::new();
        for (slot, handle) in group.handles.iter_mut().zip(handles) {
            *slot = Some(handle);
        }
        group
    }

    pub(crate) fn terminate_reverse<B: ThreadBindings>(self, threads: &ThreadApi<B>) {
        self.handles
            .into_iter()
            .rev()
            .flatten()
            .for_each(|thread| threads.request_terminate(thread));
    }
}

/// Typed firmware thread operations available to package code.
pub trait FirmwareThreads: private::FirmwareThreads {
    /// Return whether the current package thread should terminate.
    fn should_terminate(&self) -> bool;

    /// Sleep the current package thread for a duration.
    fn sleep_for(&self, duration: Duration);

    /// Set the current package thread priority when supported by firmware.
    ///
    /// # Errors
    ///
    /// Returns [`ThreadError`] when firmware does not support or rejects the operation.
    fn set_priority(&self, priority: ThreadPriority) -> Result<(), ThreadError>;
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

    /// Spawn firmware threads in order, terminating the started subset if one cannot start.
    ///
    /// C map: Float Out Boy passes its position-independent thread and string addresses
    /// directly to spawn at third_party/float-out-boy/src/main.c:2438-2444.
    /// VESC's `lib_request_terminate` does not return until the thread has
    /// terminated (`lispBM/lispif_c_lib.c:126-145`).
    ///
    /// The 20-slot limit is shared by all native libraries in firmware. This
    /// local bound only prevents one start operation from exceeding that hard
    /// ceiling; firmware remains authoritative when other libraries use slots.
    pub(crate) fn spawn_threads<S, const N: usize>(
        &self,
        specs: [ThreadSpec<S>; N],
        state: NonNull<S>,
    ) -> Option<ThreadGroup> {
        if N > MAX_PACKAGE_THREADS {
            return None;
        }
        let mut threads = ThreadGroup::new();
        for (slot, spec) in threads.handles.iter_mut().zip(specs) {
            let arg = match spec.argument {
                ThreadArgument::PackageState => state.as_ptr().cast::<c_void>(),
                ThreadArgument::None => core::ptr::null_mut(),
            };
            // C map: lispif_spawn consumes the runtime entry, name, and
            // argument addresses unchanged at
            // third_party/vesc/lispBM/lispif_c_lib.c:98-125.
            let thread = unsafe {
                self.bindings.spawn(
                    spec.entry,
                    spec.stack_size.bytes(),
                    spec.name.as_cstr().as_ptr(),
                    arg,
                )
            };
            let Some(handle) = (unsafe { ThreadHandle::from_firmware(thread) }) else {
                threads.terminate_reverse(self);
                return None;
            };
            *slot = Some(handle);
        }
        Some(threads)
    }

    /// Ask a firmware thread to terminate.
    fn request_terminate(&self, thread: ThreadHandle) {
        self.bindings.request_terminate(thread);
    }

    /// Return whether the current package thread should terminate.
    pub fn should_terminate(&self) -> bool {
        self.bindings.should_terminate()
    }

    /// Sleep the current package thread for a duration.
    ///
    /// Float Out Boy's main runtime thread sleeps with `VESC_IF->sleep_us` at
    /// `src/main.c:1080`.
    pub fn sleep_for(&self, duration: Duration) {
        let mut micros = duration.as_nanos().div_ceil(1_000);
        while micros != 0 {
            // `Duration` counts nanoseconds with `u128`, while the C firmware
            // accepts only a `uint32_t` microsecond chunk, and its ChibiOS
            // conversion has a still-smaller safe limit. Clamp to that limit
            // before the checked Rust conversion; the fallback is defensive
            // and cannot crash even if those limits change independently.
            let chunk = u32::try_from(micros.min(u128::from(VESC_MAX_SAFE_SLEEP_MICROS)))
                .unwrap_or(VESC_MAX_SAFE_SLEEP_MICROS);
            self.bindings.sleep_us(chunk);
            micros = micros.saturating_sub(u128::from(chunk));
        }
    }

    /// Set the current package thread priority when supported by firmware.
    ///
    /// Float Out Boy lowers `aux_thd` priority with optional
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
        pub(crate) spawn_stacks: Cell<[usize; 3]>,
        /// Spawn names by call order.
        pub(crate) spawn_names: Cell<[*const c_char; 3]>,
        /// Spawn args by call order.
        pub(crate) spawn_args: Cell<[usize; 3]>,
        /// Spawn entries by call order.
        pub(crate) spawn_entries: Cell<[usize; 3]>,
        /// Terminated thread handles by call order.
        pub(crate) terminated_threads: Cell<[usize; 3]>,
        /// Sleep durations by call order.
        pub(crate) sleep_micros: Cell<[u32; 2]>,
        /// Thread priorities by call order.
        pub(crate) priorities: Cell<[i8; 2]>,
        spawn_results: Cell<[usize; 3]>,
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
        /// Creates fake thread bindings returning three non-null handles.
        pub fn new() -> Self {
            Self::with_spawn_results([1, 2, 3])
        }

        /// Creates fake thread bindings returning explicit raw spawn handles.
        pub fn with_spawn_results(spawn_results: [usize; 3]) -> Self {
            Self {
                spawn_calls: Cell::new(0),
                terminate_calls: Cell::new(0),
                should_terminate_calls: Cell::new(0),
                sleep_calls: Cell::new(0),
                priority_calls: Cell::new(0),
                spawn_stacks: Cell::new([0, 0, 0]),
                spawn_names: Cell::new([core::ptr::null(), core::ptr::null(), core::ptr::null()]),
                spawn_args: Cell::new([0, 0, 0]),
                spawn_entries: Cell::new([0, 0, 0]),
                terminated_threads: Cell::new([0, 0, 0]),
                sleep_micros: Cell::new([0, 0]),
                priorities: Cell::new([0, 0]),
                spawn_results: Cell::new(spawn_results),
                should_terminate_result: Cell::new(false),
                should_terminate_after_calls: Cell::new(usize::MAX),
                priority_result: Cell::new(true),
            }
        }

        pub(crate) fn with_termination_after_checks(self, checks: usize) -> Self {
            self.should_terminate_after_calls.set(checks);
            self
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
            let index = call.min(2);

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
            let index = call.min(2);
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
        ThreadContext, ThreadSpec, ThreadWorkingAreaSize, ThreadWorkingAreaSizeError,
        VESC_MAX_SAFE_SLEEP_MICROS, firmware_thread_entry, stateless_firmware_thread_entry,
    };
    use core::ffi::CStr;
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use core::time::Duration;
    use std::boxed::Box;
    use std::sync::Mutex;

    use crate::thread::test_support::FakeThreadBindings;
    use crate::units::TimestampTicks;

    #[test]
    fn semantic_app_data_capability_hides_binding_shape() {
        let bindings = crate::test_support::FakeAppDataBindings::new();
        let app_data = AppDataApi::new(&bindings);

        assert_eq!(app_data.system_timestamp(), TimestampTicks::from_ticks(0));
        assert_eq!(app_data.send(&[1, 2, 3]), Ok(()));
        assert_eq!(bindings.send_calls.get(), 1);
        assert_eq!(bindings.last_len.get(), 3);
    }

    #[test]
    fn app_data_payload_stops_at_the_firmware_packet_limit() {
        let bindings = crate::test_support::FakeAppDataBindings::new();
        let app_data = AppDataApi::new(&bindings);

        assert_eq!(app_data.send(&[0; 511]), Ok(()));
        assert_eq!(
            app_data.send(&[0; 512]),
            Err(crate::AppDataSendError::PayloadTooLarge)
        );
        assert_eq!(bindings.send_calls.get(), 1);
        assert_eq!(bindings.last_len.get(), 511);
    }

    #[test]
    fn semantic_thread_capability_hides_binding_shape() {
        fn accepts_semantic_threads(threads: &impl super::FirmwareThreads) -> bool {
            threads.should_terminate()
        }

        let bindings = FakeThreadBindings::new();
        let threads = ThreadApi::new(&bindings);
        assert!(!accepts_semantic_threads(&threads));
    }

    #[test]
    fn sleep_for_stays_within_vesc_chibios_conversion_range() {
        let zero = FakeThreadBindings::new();
        ThreadApi::new(&zero).sleep_for(Duration::ZERO);
        assert_eq!(zero.sleep_calls.get(), 0);

        let sub_microsecond = FakeThreadBindings::new();
        ThreadApi::new(&sub_microsecond).sleep_for(Duration::from_nanos(1));
        assert_eq!(sub_microsecond.sleep_micros.get(), [1, 0]);

        let overflow = FakeThreadBindings::new();
        ThreadApi::new(&overflow).sleep_for(Duration::from_micros(
            u64::from(VESC_MAX_SAFE_SLEEP_MICROS) + 1,
        ));
        assert_eq!(overflow.sleep_micros.get(), [VESC_MAX_SAFE_SLEEP_MICROS, 1]);
    }

    static RUN_CALLS: AtomicUsize = AtomicUsize::new(0);
    static OBSERVED_STATE: AtomicU32 = AtomicU32::new(0);
    static RETURNED_STATEFUL_RUNS: AtomicUsize = AtomicUsize::new(0);
    static RETURNED_STATELESS_RUNS: AtomicUsize = AtomicUsize::new(0);
    static THREAD_ENTRY_TEST_LOCK: Mutex<()> = Mutex::new(());
    static THREAD_STATE: crate::PackageStateStore<ThreadState> = crate::PackageStateStore::new();

    struct ThreadState(u32);

    impl crate::PackageRuntimeState for ThreadState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &THREAD_STATE
        }
    }

    struct RecordingThread;
    struct RecordingStatelessThread;
    struct ReturningThread;
    struct ReturningStatelessThread;

    impl FirmwareThread for RecordingThread {
        type State = ThreadState;

        fn run(ctx: ThreadContext<Self::State>) {
            RUN_CALLS.fetch_add(1, Ordering::SeqCst);
            let _ = ctx.with_state_mut(|state| {
                state.0 += 1;
                OBSERVED_STATE.store(state.0, Ordering::SeqCst);
            });
        }
    }

    impl StatelessFirmwareThread for RecordingStatelessThread {
        fn run(_ctx: StatelessThreadContext) {
            RUN_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl FirmwareThread for ReturningThread {
        type State = ThreadState;

        fn run(_ctx: ThreadContext<Self::State>) {
            RETURNED_STATEFUL_RUNS.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl StatelessFirmwareThread for ReturningStatelessThread {
        fn run(_ctx: StatelessThreadContext) {
            RETURNED_STATELESS_RUNS.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn returned_stateful_thread_waits_for_firmware_termination() {
        RETURNED_STATEFUL_RUNS.store(0, Ordering::SeqCst);
        let bindings = FakeThreadBindings::new().with_termination_after_checks(3);
        let threads = ThreadApi::new(&bindings);
        let state = Box::leak(Box::new(ThreadState(0)));

        super::run_firmware_thread_until_terminated::<ReturningThread>(
            ThreadContext::test(core::ptr::NonNull::from(state)),
            &threads,
        );

        assert_eq!(RETURNED_STATEFUL_RUNS.load(Ordering::SeqCst), 1);
        assert_eq!(bindings.should_terminate_calls.get(), 3);
        assert_eq!(bindings.sleep_calls.get(), 2);
        assert_eq!(bindings.sleep_micros.get(), [1_000, 1_000]);
    }

    #[test]
    fn returned_stateless_thread_waits_for_firmware_termination() {
        RETURNED_STATELESS_RUNS.store(0, Ordering::SeqCst);
        let bindings = FakeThreadBindings::new().with_termination_after_checks(3);
        let threads = ThreadApi::new(&bindings);

        super::run_stateless_firmware_thread_until_terminated::<ReturningStatelessThread>(
            StatelessThreadContext::test(),
            &threads,
        );

        assert_eq!(RETURNED_STATELESS_RUNS.load(Ordering::SeqCst), 1);
        assert_eq!(bindings.should_terminate_calls.get(), 3);
        assert_eq!(bindings.sleep_calls.get(), 2);
        assert_eq!(bindings.sleep_micros.get(), [1_000, 1_000]);
    }

    #[test]
    fn thread_working_area_size_accepts_chibios_minimum_and_float_out_boy_sizes() {
        assert_eq!(
            ThreadWorkingAreaSize::try_from_bytes(416).unwrap().bytes(),
            416
        );
        assert_eq!(
            ThreadWorkingAreaSize::try_from_bytes(1_024)
                .unwrap()
                .bytes(),
            1_024
        );
        assert_eq!(
            ThreadWorkingAreaSize::try_from_bytes(1_536)
                .unwrap()
                .bytes(),
            1_536
        );
    }

    #[test]
    fn thread_working_area_size_rejects_undersized_values() {
        assert_eq!(
            ThreadWorkingAreaSize::try_from_bytes(408),
            Err(ThreadWorkingAreaSizeError::TooSmall)
        );
    }

    #[test]
    fn thread_working_area_size_rejects_misaligned_values() {
        assert_eq!(
            ThreadWorkingAreaSize::try_from_bytes(420),
            Err(ThreadWorkingAreaSizeError::Misaligned)
        );
    }

    #[test]
    fn firmware_thread_entry_returns_without_state() {
        let _guard = THREAD_ENTRY_TEST_LOCK.lock().unwrap();
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);

        unsafe { firmware_thread_entry::<RecordingThread>(core::ptr::null_mut()) };

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn firmware_thread_entry_passes_typed_state_through_context() {
        let _guard = THREAD_ENTRY_TEST_LOCK.lock().unwrap();
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);
        let state = Box::leak(Box::new(ThreadState(41)));
        assert!(unsafe { THREAD_STATE.install(state) });

        unsafe {
            firmware_thread_entry::<RecordingThread>(core::ptr::from_mut(state).cast::<c_void>());
        }

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
        THREAD_STATE.clear();
        assert_eq!(state.0, 42);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 42);
    }

    #[test]
    fn stateless_firmware_thread_entry_ignores_raw_arg() {
        let _guard = THREAD_ENTRY_TEST_LOCK.lock().unwrap();
        RUN_CALLS.store(0, Ordering::SeqCst);
        OBSERVED_STATE.store(0, Ordering::SeqCst);

        unsafe {
            stateless_firmware_thread_entry::<RecordingStatelessThread>(core::ptr::null_mut());
        };

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(OBSERVED_STATE.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn stateless_firmware_thread_entry_ignores_nonnull_raw_arg() {
        let _guard = THREAD_ENTRY_TEST_LOCK.lock().unwrap();
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
        let _guard = THREAD_ENTRY_TEST_LOCK.lock().unwrap();
        RUN_CALLS.store(0, Ordering::SeqCst);

        RecordingStatelessThread::run(StatelessThreadContext::test());

        assert_eq!(RUN_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn thread_specs_pass_state_only_to_stateful_threads() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0x2000, 0x3000]);
        let threads = ThreadApi::new(&bindings);
        let first_name = crate::thread_name!("first");
        let second_name = crate::thread_name!("second");
        let third_name = crate::thread_name!("third");
        let specs = [
            ThreadSpec::from_stateful_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(1_536).unwrap(),
                first_name,
            ),
            ThreadSpec::from_stateless_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(1_024).unwrap(),
                second_name,
            ),
            ThreadSpec::from_stateless_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(768).unwrap(),
                third_name,
            ),
        ];
        let entries = specs.map(|spec| spec.entry as usize);
        let mut state = 7_u32;
        let state_arg = core::ptr::from_mut(&mut state).cast::<c_void>() as usize;

        let handles = threads.spawn_threads(specs, core::ptr::NonNull::from(&mut state));

        assert_eq!(
            handles
                .as_ref()
                .and_then(|group| group.handles[0].as_ref())
                .map(super::ThreadHandle::as_ptr),
            Some(0x1000 as *mut c_void)
        );
        assert_eq!(
            handles
                .as_ref()
                .and_then(|group| group.handles[2].as_ref())
                .map(super::ThreadHandle::as_ptr),
            Some(0x3000 as *mut c_void)
        );
        assert_eq!(
            handles
                .as_ref()
                .and_then(|group| group.handles[1].as_ref())
                .map(super::ThreadHandle::as_ptr),
            Some(0x2000 as *mut c_void)
        );
        assert_eq!(bindings.spawn_calls.get(), 3);
        assert_eq!(bindings.spawn_entries.get(), entries);
        assert_eq!(bindings.spawn_stacks.get(), [1_536, 1_024, 768]);
        assert_eq!(
            bindings
                .spawn_names
                .get()
                .map(|name| unsafe { CStr::from_ptr(name) }),
            [
                first_name.as_cstr(),
                second_name.as_cstr(),
                third_name.as_cstr()
            ]
        );
        assert_eq!(bindings.spawn_args.get(), [state_arg, 0, 0]);
        assert_eq!(bindings.terminate_calls.get(), 0);
    }

    #[test]
    fn thread_specs_terminate_started_threads_when_a_later_spawn_fails() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0x2000, 0]);
        let threads = ThreadApi::new(&bindings);
        let specs = [
            ThreadSpec::from_stateful_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(1_536).unwrap(),
                crate::thread_name!("first"),
            ),
            ThreadSpec::from_stateless_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(1_024).unwrap(),
                crate::thread_name!("second"),
            ),
            ThreadSpec::from_stateless_entry(
                stateless_firmware_thread_entry::<RecordingStatelessThread>,
                ThreadWorkingAreaSize::try_from_bytes(768).unwrap(),
                crate::thread_name!("third"),
            ),
        ];
        let mut state = 7_u32;

        let handles = threads.spawn_threads(specs, core::ptr::NonNull::from(&mut state));

        assert_eq!(handles, None);
        assert_eq!(bindings.spawn_calls.get(), 3);
        assert_eq!(bindings.terminate_calls.get(), 2);
        assert_eq!(bindings.terminated_threads.get(), [0x2000, 0x1000, 0]);
    }

    #[test]
    fn thread_name_exposes_rust_text_without_abi_terminator() {
        let name = crate::thread_name!("Float Out Boy Main");

        assert_eq!(name.as_str(), "Float Out Boy Main");
        assert_eq!(
            super::ThreadName::__from_terminated("Float Out Boy Main\0")
                .map(super::ThreadName::as_str),
            Some("Float Out Boy Main")
        );
        assert!(super::ThreadName::__from_terminated("Float Out Boy Main").is_none());
        assert!(super::ThreadName::__from_terminated("Float Out Boy\0 Main\0").is_none());
    }
}
