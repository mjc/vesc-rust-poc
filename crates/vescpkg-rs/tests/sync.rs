#![cfg(feature = "test-support")]

//! Integration tests for RAII firmware synchronization.

use vescpkg_rs::FirmwareMutex;
use vescpkg_rs::test_support::FirmwareTest;

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
