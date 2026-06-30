//! Package state shared by startup and firmware callbacks.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, Ordering};

/// Mutable package state installed by firmware startup and accessed by callbacks.
///
/// The state pointer never escapes this type. Callers use [`Self::with`] or
/// [`Self::with_mut`], which keeps the temporary state borrow within the callback.
pub struct PackageStateStore<T> {
    state: AtomicPtr<T>,
}

/// Runtime-state publication cleared on drop unless committed.
pub struct PackageStateGuard<'a, T: 'static> {
    slot: &'a PackageStateStore<T>,
    committed: bool,
}

impl<T> PackageStateGuard<'_, T> {
    /// Keep the installed runtime state after the guard is dropped.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl<T: 'static> Drop for PackageStateGuard<'_, T> {
    fn drop(&mut self) {
        if !self.committed {
            self.slot.clear();
        }
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
    fallback: fn() -> Option<&'static mut T>,
}

fn no_package_state<T>() -> Option<&'static mut T> {
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
    /// `Some`. The pointed-to state must remain valid for the duration of each callback.
    pub const fn with_firmware_fallback(
        runtime: &'a PackageStateStore<T>,
        fallback: fn() -> Option<&'static mut T>,
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
        }
    }

    /// Install package state for later callback access.
    ///
    /// # Safety
    ///
    /// `state` must outlive every callback that can access this slot, and the
    /// caller must clear the slot before freeing it.
    pub(crate) unsafe fn install(&self, state: &mut T) {
        self.state
            .store(core::ptr::from_mut(state), Ordering::Release);
    }

    /// Install package state that is cleared unless the returned guard commits.
    ///
    /// # Safety
    ///
    /// Same requirements as [`Self::install`].
    pub(crate) unsafe fn install_guard(&self, state: &mut T) -> PackageStateGuard<'_, T> {
        unsafe { self.install(state) };
        PackageStateGuard {
            slot: self,
            committed: false,
        }
    }

    /// Clear the installed state pointer.
    pub fn clear(&self) {
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
        let state = NonNull::new(self.state.load(Ordering::Acquire))?;
        Some(f(unsafe { state.as_ref() }))
    }

    /// Run `f` with installed mutable package state, when present.
    #[inline(always)]
    #[must_use]
    pub fn with_mut<R>(&self, f: impl for<'state> FnOnce(&'state mut T) -> R) -> Option<R> {
        self.with_mut_fallback(|| None, f)
    }

    #[inline(always)]
    fn with_fallback<R>(
        &self,
        fallback: fn() -> Option<&'static mut T>,
        f: impl for<'state> FnOnce(&'state T) -> R,
    ) -> Option<R> {
        let state = NonNull::new(self.state.load(Ordering::Acquire))
            .map(|state| unsafe { state.as_ref() })
            .or_else(|| fallback().map(|state| &*state))?;
        Some(f(state))
    }

    #[inline(always)]
    fn with_mut_fallback<R>(
        &self,
        fallback: fn() -> Option<&'static mut T>,
        f: impl for<'state> FnOnce(&'state mut T) -> R,
    ) -> Option<R> {
        let state = NonNull::new(self.state.load(Ordering::Acquire))
            .map(|mut state| unsafe { state.as_mut() })
            .or_else(fallback)?;
        Some(f(state))
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

    fn fallback() -> Option<&'static mut State> {
        NonNull::new(FALLBACK.load(Ordering::Acquire)).map(|mut state| unsafe { state.as_mut() })
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
    fn state_source_prefers_runtime_then_loader_fallback() {
        let runtime = PackageStateStore::new();
        let source = PackageStateAccess::with_firmware_fallback(&runtime, fallback);
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
