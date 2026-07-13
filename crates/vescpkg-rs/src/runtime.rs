//! Package state shared by startup and firmware callbacks.

use core::hint::spin_loop;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// Mutable package state installed by firmware startup and accessed by callbacks.
///
/// The state pointer never escapes this type. Callers use [`Self::with`] or
/// [`Self::with_mut`], which keeps the temporary state borrow within the callback.
pub struct PackageStateStore<T> {
    state: AtomicPtr<T>,
    borrowed: AtomicBool,
}

struct PackageStateBorrow<'a, T> {
    store: &'a PackageStateStore<T>,
}

impl<T> Drop for PackageStateBorrow<'_, T> {
    fn drop(&mut self) {
        self.store.borrowed.store(false, Ordering::Release);
    }
}

/// Package state shared by package callbacks.
///
/// C map: `ARG(PROG_ADDR)` resolves the package's loader-owned state at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:697-700`; VESC implements that lookup
/// in `third_party/vesc/lispBM/lispif_c_lib.c:151-158`. The runtime slot is the
/// Rust-owned fast path, while `fallback` preserves the firmware lookup used by
/// callbacks that can run without that slot installed.
pub struct PackageStateAccess<'a, T: 'static> {
    runtime: &'a PackageStateStore<T>,
    fallback: unsafe fn() -> Option<NonNull<T>>,
}

unsafe fn no_package_state<T>() -> Option<NonNull<T>> {
    None
}

impl<T: 'static> Clone for PackageStateAccess<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for PackageStateAccess<'_, T> {}

impl<'a, T: 'static> PackageStateAccess<'a, T> {
    /// Build a source backed only by the installed runtime slot.
    #[must_use]
    pub const fn runtime(runtime: &'a PackageStateStore<T>) -> Self {
        Self {
            runtime,
            fallback: no_package_state::<T>,
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
        fallback: unsafe fn() -> Option<NonNull<T>>,
    ) -> Self {
        Self { runtime, fallback }
    }

    /// Run `f` with package state, preferring the runtime slot over loader state.
    #[inline(always)]
    #[must_use]
    pub fn with<R>(&self, f: impl for<'state> FnOnce(&'state T) -> R) -> Option<R> {
        self.runtime.with_fallback(self.fallback, |state| f(state))
    }

    /// Run `f` with mutable package state, preferring the runtime slot over loader state.
    #[inline(always)]
    #[must_use]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.runtime.with_mut_fallback(self.fallback, f)
    }
}

impl<T: 'static> PackageStateStore<T> {
    /// Create an empty package-state slot.
    pub const fn new() -> Self {
        Self {
            state: AtomicPtr::new(core::ptr::null_mut()),
            borrowed: AtomicBool::new(false),
        }
    }

    fn try_borrow(&self) -> Option<PackageStateBorrow<'_, T>> {
        self.borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| PackageStateBorrow { store: self })
    }

    fn borrow_exclusive(&self) -> PackageStateBorrow<'_, T> {
        while self
            .borrowed
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
    #[cfg(test)]
    pub(crate) unsafe fn install(&self, state: &mut T) {
        self.state
            .store(core::ptr::from_mut(state), Ordering::Release);
    }

    /// Clear the installed state pointer.
    pub fn clear(&self) {
        let _borrow = self.borrow_exclusive();
        self.state.store(core::ptr::null_mut(), Ordering::Release);
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
        self.with_fallback(no_package_state::<T>, f)
    }

    /// Run `f` with installed mutable package state, when present.
    #[inline(always)]
    #[must_use]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.with_mut_fallback(no_package_state::<T>, f)
    }

    #[inline(always)]
    fn with_fallback<R>(
        &self,
        fallback: unsafe fn() -> Option<NonNull<T>>,
        f: impl for<'state> FnOnce(&'state T) -> R,
    ) -> Option<R> {
        let _borrow = self.try_borrow()?;
        let state =
            NonNull::new(self.state.load(Ordering::Acquire)).or_else(|| unsafe { fallback() })?;
        Some(f(unsafe { state.as_ref() }))
    }

    #[inline(always)]
    fn with_mut_fallback<R>(
        &self,
        fallback: unsafe fn() -> Option<NonNull<T>>,
        f: impl for<'state> FnOnce(&'state mut T) -> R,
    ) -> Option<R> {
        let _borrow = self.try_borrow()?;
        let mut state =
            NonNull::new(self.state.load(Ordering::Acquire)).or_else(|| unsafe { fallback() })?;
        Some(f(unsafe { state.as_mut() }))
    }
}

impl<T: 'static> Default for PackageStateStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageStateAccess, PackageStateStore};
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
        unsafe { slot.install(state) };
        assert_eq!(slot.with(|state| state.value), Some(1));
        assert_eq!(slot.with_mut(|state| state.value += 10), Some(()));
        assert_eq!(slot.with(|state| state.value), Some(11));

        slot.clear();
        assert!(!slot.is_installed());
    }

    #[test]
    fn runtime_slot_rejects_reentrant_state_borrows() {
        let slot = PackageStateStore::new();
        let state = Box::leak(Box::new(State { value: 1 }));
        unsafe { slot.install(state) };

        assert_eq!(
            slot.with_mut(|state| {
                state.value += 1;
                assert_eq!(slot.with(|_| ()), None);
                assert_eq!(slot.with_mut(|_| ()), None);
            }),
            Some(())
        );
        assert_eq!(slot.with(|state| state.value), Some(2));
    }

    #[test]
    fn state_source_prefers_runtime_then_loader_fallback() {
        let runtime = PackageStateStore::new();
        let source = unsafe { PackageStateAccess::with_firmware_fallback(&runtime, fallback) };
        let fallback = Box::leak(Box::new(State { value: 22 }));
        let runtime_state = Box::leak(Box::new(State { value: 11 }));
        FALLBACK.store(fallback, Ordering::Release);

        assert_eq!(source.with(|state| state.value), Some(22));
        unsafe { runtime.install(runtime_state) };
        assert_eq!(source.with_mut(|state| state.value = 33), Some(()));
        assert_eq!(source.with(|state| state.value), Some(33));
        assert_eq!(fallback.value, 22);

        runtime.clear();
        FALLBACK.store(core::ptr::null_mut(), Ordering::Release);
    }
}
