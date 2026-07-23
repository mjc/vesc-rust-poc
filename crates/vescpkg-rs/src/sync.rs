//! RAII wrappers for firmware-owned synchronization primitives.

use core::ffi::c_void;
use core::ptr::NonNull;

use crate::types::SystemDuration;
use crate::units::SystemTicks;

/// Outcome of a bounded firmware semaphore wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreWaitOutcome {
    /// The semaphore was signaled before the timeout elapsed.
    Signaled,
    /// The timeout elapsed before the semaphore was signaled.
    TimedOut,
}

impl SemaphoreWaitOutcome {
    /// Return whether the wait acquired a signal.
    #[must_use]
    pub const fn is_signaled(self) -> bool {
        matches!(self, Self::Signaled)
    }

    /// Return whether the wait elapsed without acquiring a signal.
    #[must_use]
    pub const fn is_timed_out(self) -> bool {
        matches!(self, Self::TimedOut)
    }
}

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

    /// Wait for at most `timeout` system ticks and report the outcome.
    pub fn wait_timeout(&self, timeout: SystemTicks) -> SemaphoreWaitOutcome {
        semaphore_wait_outcome(unsafe {
            crate::ffi::vesc_sem_wait_to(self.handle.as_ptr(), timeout.as_ticks())
        })
    }

    /// Wait for a typed system-clock duration and report the outcome.
    pub fn wait_timeout_duration(&self, timeout: SystemDuration) -> SemaphoreWaitOutcome {
        self.wait_timeout(timeout.duration())
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

fn semaphore_wait_outcome(signaled: bool) -> SemaphoreWaitOutcome {
    if signaled {
        SemaphoreWaitOutcome::Signaled
    } else {
        SemaphoreWaitOutcome::TimedOut
    }
}

impl Drop for FirmwareSemaphore {
    fn drop(&mut self) {
        unsafe { crate::ffi::vesc_free(self.handle.as_ptr()) };
    }
}
