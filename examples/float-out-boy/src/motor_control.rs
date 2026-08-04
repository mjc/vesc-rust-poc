//! Float Out Boy motor output request/apply state.
//!
//! Source map: upstream owns this in `third_party/float-out-boy/src/motor_control.c`
//! and `third_party/float-out-boy/src/motor_control.h`.

use crate::config::FloatOutBoyParkingBrakeMode;
use crate::domain::FloatOutBoyRunState;
use vescpkg_rs::prelude::{AudioFrequency, SampleRate};
use vescpkg_rs::prelude::{
    BrakeCurrent, Current, CurrentOffDelay, DutyCycle, MotorCurrent, Rpm, SYSTEM_TICK_RATE_HZ,
    SignedRatio, TimestampTicks, VescSeconds,
};
use vescpkg_rs::{MotorOutput, WireByte};
const CURRENT_OFF_DELAY: CurrentOffDelay = CurrentOffDelay::new(VescSeconds::from_seconds(0.05));

fn tone_half_period_ticks(frequency: AudioFrequency, sample_rate: SampleRate) -> u8 {
    let half_period_ticks = sample_rate.as_hertz() / (2.0 * frequency.frequency().as_hertz());
    crate::wire::saturating_trunc_f32_to_u8(half_period_ticks)
}

/// Float Out Boy motor-control request state.
///
/// Upstream `MotorControl` stores `current_requested` and `requested_current`
/// at `third_party/float-out-boy/src/motor_control.h:27-30`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyMotorControl {
    disabled: bool,
    requested_current: Option<MotorCurrent>,
    // Float Out Boy updates this flag before every idle motor output at
    // `third_party/float-out-boy/src/motor_control.c:66-70`.
    parking_brake_active: bool,
    // Float Out Boy uses `brake_timer` to release idle motor output after one second
    // at `third_party/float-out-boy/src/motor_control.c:101-109`.
    brake_timer_ticks: TimestampTicks,
    tone_ticks: u8,
    tone_counter: u8,
    tone_high: bool,
    tone_intensity: MotorCurrent,
    click_transitions_remaining: u8,
}

impl FloatOutBoyMotorControl {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(crate) fn request_current(&mut self, current: MotorCurrent) {
        // Upstream `motor_control_request_current` sets the request flag and
        // stores the requested current at `third_party/float-out-boy/src/motor_control.c:44-47`.
        self.requested_current = Some(current);
    }

    pub(crate) fn play_tone(
        &mut self,
        frequency: AudioFrequency,
        intensity: MotorCurrent,
        sample_rate: SampleRate,
    ) {
        let ticks = tone_half_period_ticks(frequency, sample_rate);
        if ticks != self.tone_ticks {
            self.tone_ticks = ticks;
            self.tone_counter = ticks;
        }
        self.tone_intensity = intensity;
    }

    pub(crate) fn stop_tone(&mut self) {
        self.tone_ticks = 0;
        self.tone_high = false;
    }

    #[expect(clippy::inline_always, reason = "keeps the linked ARM image compact")]
    #[inline(always)]
    pub(crate) fn play_click(&mut self, current: WireByte, sample_rate: SampleRate) {
        if current.as_u8() != 0 {
            self.play_tone(
                AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(350.0)),
                MotorCurrent::new(Current::from_amps(f32::from(current.as_u8()))),
                sample_rate,
            );
            self.click_transitions_remaining = 4;
        }
    }

    #[inline]
    pub(crate) fn apply_requested_current(&mut self, motor: &impl MotorOutput) -> Option<bool> {
        self.requested_current.take().map(|current| {
            // Upstream keeps this sign unchanged: `motor_control_request_current`
            // stores it at `third_party/float-out-boy/src/motor_control.c:44-47`, then
            // `motor_control_apply` passes it to `mc_set_current` at
            // `third_party/float-out-boy/src/motor_control.c:93-99`.
            if !current.is_finite() {
                return false;
            }
            motor.keep_alive();
            motor.set_current_off_delay(CURRENT_OFF_DELAY).is_ok()
                && motor.set_current(current).is_ok()
        })
    }

    #[inline]
    pub(crate) fn apply(
        &mut self,
        motor: &impl MotorOutput,
        run_state: FloatOutBoyRunState,
        abs_erpm: Rpm,
        system_time_ticks: TimestampTicks,
        parking_brake_mode: FloatOutBoyParkingBrakeMode,
        brake_current: MotorCurrent,
    ) -> bool {
        if matches!(run_state, FloatOutBoyRunState::Disabled) {
            if !self.disabled {
                // C map: disabled mode sets 0A once, then stops touching motor output at
                // `third_party/float-out-boy/src/motor_control.c:53-60`.
                let applied = motor
                    .set_current(MotorCurrent::new(Current::from_amps(0.0)))
                    .is_ok();
                self.disabled = applied;
                return applied;
            }
            return false;
        }

        self.disabled = false;
        // Upstream updates `parking_brake_active` before idle output at
        // `third_party/float-out-boy/src/motor_control.c:66-70`; enum values come from
        // `third_party/float-out-boy/src/conf/datatypes.h:31-33`.
        let parking_brake_was_active = self.parking_brake_active;
        if matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Always)
            || matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Idle)
                && run_state != FloatOutBoyRunState::Running
                && abs_erpm < Rpm::from_revolutions_per_minute(50.0)
        {
            self.parking_brake_active = true;
        } else if matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Never)
            || run_state == FloatOutBoyRunState::Running
        {
            self.parking_brake_active = false;
        }
        if self.parking_brake_active && !parking_brake_was_active {
            // Intentional Refloat bug fix: upstream initializes `brake_timer`
            // to zero at `third_party/float-out-boy/src/motor_control.c:29`.
            // Activating the parking brake after one second of controller uptime
            // can therefore release it immediately at `motor_control.c:106`.
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

        if let Some(applied) = self.apply_requested_current(motor) {
            return applied;
        }

        motor.keep_alive();
        if abs_erpm > Rpm::from_revolutions_per_minute(200.0) {
            self.brake_timer_ticks = system_time_ticks;
        }
        if system_time_ticks
            .wrapping_duration_since(self.brake_timer_ticks)
            .as_ticks()
            > crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ)
        {
            // Upstream releases idle motor output by setting 0A once
            // `timer_older(time, brake_timer, 1)` passes at
            // `third_party/float-out-boy/src/motor_control.c:101-109`; `timer_older`
            // converts seconds at `third_party/float-out-boy/src/time.h:46-51`.
            return motor
                .set_current(MotorCurrent::new(Current::from_amps(0.0)))
                .is_ok();
        } else if self.parking_brake_active && abs_erpm < Rpm::from_revolutions_per_minute(2000.0) {
            // Upstream parking brake applies duty zero below 2000 ERPM at
            // `third_party/float-out-boy/src/motor_control.c:112-114`.
            motor.set_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0)));
        } else {
            // Upstream idle fallback applies configured brake current at
            // `third_party/float-out-boy/src/motor_control.c:115-117`.
            return motor
                .set_brake_current(BrakeCurrent::new(brake_current.current()))
                .is_ok();
        }
        true
    }
}

#[cfg(test)]
mod tests;
