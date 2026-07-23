//! Refloat footpad-sequence activation state.
//!
//! The timing and repeat-state rules mirror `third_party/float-out-boy/src/konami.c`.
//! The sequence itself is borrowed from a promoted static array, so this remains
//! allocation-free in the package image.

use crate::package::time::float_out_boy_ticks_elapsed_seconds;
use crate::domain::FloatOutBoyFootpadState;
use vescpkg_rs::prelude::{TimestampTicks, VescSeconds};

const STEP_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.15);
const SEQUENCE_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FloatOutBoyKonami {
    sequence: &'static [FloatOutBoyFootpadState],
    state: usize,
    timer: TimestampTicks,
}

impl FloatOutBoyKonami {
    pub(super) const fn new(sequence: &'static [FloatOutBoyFootpadState]) -> Self {
        Self {
            sequence,
            state: 0,
            timer: TimestampTicks::from_ticks(0),
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

    pub(super) fn check(
        &mut self,
        footpad: FloatOutBoyFootpadState,
        now: TimestampTicks,
    ) -> bool {
        if self.sequence.is_empty() {
            return false;
        }
        if self.state > 0
            && float_out_boy_ticks_elapsed_seconds(now, self.timer, SEQUENCE_TIMEOUT)
        {
            self.reset();
        }

        if footpad == self.sequence[self.state]
            && float_out_boy_ticks_elapsed_seconds(now, self.timer, STEP_TIMEOUT)
        {
            self.state += 1;
            if self.state == self.sequence.len() {
                self.reset();
                return true;
            }
            self.timer = now;
        } else if self.state > 0 && footpad != self.sequence[self.state - 1] {
            self.reset();
        }
        false
    }

    fn reset(&mut self) {
        self.state = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEQUENCE: &[FloatOutBoyFootpadState] = &[
        FloatOutBoyFootpadState::Left,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Right,
    ];

    #[test]
    fn sequence_requires_source_timing_and_completes_once() {
        let mut konami = FloatOutBoyKonami::new(SEQUENCE);
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(0)));
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(1_501)));
        assert!(!konami.check(FloatOutBoyFootpadState::None, TimestampTicks::from_ticks(3_002)));
        assert!(konami.check(FloatOutBoyFootpadState::Right, TimestampTicks::from_ticks(4_503)));
        assert!(!konami.check(FloatOutBoyFootpadState::Right, TimestampTicks::from_ticks(6_004)));
    }

    #[test]
    fn wrong_state_resets_but_repeated_previous_state_is_held() {
        let mut konami = FloatOutBoyKonami::new(SEQUENCE);
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(1_501)));
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(2_000)));
        assert!(!konami.check(FloatOutBoyFootpadState::Right, TimestampTicks::from_ticks(3_501)));
        assert!(!konami.check(FloatOutBoyFootpadState::None, TimestampTicks::from_ticks(5_002)));
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(6_503)));
    }

    #[test]
    fn incomplete_sequence_expires_after_half_second() {
        let mut konami = FloatOutBoyKonami::new(SEQUENCE);
        assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(1_501)));
        assert!(!konami.check(FloatOutBoyFootpadState::None, TimestampTicks::from_ticks(7_502)));
        assert!(!konami.check(FloatOutBoyFootpadState::Right, TimestampTicks::from_ticks(9_003)));
    }
}
