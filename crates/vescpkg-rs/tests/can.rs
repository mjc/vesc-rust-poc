#![cfg(feature = "test-support")]
//! Integration coverage for the safe CAN facade.

use vescpkg_rs::{
    AngleDegrees, CanBus, CanControllerId, CanError, CanExtendedId, CanHardwareType, CanStandardId,
    Current, CurrentRelative, DutyCycle, ElectricalSpeed, MotorCurrent, PidPosition, Rpm,
    SignedRatio,
};

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
    bus.set_current(
        CanControllerId::new(7),
        MotorCurrent::new(Current::from_amps(3.0)),
    )
    .expect("remote current command");
    bus.set_duty(
        CanControllerId::new(7),
        DutyCycle::new(SignedRatio::from_ratio_const(0.5)),
    )
    .expect("remote duty command");
    bus.set_rpm(
        CanControllerId::new(7),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0)),
    )
    .expect("remote speed command");
    bus.set_position(
        CanControllerId::new(7),
        PidPosition::new(AngleDegrees::from_degrees(12.0)),
    )
    .expect("remote position command");
    bus.set_current_relative(
        CanControllerId::new(7),
        CurrentRelative::new(SignedRatio::from_ratio_const(-0.25)),
    )
    .expect("remote relative current command");
    assert_eq!(
        bus.transmit_standard(standard, &[0; 9]),
        Err(CanError::PayloadTooLong)
    );

    let status = bus
        .status(CanControllerId::new(7))
        .expect("status snapshot");
    assert_eq!(status.controller().as_u8(), 7);
    assert_eq!(
        status.electrical_speed().rpm().as_revolutions_per_minute(),
        1200.0
    );
    assert_eq!(status.motor_current().current().as_amps(), 4.5);
    assert_eq!(status.duty_cycle().ratio().as_ratio(), 0.25);
}

#[test]
fn can_bus_pings_and_reports_remote_hardware_type() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();

    assert_eq!(
        firmware
            .can()
            .ping(CanControllerId::new(7))
            .expect("CAN ping slot"),
        CanHardwareType::Vesc
    );
}

#[test]
fn can_bus_copies_status_message_two() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status2(CanControllerId::new(7))
        .expect("CAN status message 2");

    assert_eq!(status.amp_hours_discharged().charge().as_amp_hours(), 1.25);
    assert_eq!(status.amp_hours_charged().charge().as_amp_hours(), 2.5);
}
