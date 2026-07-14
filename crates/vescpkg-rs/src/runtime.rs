//! Package state shared by startup and firmware callbacks.

use core::cell::UnsafeCell;
#[cfg(not(target_arch = "arm"))]
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::ptr::NonNull;
#[cfg(not(target_arch = "arm"))]
use core::sync::atomic::AtomicBool;
#[cfg(not(target_arch = "arm"))]
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

#[cfg(not(target_arch = "arm"))]
const EMPTY: u8 = 0;
#[cfg(not(target_arch = "arm"))]
const INSTALLING: u8 = 1;
const RUNNING: u8 = 2;
const STOPPING: u8 = 3;
const STOPPED: u8 = 4;

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
    threads: UnsafeCell<Option<crate::ThreadPair>>,
    _state: PhantomData<UnsafeCell<T>>,
}

#[cfg(target_arch = "arm")]
const _: [(); 0] = [(); core::mem::size_of::<PackageStateStore<()>>()];

#[cfg(target_arch = "arm")]
const RUNTIME_MAGIC: u32 = 0x5652_5354;

#[cfg(target_arch = "arm")]
#[repr(C)]
struct FirmwareRuntime {
    magic: u32,
    state: *mut core::ffi::c_void,
    allocation: *mut core::ffi::c_void,
    mutex: *mut core::ffi::c_void,
    phase: AtomicU8,
    active: AtomicUsize,
    threads: UnsafeCell<Option<crate::ThreadPair>>,
}

#[cfg(target_arch = "arm")]
// SAFETY: every mutable field is accessed while `mutex` is held, except the
// atomic lifecycle counters used to admit and drain those locked accesses.
unsafe impl Sync for FirmwareRuntime {}

#[cfg(target_arch = "arm")]
const fn firmware_state_alignment<T>() -> usize {
    let pointer = core::mem::align_of::<*mut FirmwareRuntime>();
    let state = core::mem::align_of::<T>();
    if state > pointer { state } else { pointer }
}

#[cfg(target_arch = "arm")]
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

#[cfg(target_arch = "arm")]
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

#[cfg(target_arch = "arm")]
/// Resolve the live runtime header installed immediately before `state`.
///
/// # Safety
///
/// `state` must be the `T` pointer produced by [`firmware_runtime_state_pointer`]
/// and its allocation must remain live for the returned reference.
unsafe fn firmware_runtime_from_state<T>(state: NonNull<T>) -> Option<&'static FirmwareRuntime> {
    let backlink = state
        .cast::<*mut FirmwareRuntime>()
        .as_ptr()
        .wrapping_sub(1);
    // SAFETY: the caller guarantees `state` was produced by the layout helper,
    // which initialized the in-bounds backlink slot.
    let runtime = unsafe { NonNull::new(backlink.read()) }?;
    // SAFETY: the caller also guarantees the runtime allocation remains live.
    let runtime = unsafe { runtime.as_ref() };
    (runtime.magic == RUNTIME_MAGIC && runtime.state == state.as_ptr().cast()).then_some(runtime)
}

// SAFETY: all access to `T` is serialized by the firmware mutex on device and
// by the host-only test lock otherwise. Moving access between firmware threads
// therefore requires `T: Send`, not `T: Sync`.
unsafe impl<T: Send> Sync for PackageStateStore<T> {}

/// Loader-owned package state with package-specific stop behavior.
pub trait PackageRuntimeState: Sized + Send + 'static {
    /// Return the callback-visible slot for this state.
    fn runtime_store() -> &'static PackageStateStore<Self>;

    /// Stop package-owned resources before the state is freed.
    fn stop(&mut self);
}

struct PackageStateBorrow<'a, T> {
    #[cfg(not(target_arch = "arm"))]
    store: &'a PackageStateStore<T>,
    #[cfg(target_arch = "arm")]
    runtime: &'a FirmwareRuntime,
    _state: PhantomData<&'a T>,
}

impl<T> Drop for PackageStateBorrow<'_, T> {
    fn drop(&mut self) {
        #[cfg(target_arch = "arm")]
        {
            // SAFETY: the guard exists only after locking this live runtime mutex.
            unsafe { crate::ffi::vesc_mutex_unlock(self.runtime.mutex) };
        }
        #[cfg(not(target_arch = "arm"))]
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
pub(crate) enum PackageStateInstallError {
    #[cfg(not(target_arch = "arm"))]
    AlreadyInstalled,
    #[cfg(target_arch = "arm")]
    MutexUnavailable,
}

pub(crate) struct PackageStateResources {
    #[cfg(target_arch = "arm")]
    pub(crate) allocation: Option<NonNull<core::ffi::c_void>>,
    #[cfg(target_arch = "arm")]
    pub(crate) mutex: Option<NonNull<core::ffi::c_void>>,
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
    pub const unsafe fn with_firmware_fallback(
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

    #[cfg(target_arch = "arm")]
    fn borrow_exclusive<'runtime>(
        &self,
        runtime: &'runtime FirmwareRuntime,
    ) -> PackageStateBorrow<'runtime, T> {
        // SAFETY: a running runtime owns this firmware mutex until stop drains guards.
        unsafe { crate::ffi::vesc_mutex_lock(runtime.mutex) };
        PackageStateBorrow {
            runtime,
            _state: PhantomData,
        }
    }

    /// Install package state for later callback access.
    ///
    /// # Safety
    ///
    /// `state` must outlive every callback that can access this slot, and the
    /// caller must clear the slot before freeing it.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) unsafe fn install(&self, state: &mut T) -> Result<(), PackageStateInstallError> {
        let allocation = NonNull::from(&mut *state).cast();
        unsafe { self.install_owned(state, allocation) }
    }

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
            self.phase.store(RUNNING, Ordering::Release);
            Ok(())
        }
        #[cfg(target_arch = "arm")]
        {
            // SAFETY: the VESC interface table is live throughout native package execution.
            let mutex = NonNull::new(unsafe { crate::ffi::vesc_mutex_create() })
                .ok_or(PackageStateInstallError::MutexUnavailable)?;
            let runtime = allocation.cast::<FirmwareRuntime>();
            // SAFETY: `allocation` reserves and aligns a `FirmwareRuntime` header
            // before `state`; startup has exclusive ownership of it here.
            unsafe {
                runtime.as_ptr().write(FirmwareRuntime {
                    magic: RUNTIME_MAGIC,
                    state: core::ptr::from_mut(state).cast(),
                    allocation: allocation.as_ptr(),
                    mutex: mutex.as_ptr(),
                    phase: AtomicU8::new(RUNNING),
                    active: AtomicUsize::new(0),
                    threads: UnsafeCell::new(None),
                });
            }
            Ok(())
        }
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
        let _ = self.finish_stop(state);
    }

    pub(crate) fn begin_stop(&self, state: NonNull<T>) -> bool {
        #[cfg(not(target_arch = "arm"))]
        let phase = {
            let _ = state;
            &self.phase
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: lifecycle methods receive the exact live loader state pointer.
        let Some(phase) =
            (unsafe { firmware_runtime_from_state(state) }).map(|runtime| &runtime.phase)
        else {
            return false;
        };
        phase
            .compare_exchange(RUNNING, STOPPING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub(crate) fn install_threads(
        &self,
        state: NonNull<T>,
        threads: crate::ThreadPair,
    ) -> Result<(), crate::ThreadPair> {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let (phase, slot) = {
            let _ = state;
            (&self.phase, &self.threads)
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: thread installation receives the exact live loader state pointer.
        let Some(runtime) = (unsafe { firmware_runtime_from_state(state) }) else {
            return Err(threads);
        };
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let (phase, slot) = (&runtime.phase, &runtime.threads);
        if phase.load(Ordering::Acquire) != RUNNING {
            return Err(threads);
        }
        let slot = unsafe { &mut *slot.get() };
        if slot.is_some() {
            Err(threads)
        } else {
            *slot = Some(threads);
            Ok(())
        }
    }

    pub(crate) fn take_threads(&self, state: NonNull<T>) -> Option<crate::ThreadPair> {
        #[cfg(not(target_arch = "arm"))]
        let _borrow = self.borrow_exclusive();
        #[cfg(not(target_arch = "arm"))]
        let slot = {
            let _ = state;
            &self.threads
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: stop receives the exact live loader state pointer before freeing it.
        let runtime = unsafe { firmware_runtime_from_state(state) }?;
        #[cfg(target_arch = "arm")]
        let _borrow = self.borrow_exclusive(runtime);
        #[cfg(target_arch = "arm")]
        let slot = &runtime.threads;
        unsafe { &mut *slot.get() }.take()
    }

    pub(crate) fn finish_stop(&self, state: NonNull<T>) -> PackageStateResources {
        #[cfg(not(target_arch = "arm"))]
        let active = {
            let _ = state;
            &self.active
        };
        #[cfg(target_arch = "arm")]
        // SAFETY: stop receives the exact live loader state pointer before freeing it.
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
        #[cfg(target_arch = "arm")]
        let allocation = NonNull::new(runtime.allocation);
        drop(borrow);
        #[cfg(target_arch = "arm")]
        let mutex = NonNull::new(runtime.mutex);
        #[cfg(not(target_arch = "arm"))]
        self.phase.store(STOPPED, Ordering::Release);
        #[cfg(target_arch = "arm")]
        runtime.phase.store(STOPPED, Ordering::Release);
        PackageStateResources {
            #[cfg(target_arch = "arm")]
            allocation,
            #[cfg(target_arch = "arm")]
            mutex,
        }
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
            (runtime.phase.load(Ordering::Acquire) == RUNNING).then(|| f(unsafe { state.as_ref() }))
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
            (runtime.phase.load(Ordering::Acquire) == RUNNING).then(|| f(unsafe { state.as_mut() }))
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn enter(&self) -> Option<PackageStateEntry<'_, T>> {
        (self.phase.load(Ordering::Acquire) == RUNNING).then_some(())?;
        self.active.fetch_add(1, Ordering::AcqRel);
        if self.phase.load(Ordering::Acquire) == RUNNING {
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
        (runtime.phase.load(Ordering::Acquire) == RUNNING).then_some(())?;
        runtime.active.fetch_add(1, Ordering::AcqRel);
        if runtime.phase.load(Ordering::Acquire) == RUNNING {
            Some(PackageStateEntry {
                runtime,
                _state: PhantomData,
            })
        } else {
            runtime.active.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn running_state(&self, expected: ExpectedState<T>) -> Option<NonNull<T>> {
        (self.phase.load(Ordering::Acquire) == RUNNING).then_some(())?;
        let state = NonNull::new(self.state.load(Ordering::Acquire))?;
        match expected {
            #[cfg(not(target_arch = "arm"))]
            ExpectedState::Any => Some(state),
            ExpectedState::Exact(expected) if expected == state => Some(state),
            ExpectedState::Exact(_) => None,
        }
    }

    #[cfg(target_arch = "arm")]
    fn running_firmware(
        &self,
        expected: ExpectedState<T>,
    ) -> Option<(NonNull<T>, &'static FirmwareRuntime)> {
        let ExpectedState::Exact(state) = expected;
        // SAFETY: target `ExpectedState::Exact` originates from the loader ARG
        // or a thread argument installed by this runtime.
        let runtime = unsafe { firmware_runtime_from_state(state) }?;
        (runtime.phase.load(Ordering::Acquire) == RUNNING).then_some((state, runtime))
    }
}

impl<T: Send + 'static> Default for PackageStateStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageStateAccess, PackageStateInstallError, PackageStateStore};
    use core::ptr::NonNull;
    use core::sync::atomic::{AtomicPtr, Ordering};
    use std::boxed::Box;

    #[derive(Debug, PartialEq, Eq)]
    struct State {
        value: u32,
    }

    static FALLBACK: AtomicPtr<State> = AtomicPtr::new(core::ptr::null_mut());

    unsafe fn fallback() -> Option<NonNull<State>> {
        NonNull::new(FALLBACK.load(Ordering::Acquire))
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
}
