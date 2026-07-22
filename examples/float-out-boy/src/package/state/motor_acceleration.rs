// C map: Float Out Boy averages ERPM deltas over this many samples at
// `third_party/float-out-boy/src/motor_data.h:26`.
const WINDOW: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccelerationHistoryIndex(u8);

impl AccelerationHistoryIndex {
    const START: Self = Self(0);

    const fn next(self) -> Self {
        Self((self.0 + 1) % WINDOW as u8)
    }

    const fn as_usize(self) -> usize {
        self.0 as usize
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
pub(super) struct MotorAccelerationTracker {
    last_erpm: vescpkg_rs::prelude::Rpm,
    average: vescpkg_rs::prelude::Rpm,
    history: [vescpkg_rs::prelude::Rpm; WINDOW],
    next: AccelerationHistoryIndex,
}

impl Default for MotorAccelerationTracker {
    fn default() -> Self {
        // C map: `motor_data_init` starts the rolling ERPM average at zero.
        Self {
            last_erpm: vescpkg_rs::prelude::Rpm::ZERO,
            average: vescpkg_rs::prelude::Rpm::ZERO,
            history: [vescpkg_rs::prelude::Rpm::ZERO; WINDOW],
            next: AccelerationHistoryIndex::START,
        }
    }
}

impl MotorAccelerationTracker {
    pub(super) fn record(&mut self, motor_erpm: vescpkg_rs::prelude::Rpm) {
        // C map: `third_party/float-out-boy/src/motor_data.c:128-133` subtracts the previous ERPM,
        // replaces one rolling history slot, and adjusts the stored average by the delta.
        let current = motor_erpm - self.last_erpm;
        let previous = self.next.replace(&mut self.history, current);
        self.average = self.average + (current - previous) / WINDOW as f32;

        self.last_erpm = motor_erpm;
        self.next = self.next.next();
    }

    pub(super) const fn average(self) -> vescpkg_rs::prelude::Rpm {
        // C map: `motor_data.c` exposes the rolling average ERPM as the
        // filtered acceleration output.
        self.average
    }
}

#[cfg(test)]
mod tests {
    use super::{AccelerationHistoryIndex, MotorAccelerationTracker, WINDOW};
    use vescpkg_rs::prelude::Rpm;

    #[test]
    fn acceleration_history_index_wraps_inside_float_out_boy_window() {
        let mut index = AccelerationHistoryIndex::START;

        for _ in 0..WINDOW {
            index = index.next();
        }

        assert_eq!(index, AccelerationHistoryIndex::START);
    }

    #[test]
    fn record_matches_float_out_boy_rolling_erpm_delta_average() {
        let mut tracker = MotorAccelerationTracker::default();

        for step in 1..=WINDOW {
            tracker.record(Rpm::from_revolutions_per_minute(step as f32 * 10.0));
        }

        assert_eq!(tracker.average().as_revolutions_per_minute(), 10.0);

        tracker.record(Rpm::from_revolutions_per_minute(410.0));

        assert_eq!(tracker.average().as_revolutions_per_minute(), 10.0);

        tracker.record(Rpm::from_revolutions_per_minute(450.0));

        // Float Out Boy replaces the oldest 10 ERPM sample with the current 40 ERPM sample:
        // `10 + (40 - 10) / ACCEL_ARRAY_SIZE`.
        assert_eq!(tracker.average().as_revolutions_per_minute(), 10.75);
    }
}
