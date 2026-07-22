#![cfg(feature = "test-support")]
//! Integration coverage for typed motor handbrake commands.

use vescpkg_rs::{
    AngleDegrees, Current, DCurrent, DutyCycle, ElectricalSpeed, HandbrakeCurrent,
    HandbrakeRelative, MotorOutput, MotorSelection, MotorTelemetry, OdometerMeters,
    OpenLoopCurrent, OpenLoopPhase, PidPosition, Ratio, Rpm, SignedRatio, VescSeconds,
};

unsafe extern "C" fn test_pwm_callback() {}

#[test]
fn motor_exposes_typed_handbrake_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new()
        .with_d_axis_current(Some(DCurrent::new(Current::from_amps(1.5))));
    firmware
        .motor()
        .set_handbrake(HandbrakeCurrent::new(Current::from_amps(2.0)));
    firmware
        .motor()
        .set_handbrake_relative(HandbrakeRelative::new(Ratio::from_ratio_const(0.25)));

    let telemetry = firmware.telemetry();
    assert!(firmware.motor().dc_calibration_done());
    let pwm_lease = unsafe {
        firmware
            .motor()
            .register_pwm_callback(test_pwm_callback)
            .unwrap()
    };
    drop(pwm_lease);
    assert_eq!(telemetry.firmware_fault_description(), Some("TEST_FAULT"));
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
    assert_eq!(telemetry.average_motor_current().current().as_amps(), 6.0);
    assert_eq!(telemetry.peak_motor_current().current().as_amps(), 18.0);
    assert_eq!(
        telemetry
            .average_mosfet_temperature()
            .temperature()
            .as_degrees_celsius(),
        45.0
    );
    assert_eq!(
        telemetry
            .peak_mosfet_temperature()
            .temperature()
            .as_degrees_celsius(),
        60.0
    );
    assert_eq!(
        telemetry
            .average_motor_temperature()
            .temperature()
            .as_degrees_celsius(),
        40.0
    );
    assert_eq!(
        telemetry
            .peak_motor_temperature()
            .temperature()
            .as_degrees_celsius(),
        55.0
    );
    assert_eq!(
        telemetry.statistics_count_time().duration().as_seconds(),
        90.0
    );
    assert_eq!(
        telemetry.signed_trip_distance().distance().as_meters(),
        -3.5
    );
    assert_eq!(telemetry.pid_position_setpoint().angle().as_degrees(), 42.0);
    assert_eq!(telemetry.pid_position().angle().as_degrees(), 12.0);
    assert_eq!(telemetry.d_axis_current().unwrap().current().as_amps(), 1.5);
    assert_eq!(telemetry.q_axis_current().unwrap().current().as_amps(), 2.5);
    assert_eq!(
        telemetry.d_axis_voltage().unwrap().voltage().as_volts(),
        3.5
    );
    assert_eq!(
        telemetry.q_axis_voltage().unwrap().voltage().as_volts(),
        4.5
    );
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
    firmware
        .motor()
        .update_pid_position_offset(PidPosition::new(AngleDegrees::from_degrees(5.0)), true);
    firmware
        .motor()
        .set_odometer(OdometerMeters::from_meters(12_345));
    firmware
        .motor()
        .set_pid_speed(ElectricalSpeed::new(Rpm::from_revolutions_per_minute(
            1500.0,
        )));
    firmware
        .motor()
        .set_pid_position(PidPosition::new(AngleDegrees::from_degrees(90.0)));
    firmware.motor().select_motor(MotorSelection::new(1));
    firmware
        .motor()
        .set_duty_cycle_without_ramping(DutyCycle::new(SignedRatio::from_ratio_const(0.2)));
    let advanced = firmware.advanced_foc();
    unsafe {
        advanced
            .set_open_loop_current(
                OpenLoopCurrent::new(Current::from_amps(3.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(300.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_phase(
                OpenLoopCurrent::new(Current::from_amps(2.0)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(45.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_duty(
                DutyCycle::new(SignedRatio::from_ratio_const(0.1)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(200.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_duty_phase(
                DutyCycle::new(SignedRatio::from_ratio_const(0.15)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(90.0)),
            )
            .unwrap();
    }
}
