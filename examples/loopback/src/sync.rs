//! Usage-shaped RAII synchronization for package code.

use vescpkg_rs::FirmwareMutex;

/// Run a package operation while holding a firmware-owned mutex.
pub fn with_probe_lock<R>(operation: impl FnOnce() -> R) -> Option<R> {
    let mutex = FirmwareMutex::new()?;
    let _guard = mutex.lock();
    Some(operation())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::with_probe_lock;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn package_sync_operation_uses_a_scoped_firmware_mutex() {
        let _firmware = FirmwareTest::new();
        assert_eq!(with_probe_lock(|| 42), Some(42));
    }
}
