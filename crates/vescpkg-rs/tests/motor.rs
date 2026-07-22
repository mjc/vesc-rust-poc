//! Integration tests for typed motor telemetry.
#![cfg(feature = "test-support")]

use vescpkg_rs::prelude::{
    AngleDegrees, AudioChannel, AudioFrequency, AudioVoltage, Current, FirmwareFaultCode,
    Frequency, HandbrakeCurrent, HandbrakeRelative, InputCurrentLimit, OdometerMeters, PidPosition,
    DutyCycle, MotorSelection, OpenLoopCurrent, OpenLoopPhase, PidPosition, Ratio, Rpm,
    SignedRatio, ElectricalSpeed, VescSeconds, Voltage,
};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{MotorOutput, MotorTelemetry};

const MAX_WIRE_FAULT: FirmwareFaultCode = FirmwareFaultCode::from_wire_code(u8::MAX);

#[test]
fn firmware_fault_code_can_be_built_at_compile_time() {
    assert!(!MAX_WIRE_FAULT.is_none());
}

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

#[test]
fn foc_haptic_tone_uses_typed_audio_values() {
    let firmware = FirmwareTest::new();

    assert!(firmware.motor().play_foc_tone(
        AudioChannel::try_new(0).unwrap(),
        AudioFrequency::new(Frequency::from_hertz(440.0)),
        AudioVoltage::new(Voltage::from_volts(0.25)),
    ));
    assert_eq!(firmware.foc_tone_command_count(), 1);
    assert_eq!(
        firmware.commanded_foc_tone_channel(),
        AudioChannel::try_new(0).ok()
    );
    assert_eq!(
        firmware.commanded_foc_tone_frequency(),
        AudioFrequency::new(Frequency::from_hertz(440.0))
    );
    assert_eq!(
        firmware.commanded_foc_tone_voltage(),
        AudioVoltage::new(Voltage::from_volts(0.25))
    );
}

#[test]
fn typed_audio_frequency_exposes_hertz_without_erasing_its_domain() {
    let frequency = AudioFrequency::new(Frequency::from_hertz(440.0));

    assert!((frequency.as_hertz() - 440.0).abs() < f32::EPSILON);
}

#[test]
fn input_current_limits_preserve_positive_magnitudes_for_haptic_saturation() {
    let firmware = FirmwareTest::new().with_input_current_limits(
        InputCurrentLimit::new(Current::from_amps(30.0)),
        InputCurrentLimit::new(Current::from_amps(15.0)),
    );

    assert_eq!(
        firmware.telemetry().drive_input_current_limit(),
        InputCurrentLimit::new(Current::from_amps(30.0))
    );
    assert_eq!(
        firmware.telemetry().brake_input_current_limit(),
        InputCurrentLimit::new(Current::from_amps(15.0))
    );
}

unsafe extern "C" fn test_pwm_callback() {}

#[test]
fn motor_exposes_typed_handbrake_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new()
        .with_d_axis_current(Some(DCurrent::new(Current::from_amps(1.5))));
    firmware
        .motor()
        .set_handbrake(HandbrakeCurrent::new(Current::from_amps(2.0)));
    firmware.motor().set_handbrake_relative(
        HandbrakeRelative::new(Ratio::from_ratio_const(0.25)),
    );
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
