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
    assert!(capabilities.inputs().is_ok());
    assert!(capabilities.require_inputs().is_ok());
    assert!(capabilities.audio().is_ok());
    assert!(capabilities.uart().is_ok());
    assert!(capabilities.settings().is_ok());
    assert!(capabilities.imu().is_ok());
}

#[test]
fn package_imu_construction_reports_a_missing_required_slot() {
    let mut words = vec![1_usize; VescIfAbi::FIELD_COUNT];
    words[VescIfAbi::IMU_GET_MAG.slot_index()] = 0;
    let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

    assert_eq!(
        capabilities.imu().err().map(|error| error.slot()),
        Some(VescIfAbi::IMU_GET_MAG)
    );
}
