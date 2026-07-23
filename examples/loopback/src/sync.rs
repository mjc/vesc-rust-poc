//! Usage-shaped RAII synchronization for package code.

use vescpkg_rs::prelude::SystemTicks;
use vescpkg_rs::{FirmwareMutex, FirmwareSemaphore, SemaphoreWaitOutcome};

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
    match semaphore.wait_timeout(SystemTicks::from_ticks(1)) {
        SemaphoreWaitOutcome::Signaled => Some(operation()),
        SemaphoreWaitOutcome::TimedOut => None,
    }
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

    #[test]
    fn package_mutex_guard_unlocks_and_frees_on_panic() {
        let firmware = FirmwareTest::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = with_probe_lock(|| panic!("probe panic"));
        }));

        assert!(result.is_err());
        assert_eq!(firmware.mutex_free_count(), 1);
    }

    #[test]
    fn package_semaphore_timeout_fails_closed_and_frees() {
        let firmware = FirmwareTest::new();
        firmware.fail_semaphore_timeout();

        assert_eq!(after_probe_signal(|| 42), None);
        assert_eq!(firmware.semaphore_free_count(), 1);
    }
}
