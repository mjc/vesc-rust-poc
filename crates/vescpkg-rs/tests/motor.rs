#![cfg(feature = "test-support")]
//! Integration tests for typed motor telemetry.

use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::FirmwareFaultCode;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn firmware_fault_name_trims_the_vesc_prefix_without_allocating() {
    let firmware = FirmwareTest::new().with_firmware_fault(FirmwareFaultCode::from_wire_code(5));

    assert_eq!(
        firmware
            .telemetry()
            .firmware_fault_name(FirmwareFaultCode::from_wire_code(5)),
        Some(b"OVER_TEMP_FET".as_slice()),
    );
}
