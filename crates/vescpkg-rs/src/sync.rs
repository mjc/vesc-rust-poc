//! RAII wrappers for firmware-owned synchronization primitives.

use core::ffi::c_void;
use core::ptr::NonNull;

use crate::units::SystemTicks;

/// An owned firmware mutex allocated from the VESC package heap.
///
/// The handle is deliberately neither `Send` nor `Sync`: firmware mutex
/// ownership and the thread that may use it are provider-defined.
///
/// ```compile_fail
/// use vescpkg_rs::FirmwareMutex;
/// fn requires_send<T: Send>() {}
/// requires_send::<FirmwareMutex>();
/// ```
pub struct FirmwareMutex {
    handle: NonNull<c_void>,
}

impl FirmwareMutex {
    /// Create a firmware mutex, returning `None` when unavailable or allocation fails.
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

/// An owned firmware semaphore allocated from the VESC package heap.
///
/// Semaphore handles remain thread-affine until the provider documents a
/// cross-thread ownership contract.
///
/// ```compile_fail
/// use vescpkg_rs::FirmwareSemaphore;
/// fn requires_sync<T: Sync>() {}
/// requires_sync::<FirmwareSemaphore>();
/// ```
pub struct FirmwareSemaphore {
    handle: NonNull<c_void>,
}

impl FirmwareSemaphore {
    /// Create a firmware semaphore, returning `None` when unavailable or allocation fails.
    #[must_use]
    pub fn new() -> Option<Self> {
        NonNull::new(unsafe { crate::ffi::vesc_sem_create() }).map(|handle| Self { handle })
    }

    /// Block until the semaphore is signaled.
    pub fn wait(&self) {
        unsafe { crate::ffi::vesc_sem_wait(self.handle.as_ptr()) };
    }

    /// Wait for at most `timeout` system ticks, returning `false` on timeout.
    pub fn wait_timeout(&self, timeout: SystemTicks) -> bool {
        unsafe { crate::ffi::vesc_sem_wait_to(self.handle.as_ptr(), timeout.as_ticks()) }
    }

    /// Signal the semaphore.
    pub fn signal(&self) {
        unsafe { crate::ffi::vesc_sem_signal(self.handle.as_ptr()) };
    }

    /// Reset the semaphore to its unsignaled state.
    pub fn reset(&self) {
        unsafe { crate::ffi::vesc_sem_reset(self.handle.as_ptr()) };
    }
}

impl Drop for FirmwareSemaphore {
    fn drop(&mut self) {
        unsafe { crate::ffi::vesc_free(self.handle.as_ptr()) };
    }
}
