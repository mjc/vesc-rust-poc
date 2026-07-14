//! Package state shared by startup and firmware callbacks.

use core::cell::UnsafeCell;
#[cfg(not(target_arch = "arm"))]
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::ptr::NonNull;
#[cfg(not(target_arch = "arm"))]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering};

const EMPTY: u8 = 0;
const INSTALLING: u8 = 1;
const RUNNING: u8 = 2;
const STOPPING: u8 = 3;
const STOPPED: u8 = 4;

/// Mutable package state installed by firmware startup and accessed by callbacks.
///
/// The state pointer never escapes this type. Callers use [`Self::with`] or
/// [`Self::with_mut`], which keeps the temporary state borrow within the callback.
pub struct PackageStateStore<T> {
    state: AtomicPtr<T>,
    #[cfg(target_arch = "arm")]
    allocation: AtomicPtr<core::ffi::c_void>,
    #[cfg(target_arch = "arm")]
    mutex: AtomicPtr<core::ffi::c_void>,
    phase: AtomicU8,
    active: AtomicUsize,
    #[cfg(not(target_arch = "arm"))]
    host_lock: AtomicBool,
    threads: UnsafeCell<Option<crate::ThreadPair>>,
    _state: PhantomData<UnsafeCell<T>>,
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
    store: &'a PackageStateStore<T>,
}

impl<T> Drop for PackageStateBorrow<'_, T> {
    fn drop(&mut self) {
        #[cfg(target_arch = "arm")]
        unsafe {
            crate::ffi::vesc_mutex_unlock(self.store.mutex.load(Ordering::Acquire));
        }
        #[cfg(not(target_arch = "arm"))]
        self.store.host_lock.store(false, Ordering::Release);
    }
}

struct PackageStateEntry<'a, T> {
    store: &'a PackageStateStore<T>,
}

impl<T> Drop for PackageStateEntry<'_, T> {
    fn drop(&mut self) {
        self.store.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageStateInstallError {
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
    Installed,
    Firmware(unsafe fn() -> Option<NonNull<T>>),
}

pub(crate) enum ExpectedState<T> {
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
            StateIdentity::Installed => Some(ExpectedState::Any),
            StateIdentity::Firmware(state) => unsafe { state() }.map(ExpectedState::Exact),
        }
    }
}

impl<T: Send + 'static> PackageStateStore<T> {
    /// Create an empty package-state slot.
    pub const fn new() -> Self {
        Self {
            state: AtomicPtr::new(core::ptr::null_mut()),
            #[cfg(target_arch = "arm")]
            allocation: AtomicPtr::new(core::ptr::null_mut()),
            #[cfg(target_arch = "arm")]
            mutex: AtomicPtr::new(core::ptr::null_mut()),
            phase: AtomicU8::new(EMPTY),
            active: AtomicUsize::new(0),
            #[cfg(not(target_arch = "arm"))]
            host_lock: AtomicBool::new(false),
            threads: UnsafeCell::new(None),
            _state: PhantomData,
        }
    }

    fn borrow_exclusive(&self) -> PackageStateBorrow<'_, T> {
        #[cfg(target_arch = "arm")]
        {
            let mutex = NonNull::new(self.mutex.load(Ordering::Acquire))
                .expect("running package state owns a mutex");
            unsafe { crate::ffi::vesc_mutex_lock(mutex.as_ptr()) };
        }
        #[cfg(not(target_arch = "arm"))]
        while self
            .host_lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        PackageStateBorrow { store: self }
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
        let phase = self.phase.load(Ordering::Acquire);
        if !matches!(phase, EMPTY | STOPPED)
            || self
                .phase
                .compare_exchange(phase, INSTALLING, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(PackageStateInstallError::AlreadyInstalled);
        }

        #[cfg(target_arch = "arm")]
        {
            let mutex =
                NonNull::new(unsafe { crate::ffi::vesc_mutex_create() }).ok_or_else(|| {
                    self.phase.store(EMPTY, Ordering::Release);
                    PackageStateInstallError::MutexUnavailable
                })?;
            self.mutex.store(mutex.as_ptr(), Ordering::Release);
        }
        self.state
            .store(core::ptr::from_mut(state), Ordering::Release);
        #[cfg(target_arch = "arm")]
        self.allocation
            .store(allocation.as_ptr(), Ordering::Release);
        #[cfg(not(target_arch = "arm"))]
        let _ = allocation;
        self.phase.store(RUNNING, Ordering::Release);
        Ok(())
    }

    /// Clear the installed state pointer.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn clear(&self) {
        self.begin_stop();
        let _ = self.take_threads();
        let _ = self.finish_stop();
    }

    pub(crate) fn begin_stop(&self) {
        let _ = self
            .phase
            .compare_exchange(RUNNING, STOPPING, Ordering::AcqRel, Ordering::Acquire);
    }

    pub(crate) fn install_threads(
        &self,
        threads: crate::ThreadPair,
    ) -> Result<(), crate::ThreadPair> {
        let _borrow = self.borrow_exclusive();
        if self.phase.load(Ordering::Acquire) != RUNNING {
            return Err(threads);
        }
        let slot = unsafe { &mut *self.threads.get() };
        if slot.is_some() {
            Err(threads)
        } else {
            *slot = Some(threads);
            Ok(())
        }
    }

    pub(crate) fn take_threads(&self) -> Option<crate::ThreadPair> {
        let _borrow = self.borrow_exclusive();
        unsafe { &mut *self.threads.get() }.take()
    }

    pub(crate) fn finish_stop(&self) -> PackageStateResources {
        while self.active.load(Ordering::Acquire) != 0 {
            #[cfg(target_arch = "arm")]
            unsafe {
                crate::ffi::vesc_sleep_us(1);
            }
            #[cfg(not(target_arch = "arm"))]
            spin_loop();
        }
        let borrow = self.borrow_exclusive();
        self.state.store(core::ptr::null_mut(), Ordering::Release);
        #[cfg(target_arch = "arm")]
        let allocation = NonNull::new(
            self.allocation
                .swap(core::ptr::null_mut(), Ordering::AcqRel),
        );
        drop(borrow);
        #[cfg(target_arch = "arm")]
        let mutex = NonNull::new(self.mutex.swap(core::ptr::null_mut(), Ordering::AcqRel));
        self.phase.store(STOPPED, Ordering::Release);
        PackageStateResources {
            #[cfg(target_arch = "arm")]
            allocation,
            #[cfg(target_arch = "arm")]
            mutex,
        }
    }

    /// Whether startup has installed state.
    #[must_use]
    #[cfg(any(test, feature = "test-support"))]
    pub fn is_installed(&self) -> bool {
        !self.state.load(Ordering::Acquire).is_null()
    }

    /// Run `f` with installed package state, when present.
    #[inline(always)]
    #[must_use]
    pub fn with<R>(&self, f: impl for<'state> FnOnce(&'state T) -> R) -> Option<R> {
        self.with_expected(ExpectedState::Any, f)
    }

    /// Run `f` with installed mutable package state, when present.
    #[inline(always)]
    #[must_use]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.with_expected_mut(ExpectedState::Any, f)
    }

    #[inline(always)]
    pub(crate) fn with_expected<R>(
        &self,
        expected: ExpectedState<T>,
        f: impl for<'state> FnOnce(&'state T) -> R,
    ) -> Option<R> {
        let _entry = self.enter()?;
        let _borrow = self.borrow_exclusive();
        let state = self.running_state(expected)?;
        Some(f(unsafe { state.as_ref() }))
    }

    #[inline(always)]
    pub(crate) fn with_expected_mut<R>(
        &self,
        expected: ExpectedState<T>,
        f: impl for<'state> FnOnce(&'state mut T) -> R,
    ) -> Option<R> {
        let _entry = self.enter()?;
        let _borrow = self.borrow_exclusive();
        let mut state = self.running_state(expected)?;
        Some(f(unsafe { state.as_mut() }))
    }

    fn enter(&self) -> Option<PackageStateEntry<'_, T>> {
        (self.phase.load(Ordering::Acquire) == RUNNING).then_some(())?;
        self.active.fetch_add(1, Ordering::AcqRel);
        if self.phase.load(Ordering::Acquire) == RUNNING {
            Some(PackageStateEntry { store: self })
        } else {
            self.active.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }

    fn running_state(&self, expected: ExpectedState<T>) -> Option<NonNull<T>> {
        (self.phase.load(Ordering::Acquire) == RUNNING).then_some(())?;
        let state = NonNull::new(self.state.load(Ordering::Acquire))?;
        match expected {
            ExpectedState::Any => Some(state),
            ExpectedState::Exact(expected) if expected == state => Some(state),
            ExpectedState::Exact(_) => None,
        }
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
