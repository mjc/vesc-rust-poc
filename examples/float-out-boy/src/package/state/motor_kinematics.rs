use crate::ema::EmaAlpha;
use core::num::NonZeroU8;
use vescpkg_rs::prelude::{Frequency, Rpm, SampleRate, VescSeconds};

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct ElectricalAcceleration(f32);

impl ElectricalAcceleration {
    pub(super) const ZERO: Self = Self(0.0);

    pub(super) const fn from_erpm_per_second(value: f32) -> Self {
        Self(value)
    }

    fn from_speed_change(speed: Rpm, elapsed: VescSeconds) -> Self {
        Self(speed.as_revolutions_per_minute() / elapsed.as_seconds())
    }

    pub(super) const fn as_erpm_per_second(self) -> f32 {
        self.0
    }

    pub(super) const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub(super) const fn is_negative(self) -> bool {
        self.0.is_sign_negative()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HistoryWindow(NonZeroU8);

impl HistoryWindow {
    const fn new(value: u8) -> Self {
        Self(match NonZeroU8::new(value) {
            Some(value) => value,
            None => NonZeroU8::MIN,
        })
    }

    const fn get(self) -> u8 {
        self.0.get()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct HistoryIndex(u8);

impl HistoryIndex {
    const fn get(self) -> u8 {
        self.0
    }

    fn advance(&mut self, window: HistoryWindow) {
        self.0 = self.0.saturating_add(1);
        if self.0 >= window.get() {
            self.0 = 0;
        }
    }
}

// Float Out Boy's package loop is fixed at 500 Hz. Refloat's 8 Hz source
// formula needs 27 samples there; keep a small bounded margin for jitter.
const MAX_WINDOW: usize = 32;
const MAX_WINDOW_U8: u8 = 32;
const MAX_WINDOW_U32: u32 = 32;
const ABS_ERPM_CUTOFF: Frequency = Frequency::from_hertz(10.0);
const ACCELERATION_CUTOFF: Frequency = Frequency::from_hertz(8.0);

fn window_as_f32(window: HistoryWindow) -> f32 {
    f32::from(window.get())
}

#[derive(Debug)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq))]
pub(super) struct MotorKinematicsTracker {
    last_erpm: Rpm,
    smoothed_abs_erpm: Rpm,
    absolute_speed_smoothing: EmaAlpha,
    average: ElectricalAcceleration,
    #[cfg(not(target_arch = "arm"))]
    history: [ElectricalAcceleration; MAX_WINDOW],
    #[cfg(target_arch = "arm")]
    history: Option<alloc::vec::Vec<ElectricalAcceleration>>,
    next: HistoryIndex,
    window: HistoryWindow,
    pending_window: Option<HistoryWindow>,
}

impl Default for MotorKinematicsTracker {
    fn default() -> Self {
        let mut tracker = Self {
            last_erpm: Rpm::ZERO,
            smoothed_abs_erpm: Rpm::ZERO,
            absolute_speed_smoothing: EmaAlpha::default(),
            average: ElectricalAcceleration::ZERO,
            #[cfg(not(target_arch = "arm"))]
            history: [ElectricalAcceleration::ZERO; MAX_WINDOW],
            #[cfg(target_arch = "arm")]
            history: None,
            next: HistoryIndex::default(),
            window: HistoryWindow::new(1),
            pending_window: None,
        };
        tracker.configure(crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE);
        tracker.window = tracker.pending_window.take().unwrap_or(tracker.window);
        tracker
    }
}

impl MotorKinematicsTracker {
    #[cfg(target_arch = "arm")]
    pub(super) fn allocate_history(&mut self) -> bool {
        let mut history = alloc::vec::Vec::new();
        if history.try_reserve_exact(MAX_WINDOW).is_err() {
            return false;
        }
        history.resize(MAX_WINDOW, ElectricalAcceleration::ZERO);
        self.history = Some(history);
        true
    }

    pub(super) fn configure(&mut self, sample_rate: SampleRate) {
        self.absolute_speed_smoothing = EmaAlpha::from_sample_rate(ABS_ERPM_CUTOFF, sample_rate);
        let normalized_cutoff = ACCELERATION_CUTOFF.as_hertz() / sample_rate.as_hertz();
        let window = HistoryWindow::new(
            u8::try_from(
                crate::wire::saturating_trunc_f32_to_u32(
                    vescpkg_rs::sqrt(0.196_202 + normalized_cutoff * normalized_cutoff)
                        / normalized_cutoff,
                )
                .min(MAX_WINDOW_U32),
            )
            .unwrap_or(MAX_WINDOW_U8)
            .clamp(1, MAX_WINDOW_U8),
        );
        if window != self.window && self.pending_window.is_none() {
            self.pending_window = Some(window);
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
        let current =
            ElectricalAcceleration::from_speed_change(motor_erpm - self.last_erpm, elapsed);
        let next = usize::from(self.next.get());
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
        self.average.0 += (current.0 - previous.0) / divisor;
        self.apply_pending_window_at_wrap();

        self.last_erpm = motor_erpm;
        self.next.advance(self.window);
    }

    fn apply_pending_window_at_wrap(&mut self) {
        let old_window = self.window;
        let Some(new_window) = self.pending_window else {
            return;
        };
        if self.next.get() != old_window.get().saturating_sub(1) {
            return;
        }

        if new_window.get() < old_window.get() {
            let range = usize::from(new_window.get())..usize::from(old_window.get());
            #[cfg(not(target_arch = "arm"))]
            let Some(removed) = self
                .history
                .get(range)
                .map(|history| history.iter().map(|value| value.0).sum::<f32>())
            else {
                return;
            };
            #[cfg(target_arch = "arm")]
            let Some(removed) = self.history.as_deref().and_then(|history| {
                history
                    .get(range)
                    .map(|history| history.iter().map(|value| value.0).sum::<f32>())
            }) else {
                return;
            };
            self.average.0 = (self.average.0 - removed / window_as_f32(old_window))
                * (window_as_f32(old_window) / window_as_f32(new_window));
        } else {
            let range = usize::from(old_window.get())..usize::from(new_window.get());
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
        self.pending_window = None;
    }

    pub(super) fn reset_acceleration(&mut self) {
        self.average = ElectricalAcceleration::ZERO;
        #[cfg(not(target_arch = "arm"))]
        self.history.fill(ElectricalAcceleration::ZERO);
        #[cfg(target_arch = "arm")]
        if let Some(history) = self.history.as_deref_mut() {
            history.fill(ElectricalAcceleration::ZERO);
        }
        self.next = HistoryIndex::default();
    }

    pub(super) const fn average(&self) -> ElectricalAcceleration {
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
