use crate::ema::EmaAlpha;
use vescpkg_rs::prelude::{Frequency, Rpm, SampleRate, VescSeconds};

// The package schema caps loop frequency at 4 kHz; Refloat's 8 Hz source
// formula needs 221 samples there. One spare slot bounds package state.
const MAX_WINDOW: usize = 222;
const MAX_WINDOW_U8: u8 = 222;
const MAX_WINDOW_U32: u32 = 222;
const ABS_ERPM_CUTOFF: Frequency = Frequency::from_hertz(10.0);
const ACCELERATION_CUTOFF: Frequency = Frequency::from_hertz(8.0);

fn window_as_f32(window: u8) -> f32 {
    f32::from(window)
}

#[derive(Debug)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq))]
pub(super) struct MotorKinematicsTracker {
    last_erpm: Rpm,
    smoothed_abs_erpm: Rpm,
    absolute_speed_smoothing: EmaAlpha,
    average: Rpm,
    #[cfg(not(target_arch = "arm"))]
    history: [Rpm; MAX_WINDOW],
    #[cfg(target_arch = "arm")]
    history: Option<vescpkg_rs::FallibleBox<[Rpm; MAX_WINDOW]>>,
    next: u8,
    window: u8,
    pending_window: u8,
}

impl Default for MotorKinematicsTracker {
    fn default() -> Self {
        let mut tracker = Self {
            last_erpm: Rpm::ZERO,
            smoothed_abs_erpm: Rpm::ZERO,
            absolute_speed_smoothing: EmaAlpha::default(),
            average: Rpm::ZERO,
            #[cfg(not(target_arch = "arm"))]
            history: [Rpm::ZERO; MAX_WINDOW],
            #[cfg(target_arch = "arm")]
            history: None,
            next: 0,
            window: 0,
            pending_window: 0,
        };
        tracker.configure(crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE);
        tracker
    }
}

impl MotorKinematicsTracker {
    #[cfg(target_arch = "arm")]
    pub(super) fn allocate_history(&mut self) -> bool {
        let Ok(history) = vescpkg_rs::FallibleBox::try_new([Rpm::ZERO; MAX_WINDOW]) else {
            return false;
        };
        self.history = Some(history);
        true
    }

    pub(super) fn configure(&mut self, sample_rate: SampleRate) {
        self.absolute_speed_smoothing = EmaAlpha::from_sample_rate(ABS_ERPM_CUTOFF, sample_rate);
        let normalized_cutoff = ACCELERATION_CUTOFF.as_hertz() / sample_rate.as_hertz();
        let window = u8::try_from(
            crate::wire::saturating_trunc_f32_to_u32(
                vescpkg_rs::sqrt(0.196_202 + normalized_cutoff * normalized_cutoff)
                    / normalized_cutoff,
            )
            .min(MAX_WINDOW_U32),
        )
        .unwrap_or(MAX_WINDOW_U8)
        .clamp(1, MAX_WINDOW_U8);
        if self.window == 0 {
            self.window = window;
        } else if window != self.window && self.pending_window == 0 {
            self.pending_window = window;
        }
    }

    pub(super) fn record(&mut self, motor_erpm: Rpm, elapsed: VescSeconds) {
        let previous_abs_erpm = self.smoothed_abs_erpm.as_revolutions_per_minute();
        let current_abs_erpm = motor_erpm.abs().as_revolutions_per_minute();
        self.smoothed_abs_erpm = Rpm::from_revolutions_per_minute(
            previous_abs_erpm
                + self.absolute_speed_smoothing.factor() * (current_abs_erpm - previous_abs_erpm),
        );

        // C map: `third_party/float-out-boy/src/motor_data.c:128-133` subtracts the previous ERPM,
        // replaces one rolling history slot, and adjusts the stored average by the delta.
        let current = (motor_erpm - self.last_erpm) / elapsed.as_seconds();
        let next = usize::from(self.next);
        #[cfg(not(target_arch = "arm"))]
        let slot = self.history.get_mut(next);
        #[cfg(target_arch = "arm")]
        let slot = self
            .history
            .as_deref_mut()
            .and_then(|history| history.get_mut(next));
        let Some(slot) = slot else {
            self.reset_acceleration();
            self.last_erpm = motor_erpm;
            return;
        };
        let previous = core::mem::replace(slot, current);
        let divisor = window_as_f32(self.window);
        self.average = self.average + (current - previous) / divisor;
        self.apply_pending_window_at_wrap();

        self.last_erpm = motor_erpm;
        self.next = self.next.saturating_add(1);
        if self.next >= self.window {
            self.next = 0;
        }
    }

    fn apply_pending_window_at_wrap(&mut self) {
        let old_window = self.window;
        let new_window = self.pending_window;
        if new_window == 0 || self.next != old_window.saturating_sub(1) {
            return;
        }

        if new_window < old_window {
            let range = usize::from(new_window)..usize::from(old_window);
            #[cfg(not(target_arch = "arm"))]
            let Some(removed) = self
                .history
                .get(range)
                .map(|history| history.iter().copied().fold(Rpm::ZERO, core::ops::Add::add))
            else {
                return;
            };
            #[cfg(target_arch = "arm")]
            let Some(removed) = self.history.as_deref().and_then(|history| {
                history
                    .get(range)
                    .map(|history| history.iter().copied().fold(Rpm::ZERO, core::ops::Add::add))
            }) else {
                return;
            };
            self.average = (self.average - removed / window_as_f32(old_window))
                * (window_as_f32(old_window) / window_as_f32(new_window));
        } else {
            let range = usize::from(old_window)..usize::from(new_window);
            #[cfg(not(target_arch = "arm"))]
            let Some(history) = self.history.get_mut(range) else {
                return;
            };
            #[cfg(not(target_arch = "arm"))]
            history.fill(self.average);
            #[cfg(target_arch = "arm")]
            let Some(history) = self
                .history
                .as_deref_mut()
                .and_then(|history| history.get_mut(range))
            else {
                return;
            };
            #[cfg(target_arch = "arm")]
            history.fill(self.average);
        }

        self.window = new_window;
        self.pending_window = 0;
    }

    pub(super) fn reset_acceleration(&mut self) {
        self.average = Rpm::ZERO;
        #[cfg(not(target_arch = "arm"))]
        self.history.fill(Rpm::ZERO);
        #[cfg(target_arch = "arm")]
        if let Some(history) = self.history.as_deref_mut() {
            history.fill(Rpm::ZERO);
        }
        self.next = 0;
    }

    pub(super) const fn average(&self) -> Rpm {
        // C map: `motor_data.c` exposes the rolling average ERPM as the
        // filtered acceleration output.
        self.average
    }

    pub(super) const fn smoothed_abs_erpm(&self) -> Rpm {
        self.smoothed_abs_erpm
    }
}

#[cfg(test)]
mod tests;
