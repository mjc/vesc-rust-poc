#![cfg(feature = "test-support")]
#![allow(clippy::redundant_closure_for_method_calls)]

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
    assert!(capabilities.advanced_foc().is_ok());
    assert!(capabilities.motor().is_ok());
    assert!(capabilities.motor_telemetry().is_ok());
}

#[test]
fn package_advanced_foc_construction_reports_a_missing_slot() {
    let mut words = vec![1_usize; VescIfAbi::FIELD_COUNT];
    words[VescIfAbi::FOC_SET_OPENLOOP_DUTY.slot_index()] = 0;
    let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

    assert_eq!(
        capabilities.advanced_foc().err().map(|error| error.slot()),
        Some(VescIfAbi::FOC_SET_OPENLOOP_DUTY)
    );
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

#[test]
fn package_motor_construction_reports_a_missing_required_slot() {
    let mut words = vec![1_usize; VescIfAbi::FIELD_COUNT];
    words[VescIfAbi::MC_SET_CURRENT.slot_index()] = 0;
    let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

    assert_eq!(
        capabilities.motor().err().map(|error| error.slot()),
        Some(VescIfAbi::MC_SET_CURRENT)
    );
    assert_eq!(
        capabilities
            .motor_telemetry()
            .err()
            .map(|error| error.slot()),
        Some(VescIfAbi::MC_SET_CURRENT)
    );

    words[VescIfAbi::MC_SET_CURRENT.slot_index()] = 1;
    words[VescIfAbi::MC_SELECT_MOTOR_THREAD.slot_index()] = 0;
    let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));
    assert_eq!(
        capabilities.motor().err().map(|error| error.slot()),
        Some(VescIfAbi::MC_SELECT_MOTOR_THREAD)
    );
}
