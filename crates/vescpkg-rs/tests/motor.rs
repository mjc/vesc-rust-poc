//! Integration tests for typed motor telemetry.
#![cfg(feature = "test-support")]

use vescpkg_rs::prelude::{
    AudioChannel, AudioFrequency, AudioVoltage, Current, FirmwareFaultCode, Frequency,
    InputCurrentLimit, Voltage,
};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{MotorOutput, MotorTelemetry};

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
