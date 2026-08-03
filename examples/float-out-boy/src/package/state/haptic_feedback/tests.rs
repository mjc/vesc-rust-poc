use super::*;
use crate::config::FloatOutBoyConfigImage;
use vescpkg_rs::prelude::Ratio;
use vescpkg_rs::test_support::FirmwareTest;

fn duty_input() -> HapticFeedbackInput {
    HapticFeedbackInput {
        run_state: FloatOutBoyRunState::Running,
        mode: FloatOutBoyMode::Normal,
        setpoint_adjustment: FloatOutBoySetpointAdjustment::PushbackDuty,
        duty_cycle: Ratio::from_ratio_const(0.81),
        duty_solid_threshold: Ratio::from_ratio_const(0.85),
        speed: Speed::ZERO,
        current_saturation: Ratio::from_ratio_const(0.0),
        fatal_error: false,
    }
}

#[test]
fn duty_pushback_starts_the_scaled_warning_tone_like_float_out_boy() {
    let firmware = FirmwareTest::new();
    let mut haptic = HapticFeedbackState::new();
    let mut motor_control = FloatOutBoyMotorControl::new();

    haptic.update(
        FloatOutBoyConfigImage::defaults().haptic(),
        duty_input(),
        firmware.motor(),
        &mut motor_control,
        TimestampTicks::from_ticks(0),
        SampleRate::from_hertz(832.0),
    );

    assert_eq!(firmware.foc_tone_command_count(), 1);
    assert_f32_eq!(
        firmware
            .commanded_foc_tone_frequency()
            .frequency()
            .as_hertz(),
        495.0
    );
    assert_f32_eq!(
        firmware.commanded_foc_tone_voltage().voltage().as_volts(),
        0.6
    );
}

#[test]
fn generated_haptic_defaults_decode_at_the_float_out_boy_offsets() {
    let defaults = FloatOutBoyConfigImage::defaults();
    let config = defaults.haptic();

    assert_f32_eq!(config.duty_frequency().frequency().as_hertz(), 495.0);
    assert_f32_eq!(config.duty_strength().voltage().as_volts(), 3.0);
    assert_f32_eq!(config.error_frequency().frequency().as_hertz(), 550.0);
    assert_f32_eq!(config.error_strength().voltage().as_volts(), 3.0);
    assert_f32_eq!(config.vibrate_frequency().frequency().as_hertz(), 70.0);
    assert_f32_eq!(config.vibrate_strength().current().as_amps(), 0.0);
    assert_f32_eq!(config.duty_solid_offset().as_ratio(), 0.05);
    assert_f32_eq!(config.current_threshold().as_ratio(), 0.0);
    assert_f32_eq!(config.min_strength().as_ratio(), 0.2);
    assert!((config.max_strength_speed().as_kilometers_per_hour() - 30.0).abs() < 0.0001);
    assert_f32_eq!(config.strength_curvature().as_ratio(), 0.6);
}

#[test]
fn haptic_strength_scaling_uses_speed_magnitude_in_reverse() {
    let image = FloatOutBoyConfigImage::defaults();
    let config = image.haptic();
    let forward = strength_scale(config, Speed::from_meters_per_second(5.0));
    let reverse = strength_scale(config, Speed::from_meters_per_second(-5.0));

    assert_f32_eq!(reverse, forward);
}

#[test]
fn warning_pattern_stops_on_the_odd_beat_and_restarts_on_the_next_even_beat() {
    let firmware = FirmwareTest::new();
    let config = FloatOutBoyConfigImage::defaults();
    let mut haptic = HapticFeedbackState::new();
    let mut motor_control = FloatOutBoyMotorControl::new();
    for tick in [0, TONE_LENGTH_TICKS, TONE_LENGTH_TICKS * 2] {
        haptic.update(
            config.haptic(),
            duty_input(),
            firmware.motor(),
            &mut motor_control,
            TimestampTicks::from_ticks(tick),
            SampleRate::from_hertz(832.0),
        );
    }

    assert_eq!(firmware.foc_tone_command_count(), 3);
    assert_f32_eq!(
        firmware.commanded_foc_tone_voltage().voltage().as_volts(),
        0.6
    );
}

#[test]
fn patterned_haptic_periods_match_refloat() {
    for (feedback_type, beats, cycle_ticks) in [
        (HapticFeedbackType::DutySpeed, 2, 2_000),
        (HapticFeedbackType::ErrorTemperature, 6, 6_000),
        (HapticFeedbackType::ErrorVoltage, 8, 8_000),
        (HapticFeedbackType::ErrorFatal, 10, 10_000),
    ] {
        assert_eq!(feedback_type.beats(), beats);
        assert_eq!(TONE_LENGTH_TICKS.checked_mul(beats), Some(cycle_ticks));
    }
    assert_eq!(HapticFeedbackType::DutyContinuous.beats(), 0);
    assert_eq!(HapticFeedbackType::None.beats(), 0);
}

#[test]
fn fatal_alert_uses_the_error_tone_before_pushback_selection() {
    let firmware = FirmwareTest::new();
    let mut haptic = HapticFeedbackState::new();
    let mut motor_control = FloatOutBoyMotorControl::new();
    let mut input = duty_input();
    input.fatal_error = true;

    haptic.update(
        FloatOutBoyConfigImage::defaults().haptic(),
        input,
        firmware.motor(),
        &mut motor_control,
        TimestampTicks::from_ticks(0),
        SampleRate::from_hertz(832.0),
    );

    assert_f32_eq!(
        firmware
            .commanded_foc_tone_frequency()
            .frequency()
            .as_hertz(),
        550.0
    );
}

#[test]
fn handtest_stops_an_active_haptic_tone() {
    let firmware = FirmwareTest::new();
    let config = FloatOutBoyConfigImage::defaults();
    let mut haptic = HapticFeedbackState::new();
    let mut motor_control = FloatOutBoyMotorControl::new();
    haptic.update(
        config.haptic(),
        duty_input(),
        firmware.motor(),
        &mut motor_control,
        TimestampTicks::from_ticks(0),
        SampleRate::from_hertz(832.0),
    );
    let mut handtest = duty_input();
    handtest.mode = FloatOutBoyMode::HandTest;
    haptic.update(
        config.haptic(),
        handtest,
        firmware.motor(),
        &mut motor_control,
        TimestampTicks::from_ticks(1),
        SampleRate::from_hertz(832.0),
    );

    assert_eq!(firmware.foc_tone_command_count(), 2);
    assert_f32_eq!(
        firmware.commanded_foc_tone_voltage().voltage().as_volts(),
        0.0
    );
}

#[test]
fn configured_current_saturation_starts_the_continuous_warning() {
    let firmware = FirmwareTest::new();
    let mut config = FloatOutBoyConfigImage::defaults();
    assert!(config.set_haptic_current_threshold(Ratio::from_ratio_const(0.8)));
    let mut haptic = HapticFeedbackState::new();
    let mut motor_control = FloatOutBoyMotorControl::new();
    let mut input = duty_input();
    input.setpoint_adjustment = FloatOutBoySetpointAdjustment::None;
    input.current_saturation = Ratio::from_ratio_const(0.81);

    haptic.update(
        config.haptic(),
        input,
        firmware.motor(),
        &mut motor_control,
        TimestampTicks::from_ticks(0),
        SampleRate::from_hertz(832.0),
    );

    assert_eq!(firmware.foc_tone_command_count(), 1);
}

#[test]
fn negative_regen_limit_produces_refloat_battery_saturation() {
    let saturation = super::normalized_current_saturation(
        vescpkg_rs::Current::from_amps(-10.0),
        vescpkg_rs::Current::from_amps(-20.0),
    );

    assert!((saturation - 0.5).abs() < f32::EPSILON);
}
