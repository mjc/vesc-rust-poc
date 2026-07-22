#![cfg(feature = "test-support")]
//! Integration coverage for the safe CAN facade.

use vescpkg_rs::{
    AngleDegrees, CanBus, CanControllerId, CanError, CanExtendedId, CanHardwareType,
    CanReceiverGuard, CanReceiverHandler, CanReceiverId, CanStandardId, CanStatusStore, Current,
    CurrentRelative, DutyCycle, ElectricalSpeed, MotorCurrent, PackageRuntimeState,
    PackageStateStore, PidPosition, Rpm, SignedRatio,
};

struct ReceiverHandler;

impl CanReceiverHandler for ReceiverHandler {
    fn receive(_id: CanReceiverId, _payload: &[u8]) -> bool {
        true
    }
}

struct PackageState {
    _guard: Option<CanReceiverGuard>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

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

    let remote = bus.remote_motor(CanControllerId::new(7));
    assert_eq!(remote.controller().as_u8(), 7);
    remote
        .set_current(MotorCurrent::new(Current::from_amps(1.0)))
        .expect("scoped remote current command");
    assert!(remote.status().is_some());
}

#[test]
fn can_transmit_reports_absent_optional_slot() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.set_can_available(false);
    let standard = CanStandardId::try_new(0x123).expect("valid standard id");

    assert_eq!(
        firmware.can().transmit_standard(standard, &[1, 2, 3]),
        Err(CanError::Unsupported)
    );
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

#[test]
fn can_bus_copies_status_message_three() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status3(CanControllerId::new(7))
        .expect("CAN status message 3");

    assert_eq!(
        status.watt_hours_discharged().energy().as_watt_hours(),
        10.0
    );
    assert_eq!(status.watt_hours_charged().energy().as_watt_hours(), 4.0);
}

#[test]
fn can_bus_copies_status_message_four() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status4(CanControllerId::new(7))
        .expect("CAN status message 4");

    assert_eq!(
        status.fet_temperature().temperature().as_degrees_celsius(),
        45.0
    );
    assert_eq!(
        status
            .motor_temperature()
            .temperature()
            .as_degrees_celsius(),
        50.0
    );
    assert_eq!(status.input_current().current().as_amps(), 3.0);
    assert_eq!(status.position().angle().as_degrees(), 12.0);
}

#[test]
fn can_bus_copies_status_message_five() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status5(CanControllerId::new(7))
        .expect("CAN status message 5");

    assert_eq!(status.input_voltage().voltage().as_volts(), 48.0);
    assert_eq!(status.tachometer().steps().as_steps(), 1234);
}

#[test]
fn can_bus_copies_status_message_six() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status6(CanControllerId::new(7))
        .expect("CAN status message 6");

    assert_eq!(status.adc1().voltage().as_volts(), 1.0);
    assert_eq!(status.adc2().voltage().as_volts(), 2.0);
    assert_eq!(status.adc3().voltage().as_volts(), 3.0);
    assert_eq!(status.ppm().ratio().as_ratio(), 0.5);
    assert_eq!(
        status
            .age_at(vescpkg_rs::TimestampTicks::from_ticks(130))
            .as_ticks(),
        7
    );
}

#[test]
fn can_status_rejects_invalid_duty_ratio() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.set_can_status_duty(2.0);

    assert!(firmware.can().status(CanControllerId::new(7)).is_none());
}

#[test]
fn can_status6_rejects_invalid_ppm_ratio() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.set_can_status_ppm(2.0);

    assert!(firmware.can().status6(CanControllerId::new(7)).is_none());
}

#[test]
fn can_status_reports_wrapping_age_and_staleness() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let status = firmware
        .can()
        .status(CanControllerId::new(7))
        .expect("CAN status");
    let now = vescpkg_rs::TimestampTicks::from_ticks(130);

    assert_eq!(status.age_at(now).as_ticks(), 7);
    assert!(status.is_stale(now, vescpkg_rs::SystemTicks::from_ticks(5)));
}

#[test]
fn can_status_store_copies_available_messages_and_can_ping() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let mut store = CanStatusStore::new(CanControllerId::new(7));

    assert_eq!(store.ping(firmware.can()), Ok(CanHardwareType::Vesc));
    assert_eq!(store.refresh(firmware.can()), 6);
    assert!(store.status().is_some());
    assert!(store.status6().is_some());
    assert_eq!(store.controller().as_u8(), 7);
}

#[test]
fn package_stop_releases_standard_can_receiver_before_next_registration() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let guard = firmware
        .can()
        .register_standard_receiver::<ReceiverHandler>()
        .expect("standard CAN receiver");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _guard: Some(guard),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    firmware
        .can()
        .register_standard_receiver::<ReceiverHandler>()
        .expect("stop released standard CAN receiver");
}

#[test]
fn package_stop_releases_extended_can_receiver_before_next_registration() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let guard = firmware
        .can()
        .register_extended_receiver::<ReceiverHandler>()
        .expect("extended CAN receiver");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _guard: Some(guard),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    firmware
        .can()
        .register_extended_receiver::<ReceiverHandler>()
        .expect("stop released extended CAN receiver");
}

#[test]
fn can_bus_sends_brake_and_off_delay_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let brake = vescpkg_rs::BrakeCurrent::new(vescpkg_rs::Current::from_amps(6.0));
    let relative = vescpkg_rs::BrakeCurrentRelative::new(
        vescpkg_rs::Ratio::from_ratio(0.25).expect("valid brake ratio"),
    );
    let delay = vescpkg_rs::CurrentOffDelay::new(vescpkg_rs::VescSeconds::from_seconds(0.5));

    firmware
        .can()
        .set_brake_current(vescpkg_rs::CanControllerId::new(7), brake)
        .expect("brake command");
    firmware
        .can()
        .set_brake_current_relative(vescpkg_rs::CanControllerId::new(7), relative)
        .expect("relative brake command");
    firmware
        .can()
        .set_current_off_delay(
            vescpkg_rs::CanControllerId::new(7),
            vescpkg_rs::MotorCurrent::new(vescpkg_rs::Current::from_amps(4.0)),
            delay,
        )
        .expect("off-delay command");
}

#[test]
fn can_bus_sends_relative_current_off_delay() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware
        .can()
        .set_current_relative_off_delay(
            CanControllerId::new(7),
            CurrentRelative::new(SignedRatio::from_ratio_const(-0.25)),
            vescpkg_rs::CurrentOffDelay::new(vescpkg_rs::VescSeconds::from_seconds(0.25)),
        )
        .expect("relative off-delay command");
}

struct TestReceiver;

impl CanReceiverHandler for TestReceiver {
    fn receive(_id: CanReceiverId, _payload: &[u8]) -> bool {
        true
    }
}

#[test]
fn can_receiver_registration_is_exclusive_and_released_on_drop() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let bus = firmware.can();
    let registration = bus
        .register_standard_receiver::<TestReceiver>()
        .expect("SID receiver registration");

    assert!(matches!(
        bus.register_standard_receiver::<TestReceiver>(),
        Err(CanError::ReceiverBusy)
    ));
    drop(registration);
    assert!(bus.register_standard_receiver::<TestReceiver>().is_ok());
}

#[test]
fn can_extended_receiver_registration_is_exclusive() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let bus = firmware.can();
    let registration = bus
        .register_extended_receiver::<TestReceiver>()
        .expect("EID receiver registration");

    assert!(matches!(
        bus.register_extended_receiver::<TestReceiver>(),
        Err(CanError::ReceiverBusy)
    ));
    drop(registration);
    assert!(bus.register_extended_receiver::<TestReceiver>().is_ok());
}
