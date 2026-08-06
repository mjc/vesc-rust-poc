use core::num::NonZeroU32;

use crate::config::FloatOutBoyHapticConfig;
use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState, FloatOutBoySetpointAdjustment};
use crate::motor_control::FloatOutBoyMotorControl;
use vescpkg_rs::prelude::{
    AudioChannel, AudioFrequency, AudioVoltage, Ratio, SYSTEM_TICK_RATE_HZ, SampleRate, Speed,
    TimestampTicks, Voltage,
};
use vescpkg_rs::{
    HapticBeatDuration, HapticPlayback, HapticPulsePattern, HapticPulsePlayer, MotorOutput,
    haptic_strength_scale,
};

const TONE_LENGTH_TICKS: u32 = crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ) / 10;
const TONE_LENGTH: HapticBeatDuration = match HapticBeatDuration::from_ticks(TONE_LENGTH_TICKS) {
    Some(duration) => duration,
    None => panic!("VESC haptic tone length must be nonzero"),
};

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
pub(super) enum HapticFeedbackType {
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
            Self::DutyContinuous => 0,
            Self::ErrorTemperature => 6,
            Self::ErrorVoltage => 8,
            Self::ErrorFatal => 10,
        }
    }

    const fn pattern(self) -> HapticPulsePattern {
        match NonZeroU32::new(self.beats()) {
            Some(beats) => HapticPulsePattern::repeating(TONE_LENGTH, beats),
            None => HapticPulsePattern::Continuous,
        }
    }
}

pub(super) type HapticFeedbackState = HapticPulsePlayer<HapticFeedbackType>;

pub(super) fn update_haptic_feedback(
    state: &mut HapticFeedbackState,
    config: FloatOutBoyHapticConfig<'_>,
    input: HapticFeedbackInput,
    motor: &impl MotorOutput,
    motor_control: &mut FloatOutBoyMotorControl,
    now: TimestampTicks,
    sample_rate: SampleRate,
) {
    match state.update(
        feedback_type(config, input),
        now,
        HapticFeedbackType::pattern,
    ) {
        HapticPlayback::Silent => {}
        HapticPlayback::Stop => {
            play_foc_tone(
                motor,
                AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(1.0)),
                AudioVoltage::new(Voltage::ZERO),
            );
            motor_control.stop_tone();
        }
        HapticPlayback::Play(kind) => {
            let strength = haptic_strength_scale(
                input.speed,
                config.min_strength(),
                config.max_strength_speed(),
                config.strength_curvature(),
            )
            .as_ratio();
            let (frequency, voltage) = match kind {
                HapticFeedbackType::DutySpeed | HapticFeedbackType::DutyContinuous => {
                    (config.duty_frequency(), config.duty_strength())
                }
                HapticFeedbackType::ErrorTemperature
                | HapticFeedbackType::ErrorVoltage
                | HapticFeedbackType::ErrorFatal => {
                    (config.error_frequency(), config.error_strength())
                }
            };
            if voltage.voltage().is_positive() {
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
        }
    }
}

fn feedback_type(
    config: FloatOutBoyHapticConfig<'_>,
    input: HapticFeedbackInput,
) -> Option<HapticFeedbackType> {
    if input.run_state != FloatOutBoyRunState::Running
        || matches!(input.mode, FloatOutBoyMode::HandTest)
    {
        return None;
    }
    if input.fatal_error {
        return Some(HapticFeedbackType::ErrorFatal);
    }
    let pushback = match input.setpoint_adjustment {
        FloatOutBoySetpointAdjustment::PushbackDuty => {
            Some(if input.duty_cycle > input.duty_solid_threshold {
                HapticFeedbackType::DutyContinuous
            } else {
                HapticFeedbackType::DutySpeed
            })
        }
        FloatOutBoySetpointAdjustment::PushbackSpeed => Some(HapticFeedbackType::DutySpeed),
        FloatOutBoySetpointAdjustment::PushbackTemperature => {
            Some(HapticFeedbackType::ErrorTemperature)
        }
        FloatOutBoySetpointAdjustment::PushbackLowVoltage
        | FloatOutBoySetpointAdjustment::PushbackHighVoltage
        | FloatOutBoySetpointAdjustment::PushbackError => Some(HapticFeedbackType::ErrorVoltage),
        _ => None,
    };
    pushback.or_else(|| {
        let threshold = config.current_threshold();
        (!threshold.is_zero() && input.current_saturation > threshold)
            .then_some(HapticFeedbackType::DutyContinuous)
    })
}

fn play_foc_tone(_motor: &impl MotorOutput, frequency: AudioFrequency, voltage: AudioVoltage) {
    let _ = vescpkg_rs::FocAudio.play_tone(AudioChannel::FIRST, frequency, voltage);
}

#[cfg(test)]
mod tests;
