use crate::config::FloatOutBoyHapticConfig;
use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState, FloatOutBoySetpointAdjustment};
use crate::motor_control::FloatOutBoyMotorControl;
use vescpkg_rs::MotorOutput;
use vescpkg_rs::prelude::{
    AudioChannel, AudioFrequency, AudioVoltage, Current, MotorCurrent, Ratio, SYSTEM_TICK_RATE_HZ,
    SampleRate, Speed, TimestampTicks, Voltage,
};

const TONE_LENGTH_TICKS: u32 = SYSTEM_TICK_RATE_HZ as u32 / 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HapticFeedbackInput {
    pub(super) run_state: FloatOutBoyRunState,
    pub(super) mode: FloatOutBoyMode,
    pub(super) setpoint_adjustment: FloatOutBoySetpointAdjustment,
    pub(super) duty_cycle: Ratio,
    pub(super) duty_solid_threshold: Ratio,
    pub(super) speed: Speed,
    pub(super) current_saturation: Ratio,
    pub(super) fatal_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HapticFeedbackType {
    None,
    DutySpeed,
    DutyContinuous,
    ErrorTemperature,
    ErrorVoltage,
    ErrorFatal,
}

impl HapticFeedbackType {
    const fn beats(self) -> u32 {
        match self {
            Self::DutySpeed => 2,
            Self::DutyContinuous | Self::None => 0,
            Self::ErrorTemperature => 6,
            Self::ErrorVoltage => 8,
            Self::ErrorFatal => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HapticFeedbackState {
    type_playing: HapticFeedbackType,
    tone_timer: TimestampTicks,
    is_playing: bool,
    can_change_type: bool,
}

impl HapticFeedbackState {
    pub(super) const fn new() -> Self {
        Self {
            type_playing: HapticFeedbackType::None,
            tone_timer: TimestampTicks::from_ticks(0),
            is_playing: false,
            can_change_type: true,
        }
    }

    pub(super) fn update(
        &mut self,
        config: FloatOutBoyHapticConfig<'_>,
        input: HapticFeedbackInput,
        motor: &impl MotorOutput,
        motor_control: &mut FloatOutBoyMotorControl,
        now: TimestampTicks,
        sample_rate: SampleRate,
    ) {
        let type_to_play = feedback_type(config, input);
        if type_to_play != self.type_playing && self.can_change_type {
            self.type_playing = type_to_play;
            self.tone_timer = now;
        }

        let should_be_playing = if matches!(self.type_playing, HapticFeedbackType::None) {
            self.can_change_type = true;
            false
        } else {
            let beats = self.type_playing.beats();
            if beats == 0 {
                self.can_change_type = true;
                true
            } else {
                let tone_time = now.wrapping_duration_since(self.tone_timer).as_ticks()
                    % (TONE_LENGTH_TICKS * beats);
                let beat = tone_time / TONE_LENGTH_TICKS;
                let off_beat = beats.saturating_sub(2);
                self.can_change_type = !self.is_playing && beat == 0;
                beat.is_multiple_of(2) && (off_beat == 0 || beat != off_beat)
            }
        };

        if self.is_playing && !should_be_playing {
            play_foc_tone(
                motor,
                AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(1.0)),
                AudioVoltage::new(Voltage::ZERO),
            );
            motor_control.stop_tone();
            self.is_playing = false;
        } else if should_be_playing {
            let strength = strength_scale(config, input.speed);
            let tone = match self.type_playing {
                HapticFeedbackType::DutySpeed | HapticFeedbackType::DutyContinuous => {
                    Some((config.duty_frequency(), config.duty_strength()))
                }
                HapticFeedbackType::ErrorTemperature
                | HapticFeedbackType::ErrorVoltage
                | HapticFeedbackType::ErrorFatal => {
                    Some((config.error_frequency(), config.error_strength()))
                }
                HapticFeedbackType::None => None,
            };
            if let Some((frequency, voltage)) = tone
                && voltage.voltage().is_positive()
            {
                play_foc_tone(
                    motor,
                    frequency,
                    AudioVoltage::new(Voltage::from_volts(voltage.voltage().as_volts() * strength)),
                );
            }
            if config.vibrate_strength().current().is_positive() {
                motor_control.play_tone(
                    config.vibrate_frequency(),
                    MotorCurrent::new(Current::from_amps(
                        config.vibrate_strength().current().as_amps() * strength,
                    )),
                    sample_rate,
                );
            }
            self.is_playing = true;
        }
    }
}

fn feedback_type(
    config: FloatOutBoyHapticConfig<'_>,
    input: HapticFeedbackInput,
) -> HapticFeedbackType {
    if !matches!(input.run_state, FloatOutBoyRunState::Running)
        || matches!(input.mode, FloatOutBoyMode::HandTest)
    {
        return HapticFeedbackType::None;
    }
    if input.fatal_error {
        return HapticFeedbackType::ErrorFatal;
    }
    match input.setpoint_adjustment {
        FloatOutBoySetpointAdjustment::PushbackDuty => {
            return if input.duty_cycle > input.duty_solid_threshold {
                HapticFeedbackType::DutyContinuous
            } else {
                HapticFeedbackType::DutySpeed
            };
        }
        FloatOutBoySetpointAdjustment::PushbackSpeed => return HapticFeedbackType::DutySpeed,
        FloatOutBoySetpointAdjustment::PushbackTemperature => {
            return HapticFeedbackType::ErrorTemperature;
        }
        FloatOutBoySetpointAdjustment::PushbackLowVoltage
        | FloatOutBoySetpointAdjustment::PushbackHighVoltage
        | FloatOutBoySetpointAdjustment::PushbackError => return HapticFeedbackType::ErrorVoltage,
        _ => {}
    }
    if config.current_threshold().as_ratio() > 0.0
        && input.current_saturation > config.current_threshold()
    {
        HapticFeedbackType::DutyContinuous
    } else {
        HapticFeedbackType::None
    }
}

fn strength_scale(config: FloatOutBoyHapticConfig<'_>, speed: Speed) -> f32 {
    let maximum_speed = config
        .max_strength_speed()
        .as_kilometers_per_hour()
        .max(1.0);
    let speed = speed.as_kilometers_per_hour();
    let minimum = config.min_strength().as_ratio();
    let linear = (1.0 - config.strength_curvature().as_ratio()) * (1.0 - minimum) / maximum_speed;
    let quadratic = (1.0 - minimum - linear * maximum_speed) / (maximum_speed * maximum_speed);
    (minimum + linear * speed + quadratic * speed * speed).min(1.0)
}

fn play_foc_tone(motor: &impl MotorOutput, frequency: AudioFrequency, voltage: AudioVoltage) {
    motor.play_foc_tone(AudioChannel::FIRST, frequency, voltage);
}

#[cfg(test)]
mod tests {
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
}
