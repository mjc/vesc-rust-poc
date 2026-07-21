//! RAII wrappers for firmware-owned synchronization primitives.

use core::ffi::c_void;
use core::ptr::NonNull;

/// An owned firmware mutex allocated from the VESC package heap.
///
/// The handle is deliberately neither `Send` nor `Sync`: firmware mutex
/// ownership and the thread that may use it are provider-defined.
pub struct FirmwareMutex {
    handle: NonNull<c_void>,
}

impl FirmwareMutex {
    /// Create a firmware mutex, returning `None` when firmware allocation fails.
    #[must_use]
    pub fn new() -> Option<Self> {
        NonNull::new(unsafe { crate::ffi::vesc_mutex_create() }).map(|handle| Self { handle })
    }

    /// Lock this mutex and return a guard that unlocks it on every drop path.
    #[must_use]
    pub fn lock(&self) -> FirmwareMutexGuard<'_> {
        unsafe { crate::ffi::vesc_mutex_lock(self.handle.as_ptr()) };
        FirmwareMutexGuard { mutex: self }
    }
}

impl Drop for FirmwareMutex {
    fn drop(&mut self) {
        unsafe { crate::ffi::vesc_free(self.handle.as_ptr()) };
    }
}

/// A locked firmware mutex.
pub struct FirmwareMutexGuard<'a> {
    mutex: &'a FirmwareMutex,
}

impl Drop for FirmwareMutexGuard<'_> {
    fn drop(&mut self) {
        unsafe { crate::ffi::vesc_mutex_unlock(self.mutex.handle.as_ptr()) };
    }
}
