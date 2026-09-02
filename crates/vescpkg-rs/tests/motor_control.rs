#![cfg(feature = "test-support")]
#![allow(clippy::float_cmp)]
//! Integration coverage for shared package motor-control state.

use vescpkg_rs::prelude::{
    AudioFrequency, Current, MotorCurrent, Rpm, SampleRate, TimestampTicks, VescSeconds,
};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{MotorControl, MotorControlRunState, ParkingBrakeMode};

#[test]
fn parking_brake_mode_preserves_unknown_firmware_values() {
    assert_eq!(u8::from(ParkingBrakeMode::from(0xff)), 0xff);
}

#[test]
fn shared_motor_control_owns_requested_current_and_idle_parking_brake() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();

    control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
    assert!(control.apply_requested_current(firmware.motor()));
    assert_eq!(firmware.keep_alive_count(), 1);
    assert_eq!(firmware.current_off_delay_count(), 1);
    assert_eq!(
        firmware.commanded_current_off_delay().duration(),
        VescSeconds::from_seconds(0.05)
    );
    assert_eq!(firmware.commanded_current().current().as_amps(), 5.0);

    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Idle,
        Rpm::ZERO,
        TimestampTicks::from_ticks(20_000),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.duty_command_count(), 1);
}

#[test]
fn shared_motor_control_sets_zero_once_while_disabled() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();

    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Disabled,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.current_command_count(), 1);
    assert_eq!(firmware.commanded_current().current().as_amps(), 0.0);
    assert!(!control.apply(
        firmware.motor(),
        MotorControlRunState::Disabled,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.current_command_count(), 1);
}

#[test]
fn shared_motor_control_seeds_idle_brake_timer_on_activation() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();

    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Idle,
        Rpm::ZERO,
        TimestampTicks::from_ticks(20_000),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.keep_alive_count(), 1);
    assert_eq!(firmware.duty_command_count(), 1);
    assert_eq!(firmware.current_command_count(), 0);
    assert_eq!(firmware.brake_current_command_count(), 0);
}

#[test]
fn shared_motor_control_modulates_requested_current_for_tones() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();
    control.play_tone(
        AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(70.0)),
        MotorCurrent::new(Current::from_amps(2.0)),
        SampleRate::from_hertz(832.0),
    );

    for _ in 0..4 {
        control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
        assert!(control.apply(
            firmware.motor(),
            MotorControlRunState::Running,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            ParkingBrakeMode::IDLE,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(firmware.commanded_current().current().as_amps(), 3.0);
    }

    control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.commanded_current().current().as_amps(), 7.0);

    control.stop_tone();
    control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(firmware.commanded_current().current().as_amps(), 5.0);
}

#[test]
fn shared_motor_control_saturates_an_empty_tone_counter() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();
    control.set_tone_phase_for_test(1, 0);

    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::IDLE,
        MotorCurrent::new(Current::ZERO),
    ));
    assert_eq!(control.tone_counter_for_test(), 1);
}

#[test]
fn shared_motor_control_keeps_high_frequency_tones_alive() {
    let firmware = FirmwareTest::new();
    let mut control = MotorControl::default();
    control.play_tone(
        AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(1_000.0)),
        MotorCurrent::new(Current::from_amps(2.0)),
        SampleRate::from_hertz(500.0),
    );

    control.request_current(MotorCurrent::new(Current::ZERO));
    assert!(control.apply(
        firmware.motor(),
        MotorControlRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        ParkingBrakeMode::NEVER,
        MotorCurrent::new(Current::ZERO),
    ));
    assert_eq!(firmware.commanded_current().current().as_amps(), 2.0);
}
