//! Shared VESC package motor-output request, tone, and idle-brake state.
//!
//! This models the package-level mechanism in `vedderb/vesc_pkg` root
//! `src/motor_control.c`; packages retain ownership of their run-state mapping
//! and configured parking-brake/current values.

// TODO(vescpkg-rs): Move the root balance package's 50/200/2000 ERPM brake
// thresholds, one-second timer, 50 ms current-off delay, and 350 Hz
// four-transition click into package-supplied policy.

use crate::prelude::{
    AudioFrequency, BrakeCurrent, Current, CurrentOffDelay, DutyCycle, MotorCurrent, Rpm,
    SYSTEM_TICK_RATE_HZ, SampleRate, SignedRatio, TimestampTicks, VescSeconds,
};
use crate::{MotorOutput, WireByte};

const CURRENT_OFF_DELAY: CurrentOffDelay = CurrentOffDelay::new(VescSeconds::from_seconds(0.05));

/// Package run-state distinctions consumed by shared motor-output policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorControlRunState {
    /// Apply zero current once, then leave disabled motor output untouched.
    Disabled,
    /// Apply package idle and parking-brake policy.
    Idle,
    /// Apply running motor requests and running idle fallback.
    Running,
}

/// Open byte value for VESC package parking-brake policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ParkingBrakeMode(u8);

impl ParkingBrakeMode {
    /// Keep the parking brake active whenever motor control is idle.
    pub const ALWAYS: Self = Self(0);
    /// Activate the parking brake only outside the running state.
    pub const IDLE: Self = Self(1);
    /// Never activate the parking brake.
    pub const NEVER: Self = Self(2);
}

impl From<u8> for ParkingBrakeMode {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<ParkingBrakeMode> for u8 {
    fn from(value: ParkingBrakeMode) -> Self {
        value.0
    }
}

/// Shared allocation-free VESC package motor-control state.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct MotorControl {
    disabled: bool,
    requested_current: Option<MotorCurrent>,
    parking_brake_active: bool,
    brake_timer_ticks: TimestampTicks,
    tone_ticks: u8,
    tone_counter: u8,
    tone_high: bool,
    tone_intensity: MotorCurrent,
    click_transitions_remaining: u8,
}

impl MotorControl {
    /// Build cleared motor-control state for tests and host fixtures.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue one motor-current request for the next apply pass.
    #[inline]
    pub fn request_current(&mut self, current: MotorCurrent) {
        self.requested_current = Some(current);
    }

    /// Start or update current-modulated motor tone output.
    pub fn play_tone(
        &mut self,
        frequency: AudioFrequency,
        intensity: MotorCurrent,
        sample_rate: SampleRate,
    ) {
        let half_period = sample_rate.as_hertz() / (2.0 * frequency.frequency().as_hertz());
        // Refloat's integer divider clamps a sub-tick half-period to one tick.
        let ticks = crate::protocol_buffer::saturating_trunc_f32_to_u8(half_period).max(1);
        if ticks != self.tone_ticks {
            self.tone_ticks = ticks;
            self.tone_counter = ticks;
        }
        self.tone_intensity = intensity;
    }

    /// Stop current-modulated motor tone output.
    pub fn stop_tone(&mut self) {
        self.tone_ticks = 0;
        self.tone_high = false;
    }

    /// Start the standard four-transition 350 Hz package click.
    #[expect(
        clippy::inline_always,
        reason = "keeps linked ARM package images compact"
    )]
    #[inline(always)]
    pub fn play_click(&mut self, current: WireByte, sample_rate: SampleRate) {
        if current.as_u8() != 0 {
            self.play_tone(
                AudioFrequency::new(crate::Frequency::from_hertz(350.0)),
                MotorCurrent::new(Current::from_amps(f32::from(current.as_u8()))),
                sample_rate,
            );
            self.click_transitions_remaining = 4;
        }
    }

    /// Apply and consume one queued current request.
    #[inline]
    pub fn apply_requested_current(&mut self, motor: &impl MotorOutput) -> bool {
        self.requested_current.take().is_some_and(|command| {
            motor.keep_alive();
            motor.set_current_off_delay(CURRENT_OFF_DELAY).is_ok()
                && motor.set_current(command).is_ok()
        })
    }

    /// Apply one shared package motor-control pass.
    #[inline]
    pub fn apply(
        &mut self,
        motor: &impl MotorOutput,
        run_state: MotorControlRunState,
        abs_erpm: Rpm,
        system_time_ticks: TimestampTicks,
        parking_brake_mode: ParkingBrakeMode,
        brake_current: MotorCurrent,
    ) -> bool {
        if run_state == MotorControlRunState::Disabled {
            if !self.disabled {
                let applied = motor.set_current(MotorCurrent::new(Current::ZERO)).is_ok();
                self.disabled = true;
                return applied;
            }
            return false;
        }

        self.disabled = false;
        let parking_brake_was_active = self.parking_brake_active;
        if parking_brake_mode == ParkingBrakeMode::ALWAYS
            || parking_brake_mode == ParkingBrakeMode::IDLE
                && run_state == MotorControlRunState::Idle
                && abs_erpm < Rpm::from_revolutions_per_minute(50.0)
        {
            self.parking_brake_active = true;
        } else if parking_brake_mode == ParkingBrakeMode::NEVER
            || run_state == MotorControlRunState::Running
        {
            self.parking_brake_active = false;
        }
        if self.parking_brake_active && !parking_brake_was_active {
            self.brake_timer_ticks = system_time_ticks;
        }

        if self.tone_ticks > 0 {
            self.tone_counter = self.tone_counter.saturating_sub(1);
            if self.tone_counter == 0 {
                self.tone_counter = self.tone_ticks;
                self.tone_high = !self.tone_high;
                self.click_transitions_remaining =
                    self.click_transitions_remaining.saturating_sub(1);
                if self.click_transitions_remaining == 1 {
                    self.stop_tone();
                }
            }
            let requested = self
                .requested_current
                .map_or(Current::ZERO, MotorCurrent::current);
            let tone = self.tone_intensity.current();
            self.request_current(MotorCurrent::new(if self.tone_high {
                requested + tone
            } else {
                requested - tone
            }));
        }

        if self.apply_requested_current(motor) {
            return true;
        }

        motor.keep_alive();
        if abs_erpm > Rpm::from_revolutions_per_minute(200.0) {
            self.brake_timer_ticks = system_time_ticks;
        }
        if system_time_ticks
            .wrapping_duration_since(self.brake_timer_ticks)
            .as_ticks()
            > crate::protocol_buffer::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ)
        {
            return motor.set_current(MotorCurrent::new(Current::ZERO)).is_ok();
        } else if self.parking_brake_active && abs_erpm < Rpm::from_revolutions_per_minute(2000.0) {
            motor.set_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0)));
        } else {
            return motor
                .set_brake_current(BrakeCurrent::new(brake_current.current()))
                .is_ok();
        }
        true
    }

    /// Override tone timing for saturation characterizations.
    #[cfg(feature = "test-support")]
    pub fn set_tone_phase_for_test(&mut self, ticks: u8, counter: u8) {
        self.tone_ticks = ticks;
        self.tone_counter = counter;
    }

    /// Return the tone counter for characterizations.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn tone_counter_for_test(self) -> u8 {
        self.tone_counter
    }
}
