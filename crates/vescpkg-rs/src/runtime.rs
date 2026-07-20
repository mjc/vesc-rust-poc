//! Package state shared by startup and firmware callbacks.

#[cfg(any(test, target_arch = "arm"))]
use core::any::TypeId;
use core::cell::UnsafeCell;
#[cfg(not(target_arch = "arm"))]
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::ptr::NonNull;
#[cfg(not(target_arch = "arm"))]
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

#[cfg(not(target_arch = "arm"))]
const EMPTY: u8 = 0;
const INSTALLING: u8 = 1;
const RUNNING: u8 = 2;
const STOPPING: u8 = 3;
const STOPPED: u8 = 4;
#[cfg(target_arch = "arm")]
const APP_DATA_CALLBACK: u8 = 1;
#[cfg(target_arch = "arm")]
const CUSTOM_CONFIG_CALLBACKS: u8 = 1 << 1;
#[cfg(target_arch = "arm")]
const IMU_CALLBACK: u8 = 1 << 2;

#[derive(Clone, Copy, Default)]
pub(crate) struct CallbackRegistrations {
    app_data: bool,
    custom_config: bool,
    imu: bool,
}

impl CallbackRegistrations {
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn clear_registered<B>(self, bindings: &B)
    where
        B: crate::bindings::AppDataBindings
            + crate::bindings::CustomConfigBindings
            + crate::bindings::ImuReadCallbackBindings,
    {
        if self.imu {
            unsafe { bindings.clear_imu_read_callback() };
        }
        if self.app_data {
            let _ = unsafe { bindings.clear_app_data_handler() };
        }
        if self.custom_config {
            let _ = unsafe { bindings.clear_custom_configs() };
        }
    }
}

#[cfg(not(target_arch = "arm"))]
#[derive(Clone, Copy)]
pub(crate) struct CallbackRecorder {
    state: NonNull<core::ffi::c_void>,
    finish_start: unsafe fn(NonNull<core::ffi::c_void>, bool) -> bool,
    app_data: unsafe fn(NonNull<core::ffi::c_void>) -> bool,
    custom_config: unsafe fn(NonNull<core::ffi::c_void>) -> bool,
    clear_custom_config: unsafe fn(NonNull<core::ffi::c_void>) -> bool,
    imu: unsafe fn(NonNull<core::ffi::c_void>) -> bool,
}

#[cfg(not(target_arch = "arm"))]
impl CallbackRecorder {
    pub(crate) fn new<T: crate::PackageRuntimeState>(state: NonNull<T>) -> Self {
        unsafe fn app_data<T: crate::PackageRuntimeState>(
            state: NonNull<core::ffi::c_void>,
        ) -> bool {
            T::runtime_store().record_app_data_callback(state.cast())
        }
        unsafe fn finish_start<T: crate::PackageRuntimeState>(
            state: NonNull<core::ffi::c_void>,
            started: bool,
        ) -> bool {
            T::runtime_store().finish_start(state.cast(), started)
        }
        unsafe fn custom_config<T: crate::PackageRuntimeState>(
            state: NonNull<core::ffi::c_void>,
        ) -> bool {
            T::runtime_store().record_custom_config_callbacks(state.cast())
        }
        unsafe fn clear_custom_config<T: crate::PackageRuntimeState>(
            state: NonNull<core::ffi::c_void>,
        ) -> bool {
            T::runtime_store().clear_custom_config_registration(state.cast())
        }
        unsafe fn imu<T: crate::PackageRuntimeState>(state: NonNull<core::ffi::c_void>) -> bool {
            T::runtime_store().record_imu_callback(state.cast())
        }

        Self {
            state: state.cast(),
            finish_start: finish_start::<T>,
            app_data: app_data::<T>,
            custom_config: custom_config::<T>,
            clear_custom_config: clear_custom_config::<T>,
            imu: imu::<T>,
        }
    }

    pub(crate) fn finish_start(self, started: bool) -> bool {
        unsafe { (self.finish_start)(self.state, started) }
    }

    pub(crate) fn record_app_data(self) -> bool {
        unsafe { (self.app_data)(self.state) }
    }

    pub(crate) fn record_custom_config(self) -> bool {
        unsafe { (self.custom_config)(self.state) }
    }

    pub(crate) fn clear_custom_config(self) -> bool {
        unsafe { (self.clear_custom_config)(self.state) }
    }

    pub(crate) fn record_imu(self) -> bool {
        unsafe { (self.imu)(self.state) }
    }
}

#[cfg(target_arch = "arm")]
#[derive(Clone, Copy)]
pub(crate) struct CallbackRecorder(NonNull<core::ffi::c_void>);

#[cfg(target_arch = "arm")]
impl CallbackRecorder {
    pub(crate) fn new<T: crate::PackageRuntimeState>(state: NonNull<T>) -> Self {
        Self(state.cast())
    }

    fn update(self, flag: u8, registered: bool) -> bool {
        unsafe {
            update_firmware_callbacks(self.0, |callbacks| match flag {
                APP_DATA_CALLBACK => callbacks.app_data = registered,
                CUSTOM_CONFIG_CALLBACKS => callbacks.custom_config = registered,
                IMU_CALLBACK => callbacks.imu = registered,
                _ => {}
            })
        }
    }

    pub(crate) fn finish_start(self, started: bool) -> bool {
        let Some(runtime) = (unsafe { firmware_runtime_from_untyped_state(self.0) }) else {
            return false;
        };
        finish_firmware_start(runtime, started)
    }

    pub(crate) fn record_app_data(self) -> bool {
        self.update(APP_DATA_CALLBACK, true)
    }

    pub(crate) fn record_custom_config(self) -> bool {
        self.update(CUSTOM_CONFIG_CALLBACKS, true)
    }

    pub(crate) fn clear_custom_config(self) -> bool {
        self.update(CUSTOM_CONFIG_CALLBACKS, false)
    }

    pub(crate) fn record_imu(self) -> bool {
        self.update(IMU_CALLBACK, true)
    }
}

/// Package-state identity shared by firmware startup and callbacks.
///
/// On firmware this is a zero-sized marker: mutable lifecycle state lives in
/// firmware heap RAM beside `T`, never in the flash-backed package image.
/// Callers use [`Self::with`] or [`Self::with_mut`] so temporary state borrows
/// remain scoped to a callback.
pub struct PackageStateStore<T> {
    #[cfg(not(target_arch = "arm"))]
    state: AtomicPtr<T>,
    #[cfg(not(target_arch = "arm"))]
    phase: AtomicU8,
    #[cfg(not(target_arch = "arm"))]
    active: AtomicUsize,
    #[cfg(not(target_arch = "arm"))]
    host_lock: AtomicBool,
    #[cfg(not(target_arch = "arm"))]
    threads: UnsafeCell<Option<crate::thread::ThreadGroup>>,
    #[cfg(not(target_arch = "arm"))]
    callbacks: UnsafeCell<CallbackRegistrations>,
    _state: PhantomData<UnsafeCell<T>>,
}

#[cfg(target_arch = "arm")]
const _: [(); 0] = [(); core::mem::size_of::<PackageStateStore<()>>()];

#[cfg(any(test, target_arch = "arm"))]
const RUNTIME_MAGIC: u32 = 0x5652_5354;

#[cfg(any(test, target_arch = "arm"))]
#[repr(C)]
struct FirmwareRuntime {
    magic: u32,
    state_type: TypeId,
    state: *mut core::ffi::c_void,
    state_lock: AtomicBool,
    phase: AtomicU8,
    active: AtomicUsize,
    threads: UnsafeCell<Option<crate::thread::ThreadGroup>>,
    callbacks: UnsafeCell<CallbackRegistrations>,
}

#[cfg(any(test, target_arch = "arm"))]
// SAFETY: every mutable field is accessed while `state_lock` is held, except the
// atomic lifecycle counters used to admit and drain those locked accesses.
unsafe impl Sync for FirmwareRuntime {}

#[cfg(any(test, target_arch = "arm"))]
const fn firmware_state_alignment<T>() -> usize {
    let pointer = core::mem::align_of::<*mut FirmwareRuntime>();
    let state = core::mem::align_of::<T>();
    if state > pointer { state } else { pointer }
}

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn firmware_runtime_allocation_size<T>() -> Option<usize> {
    core::mem::size_of::<FirmwareRuntime>()
        .checked_add(core::mem::size_of::<*mut FirmwareRuntime>())?
        .checked_add(firmware_state_alignment::<T>() - 1)?
        .checked_add(if core::mem::size_of::<T>() == 0 {
            1
        } else {
            core::mem::size_of::<T>()
        })
}

#[cfg(any(test, target_arch = "arm"))]
/// Return the `T` address within a runtime allocation and install its backlink.
///
/// # Safety
///
/// `allocation` must point to writable firmware heap memory of at least
/// `firmware_runtime_allocation_size::<T>()` bytes.
pub(crate) unsafe fn firmware_runtime_state_pointer<T>(
    allocation: NonNull<core::ffi::c_void>,
) -> NonNull<T> {
    let after_runtime = allocation.as_ptr().cast::<u8>().wrapping_add(
        core::mem::size_of::<FirmwareRuntime>() + core::mem::size_of::<*mut FirmwareRuntime>(),
    );
    let align = firmware_state_alignment::<T>();
    let state = after_runtime
        .map_addr(|address| (address + align - 1) & !(align - 1))
        .cast::<T>();
    let backlink = state.cast::<*mut FirmwareRuntime>().wrapping_sub(1);
    // SAFETY: the allocation-size calculation reserves this pointer-sized slot
    // immediately before the aligned state address.
    unsafe { backlink.write(allocation.as_ptr().cast()) };
    // SAFETY: adding an in-bounds offset to a non-null allocation cannot yield null.
    unsafe { NonNull::new_unchecked(state) }
}

#[cfg(any(test, target_arch = "arm"))]
/// Resolve the live runtime header installed immediately before `state`.
///
/// # Safety
///
/// `state` must be the `T` pointer produced by [`firmware_runtime_state_pointer`]
/// and its allocation must remain live for the returned reference.
unsafe fn firmware_runtime_from_state<T: 'static>(
    state: NonNull<T>,
) -> Option<&'static FirmwareRuntime> {
    let runtime = unsafe { firmware_runtime_from_untyped_state(state.cast()) }?;
    (runtime.state_type == TypeId::of::<T>()).then_some(runtime)
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn firmware_runtime_from_untyped_state(
    state: NonNull<core::ffi::c_void>,
) -> Option<&'static FirmwareRuntime> {
    let backlink = state
        .cast::<*mut FirmwareRuntime>()
        .as_ptr()
        .wrapping_sub(1);
    // SAFETY: the caller guarantees `state` was produced by the layout helper,
    // which initialized the in-bounds backlink slot.
    let runtime = unsafe { NonNull::new(backlink.read()) }?;
    // SAFETY: the caller also guarantees the runtime allocation remains live.
    let runtime = unsafe { runtime.as_ref() };
    (runtime.magic == RUNTIME_MAGIC && runtime.state == state.as_ptr()).then_some(runtime)
}

#[cfg(any(test, target_arch = "arm"))]
struct FirmwareRuntimeBorrow<'a>(&'a FirmwareRuntime);

#[cfg(any(test, target_arch = "arm"))]
impl Drop for FirmwareRuntimeBorrow<'_> {
    fn drop(&mut self) {
        self.0.state_lock.store(false, Ordering::Release);
    }
}

#[cfg(any(test, target_arch = "arm"))]
fn borrow_firmware_runtime(runtime: &FirmwareRuntime) -> FirmwareRuntimeBorrow<'_> {
    while runtime
        .state_lock
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        #[cfg(target_arch = "arm")]
        unsafe {
            // SAFETY: firmware callbacks and package threads run in ChibiOS thread context.
            crate::ffi::vesc_sleep_us(1);
        }
        #[cfg(not(target_arch = "arm"))]
        spin_loop();
    }
    FirmwareRuntimeBorrow(runtime)
}

#[cfg(target_arch = "arm")]
unsafe fn update_firmware_callbacks(
    state: NonNull<core::ffi::c_void>,
    update: impl FnOnce(&mut CallbackRegistrations),
) -> bool {
    let Some(runtime) = (unsafe { firmware_runtime_from_untyped_state(state) }) else {
        return false;
    };
    let _borrow = borrow_firmware_runtime(runtime);
    let running = matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING);
    if running {
        update(unsafe { &mut *runtime.callbacks.get() });
    }
    running
}

// SAFETY: all access to `T` is serialized by the runtime state lock on device and
// by the host-only test lock otherwise. Moving access between firmware threads
// therefore requires `T: Send`, not `T: Sync`.
unsafe impl<T: Send> Sync for PackageStateStore<T> {}

/// Loader-owned package state with package-specific stop behavior.
pub trait PackageRuntimeState: Sized + Send + 'static {
    /// Return the callback-visible slot for this state.
    fn runtime_store() -> &'static PackageStateStore<Self>;

    /// Stop package-owned resources before the state is freed.
    fn stop(&mut self) {}
}

#[cfg(not(target_arch = "arm"))]
struct PackageStateBorrow<'a, T> {
    store: &'a PackageStateStore<T>,
    _state: PhantomData<&'a T>,
}

#[cfg(not(target_arch = "arm"))]
impl<T> Drop for PackageStateBorrow<'_, T> {
    fn drop(&mut self) {
        self.store.host_lock.store(false, Ordering::Release);
    }
}

struct PackageStateEntry<'a, T> {
    #[cfg(not(target_arch = "arm"))]
    store: &'a PackageStateStore<T>,
    #[cfg(target_arch = "arm")]
    runtime: &'a FirmwareRuntime,
    _state: PhantomData<&'a T>,
}

impl<T> Drop for PackageStateEntry<'_, T> {
    fn drop(&mut self) {
        #[cfg(target_arch = "arm")]
        self.runtime.active.fetch_sub(1, Ordering::AcqRel);
        #[cfg(not(target_arch = "arm"))]
        self.store.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(target_arch = "arm", allow(dead_code))]
pub(crate) enum PackageStateInstallError {
    AlreadyInstalled,
}

#[cfg(any(test, target_arch = "arm"))]
fn finish_firmware_runtime(runtime: &FirmwareRuntime) {
    runtime.phase.store(STOPPED, Ordering::Release);
}

#[cfg(any(test, target_arch = "arm"))]
fn enter_firmware(runtime: &FirmwareRuntime) -> bool {
    if !matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING) {
        return false;
    }
    runtime.active.fetch_add(1, Ordering::AcqRel);
    if matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING) {
        true
    } else {
        runtime.active.fetch_sub(1, Ordering::AcqRel);
        false
    }
}

#[cfg(target_arch = "arm")]
fn finish_firmware_start(runtime: &FirmwareRuntime, started: bool) -> bool {
    let running = started
        && runtime
            .phase
            .compare_exchange(INSTALLING, RUNNING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
    runtime.active.fetch_sub(1, Ordering::AcqRel);
    running
}

enum StateIdentity<T> {
    #[cfg(not(target_arch = "arm"))]
    Installed,
    Firmware(unsafe fn() -> Option<NonNull<T>>),
}

pub(crate) enum ExpectedState<T> {
    #[cfg(not(target_arch = "arm"))]
    Any,
    Exact(NonNull<T>),
}

impl<T> Clone for ExpectedState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ExpectedState<T> {}

impl<T> Clone for StateIdentity<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StateIdentity<T> {}

/// Package state shared by package callbacks.
///
/// C map: `ARG(PROG_ADDR)` resolves the package's loader-owned state at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:697-700`; VESC implements that lookup
/// in `third_party/vesc/lispBM/lispif_c_lib.c:151-158`. The runtime slot is the
/// Rust-owned fast path, while `fallback` preserves the firmware lookup used by
/// callbacks that can run without that slot installed.
pub struct PackageStateAccess<'a, T: Send + 'static> {
    runtime: &'a PackageStateStore<T>,
    identity: StateIdentity<T>,
}

impl<T: Send + 'static> Clone for PackageStateAccess<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + 'static> Copy for PackageStateAccess<'_, T> {}

impl<'a, T: Send + 'static> PackageStateAccess<'a, T> {
    /// Build a source backed only by the installed runtime slot.
    #[must_use]
    #[cfg(not(target_arch = "arm"))]
    pub const fn runtime(runtime: &'a PackageStateStore<T>) -> Self {
        Self {
            runtime,
            identity: StateIdentity::Installed,
        }
    }

    /// Build access that prefers the store and falls back to firmware state.
    ///
    /// # Safety
    ///
    /// The fallback must return a non-null pointer to a live `T` whenever it returns
    /// `Some`. The pointed-to state must remain valid for the duration of each callback,
    /// and all callback access to that state must be coordinated through this store.
    pub(crate) const unsafe fn with_firmware_fallback(
        runtime: &'a PackageStateStore<T>,
        firmware_state: unsafe fn() -> Option<NonNull<T>>,
    ) -> Self {
        Self {
            runtime,
            identity: StateIdentity::Firmware(firmware_state),
        }
    }

    /// Run `f` with package state, preferring the runtime slot over loader state.
    #[inline(always)]
    #[must_use]
    pub fn with<R>(&self, f: impl for<'state> FnOnce(&'state T) -> R) -> Option<R> {
        self.runtime.with_expected(self.expected_state()?, f)
    }

    /// Run `f` with mutable package state, preferring the runtime slot over loader state.
    #[inline(always)]
    #[must_use]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.runtime.with_expected_mut(self.expected_state()?, f)
    }

    fn expected_state(&self) -> Option<ExpectedState<T>> {
        match self.identity {
            #[cfg(not(target_arch = "arm"))]
            StateIdentity::Installed => Some(ExpectedState::Any),
            StateIdentity::Firmware(state) => unsafe { state() }.map(ExpectedState::Exact),
        }
    }
}

#[cfg_attr(target_arch = "arm", allow(clippy::unused_self))]
impl<T: Send + 'static> PackageStateStore<T> {
    /// Create an empty package-state slot.
    pub const fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "arm"))]
            state: AtomicPtr::new(core::ptr::null_mut()),
            #[cfg(not(target_arch = "arm"))]
            phase: AtomicU8::new(EMPTY),
            #[cfg(not(target_arch = "arm"))]
            active: AtomicUsize::new(0),
            #[cfg(not(target_arch = "arm"))]
            host_lock: AtomicBool::new(false),
            #[cfg(not(target_arch = "arm"))]
            threads: UnsafeCell::new(None),
            #[cfg(not(target_arch = "arm"))]
            callbacks: UnsafeCell::new(CallbackRegistrations {
                app_data: false,
                custom_config: false,
                imu: false,
            }),
            _state: PhantomData,
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn borrow_exclusive(&self) -> PackageStateBorrow<'_, T> {
        while self
            .host_lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        PackageStateBorrow {
            store: self,
            _state: PhantomData,
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn owns(&self, state: NonNull<T>) -> bool {
        NonNull::new(self.state.load(Ordering::Acquire)) == Some(state)
    }

    #[cfg(target_arch = "arm")]
    fn borrow_exclusive<'runtime>(
        &self,
        runtime: &'runtime FirmwareRuntime,
    ) -> FirmwareRuntimeBorrow<'runtime> {
        borrow_firmware_runtime(runtime)
    }

    /// Install package state for later callback access.
    ///
    /// # Safety
    ///
    /// `state` must outlive every callback that can access this slot, and the
    /// caller must clear the slot before freeing it.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) unsafe fn install(&self, state: &mut T) -> Result<(), PackageStateInstallError> {
        let state_ptr = NonNull::from(&mut *state);
        unsafe { self.install_owned(state, state_ptr.cast()) }?;
        let _ = self.finish_start(state_ptr, true);
        Ok(())
    }

    #[cfg_attr(target_arch = "arm", allow(clippy::unnecessary_wraps))]
    pub(crate) unsafe fn install_owned(
        &self,
        state: &mut T,
        allocation: NonNull<core::ffi::c_void>,
    ) -> Result<(), PackageStateInstallError> {
        #[cfg(not(target_arch = "arm"))]
        {
            let phase = self.phase.load(Ordering::Acquire);
            if !matches!(phase, EMPTY | STOPPED)
                || self
                    .phase
                    .compare_exchange(phase, INSTALLING, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            {
                return Err(PackageStateInstallError::AlreadyInstalled);
            }
            self.state
                .store(core::ptr::from_mut(state), Ordering::Release);
            let _ = allocation;
            self.active.store(1, Ordering::Release);
            Ok(())
        }
        #[cfg(target_arch = "arm")]
        {
            let runtime = allocation.cast::<FirmwareRuntime>();
            // SAFETY: `allocation` reserves and aligns a `FirmwareRuntime` header
            // before `state`; startup has exclusive ownership of it here.
            unsafe {
                runtime.as_ptr().write(FirmwareRuntime {
                    magic: RUNTIME_MAGIC,
                    state_type: TypeId::of::<T>(),
                    state: core::ptr::from_mut(state).cast(),
                    state_lock: AtomicBool::new(false),
                    phase: AtomicU8::new(INSTALLING),
                    active: AtomicUsize::new(1),
                    threads: UnsafeCell::new(None),
                    callbacks: UnsafeCell::new(CallbackRegistrations::default()),
                });
            }
            Ok(())
        }
    }

    #[cfg_attr(target_arch = "arm", allow(dead_code))]
    pub(crate) fn finish_start(&self, state: NonNull<T>, started: bool) -> bool {
        #[cfg(not(target_arch = "arm"))]
        let Some(runtime_state) = NonNull::new(self.state.load(Ordering::Acquire)) else {
            return false;
        };
        #[cfg(not(target_arch = "arm"))]
        if runtime_state != state {
            return false;
        }
        #[cfg(not(target_arch = "arm"))]
        let (phase, active) = (&self.phase, &self.active);
        #[cfg(target_arch = "arm")]
        let Some(runtime) = (unsafe { firmware_runtime_from_state(state) }) else {
            return false;
        };
        #[cfg(target_arch = "arm")]
        let (phase, active) = (&runtime.phase, &runtime.active);
        let running = started
            && phase
                .compare_exchange(INSTALLING, RUNNING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok();
        active.fetch_sub(1, Ordering::AcqRel);
        running
    }

    /// Clear the installed state pointer.
    #[cfg(all(any(test, feature = "test-support"), not(target_arch = "arm")))]
    pub(crate) fn clear(&self) {
        let Some(state) = NonNull::new(self.state.load(Ordering::Acquire)) else {
            return;
        };
        if !self.begin_stop(state) {
            return;
        }
        let _ = self.take_threads(state);
        self.finish_stop(state);
    }

    pub(crate) fn begin_stop(&self, state: NonNull<T>) -> bool {
        #[cfg(not(target_arch = "arm"))]
        let phase = {
            if !self.owns(state) {
                return false;
            }
            &self.phase
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: lifecycle methods receive the exact live loader state pointer.
        let Some(phase) =
            (unsafe { firmware_runtime_from_state(state) }).map(|runtime| &runtime.phase)
        else {
            return false;
        };
        loop {
            let current = phase.load(Ordering::Acquire);
            if !matches!(current, INSTALLING | RUNNING) {
                return false;
            }
            if phase
                .compare_exchange(current, STOPPING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }

    pub(crate) fn install_threads(
        &self,
        state: NonNull<T>,
        threads: &mut Option<crate::thread::ThreadGroup>,
    ) -> bool {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let (phase, slot) = {
            if !self.owns(state) {
                return false;
            }
            (&self.phase, &self.threads)
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: thread installation receives the exact live loader state pointer.
        let Some(runtime) = (unsafe { firmware_runtime_from_state(state) }) else {
            return false;
        };
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let (phase, slot) = (&runtime.phase, &runtime.threads);
        if !matches!(phase.load(Ordering::Acquire), INSTALLING | RUNNING) {
            return false;
        }
        let slot = unsafe { &mut *slot.get() };
        if slot.is_some() {
            false
        } else {
            *slot = threads.take();
            true
        }
    }

    pub(crate) fn take_threads(&self, state: NonNull<T>) -> Option<crate::thread::ThreadGroup> {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let slot = {
            if !self.owns(state) {
                return None;
            }
            &self.threads
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: stop receives the exact loader state pointer retained by the
        // runtime tombstone for the lifetime of the firmware process.
        let runtime = unsafe { firmware_runtime_from_state(state) }?;
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let slot = &runtime.threads;
        unsafe { &mut *slot.get() }.take()
    }

    #[cfg(not(target_arch = "arm"))]
    pub(crate) fn record_app_data_callback(&self, state: NonNull<T>) -> bool {
        self.update_callbacks(state, |callbacks| callbacks.app_data = true)
    }

    #[cfg(not(target_arch = "arm"))]
    pub(crate) fn record_custom_config_callbacks(&self, state: NonNull<T>) -> bool {
        self.update_callbacks(state, |callbacks| callbacks.custom_config = true)
    }

    #[cfg(not(target_arch = "arm"))]
    pub(crate) fn clear_custom_config_registration(&self, state: NonNull<T>) -> bool {
        self.update_callbacks(state, |callbacks| callbacks.custom_config = false)
    }

    #[cfg(not(target_arch = "arm"))]
    pub(crate) fn record_imu_callback(&self, state: NonNull<T>) -> bool {
        self.update_callbacks(state, |callbacks| callbacks.imu = true)
    }

    #[cfg(not(target_arch = "arm"))]
    fn update_callbacks(
        &self,
        state: NonNull<T>,
        update: impl FnOnce(&mut CallbackRegistrations),
    ) -> bool {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let (phase, callbacks) = {
            if !self.owns(state) {
                return false;
            }
            (&self.phase, &self.callbacks)
        };
        #[cfg(target_arch = "arm")]
        let Some(runtime) = (unsafe { firmware_runtime_from_state(state) }) else {
            return false;
        };
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let (phase, callbacks) = (&runtime.phase, &runtime.callbacks);
        if !matches!(phase.load(Ordering::Acquire), INSTALLING | RUNNING) {
            return false;
        }
        update(unsafe { &mut *callbacks.get() });
        true
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn take_callbacks(&self, state: NonNull<T>) -> CallbackRegistrations {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let callbacks = {
            if !self.owns(state) {
                return CallbackRegistrations::default();
            }
            &self.callbacks
        };
        #[cfg(target_arch = "arm")]
        let Some(runtime) = (unsafe { firmware_runtime_from_state(state) }) else {
            return CallbackRegistrations::default();
        };
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let callbacks = &runtime.callbacks;
        core::mem::take(unsafe { &mut *callbacks.get() })
    }

    pub(crate) fn finish_stop(&self, state: NonNull<T>) {
        #[cfg(not(target_arch = "arm"))]
        let active = {
            if !self.owns(state) {
                return;
            }
            &self.active
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: stop receives the exact loader state pointer retained by the
        // runtime tombstone for the lifetime of the firmware process.
        let runtime = unsafe { firmware_runtime_from_state(state) }
            .expect("installed package state owns a runtime control block");
        #[cfg(target_arch = "arm")]
        let active = &runtime.active;
        while active.load(Ordering::Acquire) != 0 {
            #[cfg(target_arch = "arm")]
            unsafe {
                // SAFETY: VESC sleep is valid in the stop callback's thread context.
                crate::ffi::vesc_sleep_us(1);
            }
            #[cfg(not(target_arch = "arm"))]
            spin_loop();
        }
        #[cfg(not(target_arch = "arm"))]
        let borrow = self.borrow_exclusive();
        #[cfg(target_arch = "arm")]
        let borrow = self.borrow_exclusive(runtime);
        #[cfg(not(target_arch = "arm"))]
        self.state.store(core::ptr::null_mut(), Ordering::Release);
        drop(borrow);
        #[cfg(not(target_arch = "arm"))]
        self.phase.store(STOPPED, Ordering::Release);
        #[cfg(target_arch = "arm")]
        finish_firmware_runtime(runtime)
    }

    /// Whether startup has installed state.
    #[must_use]
    #[cfg(all(any(test, feature = "test-support"), not(target_arch = "arm")))]
    pub fn is_installed(&self) -> bool {
        !self.state.load(Ordering::Acquire).is_null()
    }

    /// Run `f` with installed package state, when present.
    #[inline(always)]
    #[must_use]
    #[cfg(not(target_arch = "arm"))]
    pub fn with<R>(&self, f: impl for<'state> FnOnce(&'state T) -> R) -> Option<R> {
        self.with_expected(ExpectedState::Any, f)
    }

    /// Run `f` with installed mutable package state, when present.
    #[inline(always)]
    #[must_use]
    #[cfg(not(target_arch = "arm"))]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.with_expected_mut(ExpectedState::Any, f)
    }

    #[inline(always)]
    pub(crate) fn with_expected<R>(
        &self,
        expected: ExpectedState<T>,
        f: impl for<'state> FnOnce(&'state T) -> R,
    ) -> Option<R> {
        #[cfg(not(target_arch = "arm"))]
        {
            let _entry = self.enter()?;
            let _borrow = self.borrow_exclusive();
            let state = self.running_state(expected)?;
            Some(f(unsafe { state.as_ref() }))
        }
        #[cfg(target_arch = "arm")]
        {
            let (state, runtime) = self.running_firmware(expected)?;
            let _entry = self.enter(runtime)?;
            let _borrow = self.borrow_exclusive(runtime);
            matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING)
                .then(|| f(unsafe { state.as_ref() }))
        }
    }

    #[inline(always)]
    pub(crate) fn with_expected_mut<R>(
        &self,
        expected: ExpectedState<T>,
        f: impl for<'state> FnOnce(&'state mut T) -> R,
    ) -> Option<R> {
        #[cfg(not(target_arch = "arm"))]
        {
            let _entry = self.enter()?;
            let _borrow = self.borrow_exclusive();
            let mut state = self.running_state(expected)?;
            Some(f(unsafe { state.as_mut() }))
        }
        #[cfg(target_arch = "arm")]
        {
            let (mut state, runtime) = self.running_firmware(expected)?;
            let _entry = self.enter(runtime)?;
            let _borrow = self.borrow_exclusive(runtime);
            matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING)
                .then(|| f(unsafe { state.as_mut() }))
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn enter(&self) -> Option<PackageStateEntry<'_, T>> {
        matches!(self.phase.load(Ordering::Acquire), INSTALLING | RUNNING).then_some(())?;
        self.active.fetch_add(1, Ordering::AcqRel);
        if matches!(self.phase.load(Ordering::Acquire), INSTALLING | RUNNING) {
            Some(PackageStateEntry {
                store: self,
                _state: PhantomData,
            })
        } else {
            self.active.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }

    #[cfg(target_arch = "arm")]
    fn enter<'runtime>(
        &self,
        runtime: &'runtime FirmwareRuntime,
    ) -> Option<PackageStateEntry<'runtime, T>> {
        enter_firmware(runtime).then(|| PackageStateEntry {
            runtime,
            _state: PhantomData,
        })
    }

    #[cfg(not(target_arch = "arm"))]
    fn running_state(&self, expected: ExpectedState<T>) -> Option<NonNull<T>> {
        matches!(self.phase.load(Ordering::Acquire), INSTALLING | RUNNING).then_some(())?;
        let state = NonNull::new(self.state.load(Ordering::Acquire))?;
        match expected {
            #[cfg(not(target_arch = "arm"))]
            ExpectedState::Any => Some(state),
            ExpectedState::Exact(expected) if expected == state => Some(state),
            ExpectedState::Exact(_) => None,
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[cfg_attr(not(target_arch = "arm"), allow(clippy::unused_self))]
    #[cfg_attr(target_arch = "arm", allow(clippy::infallible_destructuring_match))]
    fn running_firmware(
        &self,
        expected: ExpectedState<T>,
    ) -> Option<(NonNull<T>, &'static FirmwareRuntime)> {
        let state = match expected {
            ExpectedState::Exact(state) => state,
            #[cfg(not(target_arch = "arm"))]
            ExpectedState::Any => return None,
        };
        // SAFETY: target `ExpectedState::Exact` originates from the loader ARG
        // or a thread argument installed by this runtime.
        let runtime = unsafe { firmware_runtime_from_state(state) }?;
        matches!(runtime.phase.load(Ordering::Acquire), INSTALLING | RUNNING)
            .then_some((state, runtime))
    }
}

impl<T: Send + 'static> Default for PackageStateStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExpectedState, FirmwareRuntime, PackageStateAccess, PackageStateInstallError,
        PackageStateStore, RUNNING, RUNTIME_MAGIC, STOPPED, STOPPING, enter_firmware,
        finish_firmware_runtime, firmware_runtime_allocation_size, firmware_runtime_from_state,
        firmware_runtime_state_pointer,
    };
    use core::any::TypeId;
    use core::cell::UnsafeCell;
    use core::ptr::NonNull;
    use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicUsize, Ordering};
    use std::boxed::Box;

    #[derive(Debug, PartialEq, Eq)]
    struct State {
        value: u32,
    }

    #[repr(transparent)]
    struct WrongState(u32);

    static FALLBACK: AtomicPtr<State> = AtomicPtr::new(core::ptr::null_mut());

    unsafe fn fallback() -> Option<NonNull<State>> {
        NonNull::new(FALLBACK.load(Ordering::Acquire))
    }

    #[test]
    fn firmware_runtime_owns_its_state_lock() {
        let runtime = FirmwareRuntime {
            magic: RUNTIME_MAGIC,
            state_type: TypeId::of::<State>(),
            state: core::ptr::null_mut(),
            state_lock: AtomicBool::new(false),
            phase: AtomicU8::new(RUNNING),
            active: AtomicUsize::new(0),
            threads: UnsafeCell::new(None),
            callbacks: UnsafeCell::new(super::CallbackRegistrations::default()),
        };

        let borrow = super::borrow_firmware_runtime(&runtime);
        assert!(runtime.state_lock.load(Ordering::Acquire));
        drop(borrow);
        assert!(!runtime.state_lock.load(Ordering::Acquire));
    }

    #[test]
    fn firmware_runtime_rejects_a_caller_claiming_the_wrong_state_type() {
        let bytes = firmware_runtime_allocation_size::<State>().expect("runtime allocation size");
        let words = bytes.div_ceil(core::mem::size_of::<usize>());
        let mut backing = std::vec![0usize; words];
        let allocation = NonNull::new(backing.as_mut_ptr().cast()).expect("allocation");
        let state = unsafe { firmware_runtime_state_pointer::<State>(allocation) };
        let runtime = allocation.cast::<FirmwareRuntime>();
        unsafe {
            runtime.as_ptr().write(FirmwareRuntime {
                magic: RUNTIME_MAGIC,
                state_type: TypeId::of::<State>(),
                state: state.as_ptr().cast(),
                state_lock: AtomicBool::new(false),
                phase: AtomicU8::new(RUNNING),
                active: AtomicUsize::new(0),
                threads: UnsafeCell::new(None),
                callbacks: UnsafeCell::new(super::CallbackRegistrations::default()),
            });
        }

        assert!(unsafe { firmware_runtime_from_state(state.cast::<WrongState>()) }.is_none());
    }

    #[test]
    fn stopped_firmware_runtime_remains_a_tombstone_for_late_callbacks() {
        let bytes = firmware_runtime_allocation_size::<State>().expect("runtime allocation size");
        let words = bytes.div_ceil(core::mem::size_of::<usize>());
        let mut backing = std::vec![0usize; words];
        let allocation = NonNull::new(backing.as_mut_ptr().cast()).expect("allocation");
        let state = unsafe { firmware_runtime_state_pointer::<State>(allocation) };
        let runtime = allocation.cast::<FirmwareRuntime>();
        unsafe {
            runtime.as_ptr().write(FirmwareRuntime {
                magic: RUNTIME_MAGIC,
                state_type: TypeId::of::<State>(),
                state: state.as_ptr().cast(),
                state_lock: AtomicBool::new(false),
                phase: AtomicU8::new(STOPPING),
                active: AtomicUsize::new(0),
                threads: UnsafeCell::new(None),
                callbacks: UnsafeCell::new(super::CallbackRegistrations::default()),
            });
        }
        let runtime = unsafe { runtime.as_ref() };

        finish_firmware_runtime(runtime);

        assert_eq!(runtime.phase.load(Ordering::Acquire), STOPPED);
        assert!(unsafe { firmware_runtime_from_state(state) }.is_some());
        assert!(
            PackageStateStore::new()
                .running_firmware(ExpectedState::Exact(state))
                .is_none()
        );
    }

    #[test]
    fn callback_resolving_state_before_stop_cannot_enter_after_stop() {
        let bytes = firmware_runtime_allocation_size::<State>().expect("runtime allocation size");
        let words = bytes.div_ceil(core::mem::size_of::<usize>());
        let mut backing = std::vec![0usize; words];
        let allocation = NonNull::new(backing.as_mut_ptr().cast()).expect("allocation");
        let state = unsafe { firmware_runtime_state_pointer::<State>(allocation) };
        let runtime = allocation.cast::<FirmwareRuntime>();
        unsafe {
            runtime.as_ptr().write(FirmwareRuntime {
                magic: RUNTIME_MAGIC,
                state_type: TypeId::of::<State>(),
                state: state.as_ptr().cast(),
                state_lock: AtomicBool::new(false),
                phase: AtomicU8::new(RUNNING),
                active: AtomicUsize::new(0),
                threads: UnsafeCell::new(None),
                callbacks: UnsafeCell::new(super::CallbackRegistrations::default()),
            });
        }
        let runtime = unsafe { runtime.as_ref() };
        let state_address = state.as_ptr() as usize;
        let (resolved_tx, resolved_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let callback = std::thread::spawn(move || {
            let state = NonNull::new(state_address as *mut State).expect("state");
            let runtime = unsafe { firmware_runtime_from_state(state) }.expect("runtime");
            resolved_tx.send(()).expect("resolved");
            resume_rx.recv().expect("resume");
            enter_firmware(runtime)
        });

        resolved_rx.recv().expect("callback resolved state");
        runtime.phase.store(STOPPING, Ordering::Release);
        finish_firmware_runtime(runtime);
        resume_tx.send(()).expect("resume callback");

        assert!(!callback.join().expect("callback"));
        assert_eq!(runtime.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn runtime_slot_scopes_state_access() {
        let slot = PackageStateStore::new();
        let state = Box::leak(Box::new(State { value: 1 }));

        assert!(!slot.is_installed());
        unsafe { slot.install(state) }.unwrap();
        assert_eq!(slot.with(|state| state.value), Some(1));
        assert_eq!(slot.with_mut(|state| state.value += 10), Some(()));
        assert_eq!(slot.with(|state| state.value), Some(11));

        slot.clear();
        assert!(!slot.is_installed());
    }

    #[test]
    fn duplicate_install_does_not_replace_running_state() {
        let slot = PackageStateStore::new();
        let first = Box::leak(Box::new(State { value: 1 }));
        let second = Box::leak(Box::new(State { value: 2 }));

        unsafe { slot.install(first) }.unwrap();
        assert_eq!(
            unsafe { slot.install(second) },
            Err(PackageStateInstallError::AlreadyInstalled)
        );
        assert_eq!(slot.with(|state| state.value), Some(1));

        slot.clear();
    }

    #[test]
    fn contended_state_access_waits_without_dropping_mutation() {
        let slot: &'static PackageStateStore<State> = Box::leak(Box::new(PackageStateStore::new()));
        let state = Box::leak(Box::new(State { value: 1 }));
        unsafe { slot.install(state) }.unwrap();
        let (first_entered_tx, first_entered_rx) = std::sync::mpsc::channel();
        let (release_first_tx, release_first_rx) = std::sync::mpsc::channel();
        let (second_done_tx, second_done_rx) = std::sync::mpsc::channel();

        let first = std::thread::spawn(move || {
            slot.with_mut(|state| {
                first_entered_tx.send(()).unwrap();
                release_first_rx.recv().unwrap();
                state.value += 1;
            })
        });
        first_entered_rx.recv().unwrap();

        let second = std::thread::spawn(move || {
            second_done_tx
                .send(slot.with_mut(|state| state.value += 1))
                .unwrap();
        });

        assert!(
            second_done_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err()
        );
        release_first_tx.send(()).unwrap();
        assert_eq!(first.join().unwrap(), Some(()));
        assert_eq!(second_done_rx.recv().unwrap(), Some(()));
        second.join().unwrap();
        assert_eq!(slot.with(|state| state.value), Some(3));
    }

    #[test]
    fn clear_waits_for_every_admitted_state_access() {
        let slot: &'static PackageStateStore<State> = Box::leak(Box::new(PackageStateStore::new()));
        let state = Box::leak(Box::new(State { value: 0 }));
        unsafe { slot.install(state) }.unwrap();
        let admitted = slot.enter().unwrap();
        let (clear_done_tx, clear_done_rx) = std::sync::mpsc::channel();

        let clear = std::thread::spawn(move || {
            slot.clear();
            clear_done_tx.send(()).unwrap();
        });
        assert!(
            clear_done_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err()
        );

        drop(admitted);
        assert_eq!(clear_done_rx.recv().unwrap(), ());
        clear.join().unwrap();
        assert_eq!(slot.with(|state| state.value), None);
    }

    #[test]
    fn stop_during_install_waits_for_start_to_finish() {
        let slot: &'static PackageStateStore<State> = Box::leak(Box::new(PackageStateStore::new()));
        let state = Box::leak(Box::new(State { value: 0 }));
        let state_ptr = NonNull::from(&mut *state);
        unsafe { slot.install_owned(state, state_ptr.cast()) }.unwrap();
        assert!(slot.begin_stop(state_ptr));
        let (stop_done_tx, stop_done_rx) = std::sync::mpsc::channel();
        let state_address = state_ptr.as_ptr() as usize;

        let stop = std::thread::spawn(move || {
            let state_ptr = NonNull::new(state_address as *mut State).unwrap();
            slot.finish_stop(state_ptr);
            stop_done_tx.send(()).unwrap();
        });
        assert!(
            stop_done_rx
                .recv_timeout(std::time::Duration::from_millis(20))
                .is_err()
        );

        assert!(!slot.finish_start(state_ptr, true));
        assert_eq!(stop_done_rx.recv().unwrap(), ());
        stop.join().unwrap();
    }

    #[test]
    fn stop_takes_threads_before_termination_without_holding_the_state_gate() {
        let slot = PackageStateStore::new();
        let state = Box::leak(Box::new(State { value: 0 }));
        let state_ptr = NonNull::from(&mut *state);
        unsafe { slot.install(state) }.unwrap();
        let mut first_token = 0_u8;
        let mut second_token = 0_u8;
        let first = unsafe {
            crate::thread::ThreadHandle::from_firmware(core::ptr::from_mut(&mut first_token).cast())
        }
        .unwrap();
        let second = unsafe {
            crate::thread::ThreadHandle::from_firmware(
                core::ptr::from_mut(&mut second_token).cast(),
            )
        }
        .unwrap();
        let mut threads = Some(crate::thread::ThreadGroup::from_handles([first, second]));
        assert!(slot.install_threads(state_ptr, &mut threads));
        assert!(threads.is_none());

        assert!(slot.begin_stop(state_ptr));
        let _threads = slot
            .take_threads(state_ptr)
            .expect("installed thread group");

        assert!(!slot.host_lock.load(Ordering::Acquire));
        slot.finish_stop(state_ptr);
    }

    #[test]
    fn firmware_state_source_requires_installed_pointer_identity() {
        let runtime = PackageStateStore::new();
        let source = unsafe { PackageStateAccess::with_firmware_fallback(&runtime, fallback) };
        let runtime_state = Box::leak(Box::new(State { value: 11 }));
        let other_state = Box::leak(Box::new(State { value: 22 }));

        FALLBACK.store(other_state, Ordering::Release);
        assert_eq!(source.with(|state| state.value), None);
        unsafe { runtime.install(runtime_state) }.unwrap();
        assert_eq!(source.with(|state| state.value), None);
        FALLBACK.store(runtime_state, Ordering::Release);
        assert_eq!(source.with_mut(|state| state.value = 33), Some(()));
        assert_eq!(source.with(|state| state.value), Some(33));
        assert_eq!(other_state.value, 22);

        runtime.clear();
        assert_eq!(source.with(|state| state.value), None);
        FALLBACK.store(core::ptr::null_mut(), Ordering::Release);
    }

    #[test]
    fn stop_clears_only_callbacks_registered_by_the_package() {
        let runtime = PackageStateStore::new();
        let state = Box::leak(Box::new(State { value: 0 }));
        let state_ptr = NonNull::from(&mut *state);
        unsafe { runtime.install(state) }.unwrap();
        assert!(runtime.record_app_data_callback(state_ptr));
        assert!(runtime.record_imu_callback(state_ptr));

        let bindings = crate::test_support::FakeAppDataBindings::new();
        runtime
            .take_callbacks(state_ptr)
            .clear_registered(&bindings);
        assert_eq!(bindings.handler_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 0);
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);

        super::CallbackRegistrations::default().clear_registered(&bindings);
        assert_eq!(bindings.handler_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 0);
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);
        runtime.clear();
    }

    #[test]
    fn stale_state_cannot_stop_or_update_the_current_installation() {
        let runtime = PackageStateStore::new();
        let installed = Box::leak(Box::new(State { value: 0 }));
        let foreign = Box::leak(Box::new(State { value: 0 }));
        let installed_id = NonNull::from(&mut *installed);
        let foreign_id = NonNull::from(&mut *foreign);
        unsafe { runtime.install(installed) }.unwrap();

        assert!(!runtime.begin_stop(foreign_id));
        assert!(!runtime.record_app_data_callback(foreign_id));
        runtime.take_callbacks(foreign_id);
        assert!(runtime.record_app_data_callback(installed_id));

        runtime.clear();
    }
}
