use crate::config::FloatOutBoyHapticConfig;
use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState, FloatOutBoySetpointAdjustment};
use crate::motor_control::FloatOutBoyMotorControl;
use vescpkg_rs::MotorOutput;
use vescpkg_rs::prelude::{
    AudioChannel, AudioFrequency, AudioVoltage, Current, Ratio, SYSTEM_TICK_RATE_HZ, SampleRate,
    Speed, TimestampTicks, Voltage,
};

const TONE_LENGTH_TICKS: u32 = crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ) / 10;

pub(super) fn normalized_current_saturation(current: Current, limit: Current) -> f32 {
    let limit = limit.abs();
    if limit.is_positive() {
        current.abs().as_amps() / limit.as_amps()
    } else {
        0.0
    }
}

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

impl Default for HapticFeedbackState {
    fn default() -> Self {
        Self::new()
    }
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
                let cycle_ticks = TONE_LENGTH_TICKS.saturating_mul(beats);
                let tone_time = now
                    .wrapping_duration_since(self.tone_timer)
                    .as_ticks()
                    .checked_rem(cycle_ticks)
                    .unwrap_or_default();
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
            let vibrate_strength = config.vibrate_strength();
            if vibrate_strength.is_positive() {
                motor_control.play_tone(
                    config.vibrate_frequency(),
                    vibrate_strength * strength,
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
    let current_threshold = config.current_threshold();
    if !current_threshold.is_zero() && input.current_saturation > current_threshold {
        HapticFeedbackType::DutyContinuous
    } else {
        HapticFeedbackType::None
    }
}

fn strength_scale(config: FloatOutBoyHapticConfig<'_>, speed: Speed) -> f32 {
    let configured_maximum_speed = config.max_strength_speed().as_kilometers_per_hour();
    let maximum_speed = if configured_maximum_speed > 0.0 {
        configured_maximum_speed
    } else {
        1.0
    };
    let speed = speed.as_kilometers_per_hour().abs();
    let minimum = config.min_strength().as_ratio();
    let linear = (1.0 - config.strength_curvature().as_ratio()) * (1.0 - minimum) / maximum_speed;
    let quadratic = (1.0 - minimum - linear * maximum_speed) / (maximum_speed * maximum_speed);
    (minimum + linear * speed + quadratic * speed * speed).min(1.0)
}

fn play_foc_tone(_motor: &impl MotorOutput, frequency: AudioFrequency, voltage: AudioVoltage) {
    let _ = vescpkg_rs::FocAudio.play_tone(AudioChannel::FIRST, frequency, voltage);
}

#[cfg(test)]
mod tests;
