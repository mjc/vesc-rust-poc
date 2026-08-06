//! Refloat footpad-sequence activation state.
//!
//! The timing and repeat-state rules mirror `third_party/float-out-boy/src/konami.c`.
//! The sequence itself is borrowed from a promoted static array, so this remains
//! allocation-free in the package image.

use crate::domain::FloatOutBoyFootpadState;
use vescpkg_rs::prelude::{AngleRadians, TimestampTicks, VescSeconds};
use vescpkg_rs::{ImuPitch, WrappingTimer};

const STEP_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.15);
const SEQUENCE_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FloatOutBoyKonami {
    sequence: &'static [FloatOutBoyFootpadState],
    state: usize,
    timer: WrappingTimer,
}

impl FloatOutBoyKonami {
    pub(super) const fn new(sequence: &'static [FloatOutBoyFootpadState]) -> Self {
        Self {
            sequence,
            state: 0,
            timer: WrappingTimer::started_at(TimestampTicks::from_ticks(0)),
        }
    }

    pub(super) const fn flywheel() -> Self {
        Self::new(&[
            FloatOutBoyFootpadState::Left,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Right,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Left,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Right,
            FloatOutBoyFootpadState::None,
        ])
    }

    pub(super) const fn headlights_on() -> Self {
        Self::new(&[
            FloatOutBoyFootpadState::Left,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Left,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Right,
        ])
    }

    pub(super) const fn headlights_off() -> Self {
        Self::new(&[
            FloatOutBoyFootpadState::Right,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Right,
            FloatOutBoyFootpadState::None,
            FloatOutBoyFootpadState::Left,
        ])
    }

    pub(super) fn check(&mut self, footpad: FloatOutBoyFootpadState, now: TimestampTicks) -> bool {
        if self.sequence.is_empty() {
            return false;
        }
        if self.state > 0 && self.timer.older_than(now, SEQUENCE_TIMEOUT) {
            self.reset();
        }

        if self.sequence.get(self.state).copied() == Some(footpad)
            && self.timer.older_than(now, STEP_TIMEOUT)
        {
            self.state = self.state.saturating_add(1);
            if self.state == self.sequence.len() {
                self.reset();
                return true;
            }
            self.timer.restart(now);
        } else if self
            .state
            .checked_sub(1)
            .and_then(|index| self.sequence.get(index))
            .copied()
            != Some(footpad)
            && self.state > 0
        {
            self.reset();
        }
        false
    }

    #[inline]
    pub(super) fn check_flywheel(
        &mut self,
        pitch: ImuPitch,
        footpad: FloatOutBoyFootpadState,
        now: TimestampTicks,
    ) -> bool {
        // C gates the Flywheel sequence to 75 < current IMU pitch < 105 at
        // `third_party/float-out-boy/src/main.c:947-953`.
        let pitch = pitch.angle();
        pitch > AngleRadians::from_degrees(75.0)
            && pitch < AngleRadians::from_degrees(105.0)
            && self.check(footpad, now)
    }

    fn reset(&mut self) {
        self.state = 0;
    }
}

#[cfg(test)]
mod tests;
