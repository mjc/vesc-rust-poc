#![cfg(feature = "test-support")]
//! Integration coverage for typed motor handbrake commands.

use vescpkg_rs::{
    Current, HandbrakeCurrent, HandbrakeRelative, MotorOutput, MotorTelemetry, Ratio, VescSeconds,
};

#[test]
fn motor_exposes_typed_handbrake_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware
        .motor()
        .set_handbrake(HandbrakeCurrent::new(Current::from_amps(2.0)));
    firmware
        .motor()
        .set_handbrake_relative(HandbrakeRelative::new(Ratio::from_ratio_const(0.25)));

    let telemetry = firmware.telemetry();
    assert_eq!(
        telemetry.motor_current_unfiltered().current().as_amps(),
        12.0
    );
    assert_eq!(
        telemetry
            .directional_motor_current_unfiltered()
            .current()
            .as_amps(),
        -12.5
    );
    assert_eq!(
        telemetry.battery_current_unfiltered().current().as_amps(),
        8.0
    );
    assert_eq!(telemetry.average_power().power().as_watts(), 120.0);
    assert_eq!(telemetry.peak_power().power().as_watts(), 240.0);
    assert_eq!(
        telemetry.average_speed().speed().as_meters_per_second(),
        4.0
    );
    assert_eq!(telemetry.peak_speed().speed().as_meters_per_second(), 8.0);
    assert_eq!(
        telemetry.average_motor_current().current().as_amps(),
        6.0
    );
    assert_eq!(telemetry.peak_motor_current().current().as_amps(), 18.0);
    assert_eq!(telemetry.tachometer(false).steps().as_steps(), 1234);
    assert_eq!(telemetry.absolute_tachometer(true).steps().as_steps(), 5678);
    assert_eq!(telemetry.sampling_frequency().as_hertz(), 20_000.0);
    firmware.motor().release_motor();
    assert!(
        firmware
            .motor()
            .wait_for_motor_release(VescSeconds::from_seconds(0.1))
    );
    firmware.motor().reset_statistics();
}
