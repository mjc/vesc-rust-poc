//! RAII synchronization handles for the Express shared runtime.

use super::functions::{LibMutex, LibSemaphore};
use super::{ExpressCallError, ExpressRuntime};

/// Error returned while creating an Express synchronization handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressSyncError {
    /// The corresponding firmware function slot was absent.
    Unavailable(ExpressCallError),
    /// Firmware returned a null handle.
    NullHandle,
}

/// Firmware-owned Express mutex released on drop.
#[derive(Debug)]
pub struct ExpressMutex<'a> {
    runtime: ExpressRuntime<'a>,
    handle: LibMutex,
}

impl<'a> ExpressMutex<'a> {
    /// Create a firmware mutex and retain ownership until drop.
    pub fn new(runtime: ExpressRuntime<'a>) -> Result<Self, ExpressSyncError> {
        let handle = runtime
            .mutex_create()
            .map_err(ExpressSyncError::Unavailable)?;
        if handle.is_null() {
            return Err(ExpressSyncError::NullHandle);
        }
        Ok(Self { runtime, handle })
    }

    /// Lock the mutex and return a guard that unlocks on drop.
    pub fn lock(&self) -> Result<ExpressMutexGuard<'_, 'a>, ExpressCallError> {
        self.runtime.mutex_lock(self.handle)?;
        Ok(ExpressMutexGuard { mutex: self })
    }
}

impl Drop for ExpressMutex<'_> {
    fn drop(&mut self) {
        let _ = self.runtime.free(self.handle);
    }
}

/// An Express mutex lock that unlocks when dropped.
#[derive(Debug)]
pub struct ExpressMutexGuard<'mutex, 'runtime> {
    mutex: &'mutex ExpressMutex<'runtime>,
}

impl Drop for ExpressMutexGuard<'_, '_> {
    fn drop(&mut self) {
        let _ = self.mutex.runtime.mutex_unlock(self.mutex.handle);
    }
}

/// Firmware-owned Express semaphore released on drop.
#[derive(Debug)]
pub struct ExpressSemaphore<'a> {
    runtime: ExpressRuntime<'a>,
    handle: LibSemaphore,
}

impl<'a> ExpressSemaphore<'a> {
    /// Create a firmware semaphore and retain ownership until drop.
    pub fn new(runtime: ExpressRuntime<'a>) -> Result<Self, ExpressSyncError> {
        let handle = runtime
            .semaphore_create()
            .map_err(ExpressSyncError::Unavailable)?;
        if handle.is_null() {
            return Err(ExpressSyncError::NullHandle);
        }
        Ok(Self { runtime, handle })
    }

    /// Wait until the semaphore is signaled.
    pub fn wait(&self) -> Result<(), ExpressCallError> {
        self.runtime.semaphore_wait(self.handle)
    }

    /// Signal the semaphore.
    pub fn signal(&self) -> Result<(), ExpressCallError> {
        self.runtime.semaphore_signal(self.handle)
    }

    /// Wait for at most the supplied firmware ticks.
    pub fn wait_to(&self, ticks: u32) -> Result<bool, ExpressCallError> {
        self.runtime.semaphore_wait_to(self.handle, ticks)
    }

    /// Reset the semaphore.
    pub fn reset(&self) -> Result<(), ExpressCallError> {
        self.runtime.semaphore_reset(self.handle)
    }
}

impl Drop for ExpressSemaphore<'_> {
    fn drop(&mut self) {
        let _ = self.runtime.free(self.handle);
    }
}
