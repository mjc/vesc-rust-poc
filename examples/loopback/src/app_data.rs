//! Loopback app-data callback registration for the example package.

#![cfg(all(not(test), target_arch = "arm"))]

use vescpkg_rs::{LoopbackLifecycle, RealBindings};

/// Register the package's loopback callback with the firmware app-data slot.
#[inline(always)]
pub(crate) fn register() -> bool {
    LoopbackLifecycle::new(RealBindings)
        .register_app_data_handler(vescpkg_rs::ble_loopback::loopback_handle_app_data)
        .is_ok()
}
