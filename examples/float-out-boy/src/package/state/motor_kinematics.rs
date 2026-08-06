// C map: Float Out Boy averages ERPM deltas over this many samples at
// `third_party/float-out-boy/src/motor_data.h:26`.
const WINDOW: usize = 40;
const WINDOW_U8: u8 = 40;
const ABS_ERPM_SMOOTHING_FACTOR: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccelerationHistoryIndex(u8);

impl AccelerationHistoryIndex {
    const START: Self = Self(0);

    const fn next(self) -> Self {
        Self(self.0.wrapping_add(1) % WINDOW_U8)
    }

    fn as_usize(self) -> usize {
        usize::from(self.0)
    }

    fn replace(
        self,
        history: &mut [vescpkg_rs::prelude::Rpm; WINDOW],
        current: vescpkg_rs::prelude::Rpm,
    ) -> vescpkg_rs::prelude::Rpm {
        // C map: `third_party/float-out-boy/src/motor_data.c:128-133` swaps one
        // rolling sample slot before updating the running average.
        match history.get_mut(self.as_usize()) {
            Some(slot) => {
                let previous = *slot;
                *slot = current;
                previous
            }
            None => current,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MotorKinematicsTracker {
    last_erpm: vescpkg_rs::prelude::Rpm,
    smoothed_abs_erpm: vescpkg_rs::prelude::Rpm,
    average: vescpkg_rs::prelude::Rpm,
    history: [vescpkg_rs::prelude::Rpm; WINDOW],
    next: AccelerationHistoryIndex,
}

impl Default for MotorKinematicsTracker {
    fn default() -> Self {
        // C map: `motor_data_init` starts the rolling ERPM average at zero.
        Self {
            last_erpm: vescpkg_rs::prelude::Rpm::ZERO,
            smoothed_abs_erpm: vescpkg_rs::prelude::Rpm::ZERO,
            average: vescpkg_rs::prelude::Rpm::ZERO,
            history: [vescpkg_rs::prelude::Rpm::ZERO; WINDOW],
            next: AccelerationHistoryIndex::START,
        }
    }
}

impl MotorKinematicsTracker {
    pub(super) fn record(&mut self, motor_erpm: vescpkg_rs::prelude::Rpm) {
        let previous_abs_erpm = self.smoothed_abs_erpm.as_revolutions_per_minute();
        let current_abs_erpm = motor_erpm.abs().as_revolutions_per_minute();
        self.smoothed_abs_erpm = vescpkg_rs::prelude::Rpm::from_revolutions_per_minute(
            previous_abs_erpm + ABS_ERPM_SMOOTHING_FACTOR * (current_abs_erpm - previous_abs_erpm),
        );

        // C map: `third_party/float-out-boy/src/motor_data.c:128-133` subtracts the previous ERPM,
        // replaces one rolling history slot, and adjusts the stored average by the delta.
        let current = motor_erpm - self.last_erpm;
        let previous = self.next.replace(&mut self.history, current);
        self.average = self.average + (current - previous) / f32::from(WINDOW_U8);

        self.last_erpm = motor_erpm;
        self.next = self.next.next();
    }

    pub(super) fn reset_acceleration(&mut self) {
        self.average = vescpkg_rs::prelude::Rpm::ZERO;
        self.history = [vescpkg_rs::prelude::Rpm::ZERO; WINDOW];
        self.next = AccelerationHistoryIndex::START;
    }

    pub(super) const fn average(self) -> vescpkg_rs::prelude::Rpm {
        // C map: `motor_data.c` exposes the rolling average ERPM as the
        // filtered acceleration output.
        self.average
    }

    pub(super) const fn smoothed_abs_erpm(self) -> vescpkg_rs::prelude::Rpm {
        self.smoothed_abs_erpm
    }
}

#[cfg(test)]
mod tests;
