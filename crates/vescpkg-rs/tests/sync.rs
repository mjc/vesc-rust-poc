#![cfg(feature = "test-support")]

//! Integration tests for RAII firmware synchronization.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{FirmwareMutex, FirmwareSemaphore, SystemTicks};

#[test]
fn mutex_guard_unlocks_before_owned_mutex_is_released() {
    let firmware = FirmwareTest::new();
    let mutex = FirmwareMutex::new().expect("fake firmware creates a mutex");

    {
        let _guard = mutex.lock();
        assert_eq!(firmware.mutex_lock_count(), 1);
        assert_eq!(firmware.mutex_unlock_count(), 0);
    }

    assert_eq!(firmware.mutex_unlock_count(), 1);
    drop(mutex);
    assert_eq!(firmware.mutex_free_count(), 1);
}

#[test]
fn semaphore_exposes_wait_timeout_signal_reset_and_release() {
    let firmware = FirmwareTest::new();
    let semaphore = FirmwareSemaphore::new().expect("fake firmware creates a semaphore");

    semaphore.wait();
    assert!(semaphore.wait_timeout(SystemTicks::from_ticks(25)));
    semaphore.signal();
    semaphore.reset();

    assert_eq!(firmware.semaphore_wait_count(), 1);
    assert_eq!(firmware.semaphore_timed_wait_ticks(), Some(25));
    assert_eq!(firmware.semaphore_signal_count(), 1);
    assert_eq!(firmware.semaphore_reset_count(), 1);
    drop(semaphore);
    assert_eq!(firmware.semaphore_free_count(), 1);
}
