#![cfg(feature = "test-support")]

use vescpkg_rs::{CanBus, CanControllerId, CanError, CanExtendedId, CanStandardId};

#[test]
fn can_bus_transmits_bounded_payloads_and_copies_status() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let bus: &CanBus = firmware.can();
    let standard = CanStandardId::try_new(0x123).expect("valid standard id");
    let extended = CanExtendedId::try_new(0x12_3456).expect("valid extended id");

    bus.transmit_standard(standard, &[1, 2, 3])
        .expect("standard transmit");
    bus.transmit_extended(extended, &[4, 5])
        .expect("extended transmit");
    assert_eq!(bus.transmit_standard(standard, &[0; 9]), Err(CanError::PayloadTooLong));

    let status = bus.status(CanControllerId::new(7)).expect("status snapshot");
    assert_eq!(status.controller().as_u8(), 7);
    assert_eq!(status.electrical_speed().rpm().as_revolutions_per_minute(), 1200.0);
    assert_eq!(status.motor_current().current().as_amps(), 4.5);
    assert_eq!(status.duty_cycle().ratio().as_ratio(), 0.25);
}
