//! Usage-shaped RAII synchronization for package code.

use vescpkg_rs::prelude::SystemTicks;
use vescpkg_rs::{FirmwareMutex, FirmwareSemaphore};

/// Run a package operation while holding a firmware-owned mutex.
pub fn with_probe_lock<R>(operation: impl FnOnce() -> R) -> Option<R> {
    let mutex = FirmwareMutex::new()?;
    let _guard = mutex.lock();
    Some(operation())
}

/// Run a package operation after a bounded firmware-semaphore wait.
pub fn after_probe_signal<R>(operation: impl FnOnce() -> R) -> Option<R> {
    let semaphore = FirmwareSemaphore::new()?;
    semaphore.signal();
    semaphore
        .wait_timeout(SystemTicks::from_ticks(1))
        .then_some(operation())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{after_probe_signal, with_probe_lock};
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn package_sync_operation_uses_a_scoped_firmware_mutex() {
        let _firmware = FirmwareTest::new();
        assert_eq!(with_probe_lock(|| 42), Some(42));
    }

    #[test]
    fn package_sync_operation_uses_a_bounded_firmware_semaphore_wait() {
        let _firmware = FirmwareTest::new();
        assert_eq!(after_probe_signal(|| 42), Some(42));
    }
}
