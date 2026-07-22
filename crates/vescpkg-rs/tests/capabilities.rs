#![cfg(feature = "test-support")]

//! Public capability constructors stay independent of raw slot names.

use vescpkg_rs::{FirmwareCapabilities, VescIfPresence};
use vescpkg_rs_sys::VescIfAbi;

#[test]
fn package_callers_can_branch_on_subsystem_handles() {
    let words = [1_usize; VescIfAbi::FIELD_COUNT];
    let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

    assert!(capabilities.can_bus().is_ok());
    assert!(capabilities.nvm().is_ok());
    assert!(capabilities.audio().is_ok());
    assert!(capabilities.uart().is_ok());
    assert!(capabilities.settings().is_ok());
}
