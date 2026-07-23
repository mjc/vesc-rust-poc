#![cfg(feature = "test-support")]

//! Integration tests for RAII firmware synchronization.

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{
    FirmwareMutex, FirmwareSemaphore, SemaphoreWaitOutcome, SystemDuration, SystemTicks,
};

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
    assert_eq!(
        semaphore.wait_timeout(SystemTicks::from_ticks(25)),
        SemaphoreWaitOutcome::Signaled
    );
    assert_eq!(
        semaphore.wait_timeout_duration(SystemDuration::new(SystemTicks::from_ticks(25))),
        SemaphoreWaitOutcome::Signaled
    );
    semaphore.signal();
    semaphore.reset();

    assert_eq!(firmware.semaphore_wait_count(), 1);
    assert_eq!(firmware.semaphore_timed_wait_ticks(), Some(25));
    assert_eq!(firmware.semaphore_signal_count(), 1);
    assert_eq!(firmware.semaphore_reset_count(), 1);
    drop(semaphore);
    assert_eq!(firmware.semaphore_free_count(), 1);
}

#[test]
fn semaphore_wait_outcomes_have_named_predicates() {
    assert!(SemaphoreWaitOutcome::Signaled.is_signaled());
    assert!(!SemaphoreWaitOutcome::Signaled.is_timed_out());
    assert!(!SemaphoreWaitOutcome::TimedOut.is_signaled());
    assert!(SemaphoreWaitOutcome::TimedOut.is_timed_out());
}

#[test]
fn synchronization_creation_and_timed_wait_failures_are_reported() {
    let firmware = FirmwareTest::new();
    firmware.fail_mutex_creation();
    firmware.fail_semaphore_creation();
    assert!(FirmwareMutex::new().is_none());
    assert!(FirmwareSemaphore::new().is_none());

    drop(firmware);
    let firmware = FirmwareTest::new();
    let semaphore = FirmwareSemaphore::new().expect("fake firmware creates a semaphore");
    firmware.fail_semaphore_timeout();
    assert_eq!(
        semaphore.wait_timeout(SystemTicks::from_ticks(5)),
        SemaphoreWaitOutcome::TimedOut
    );
}
