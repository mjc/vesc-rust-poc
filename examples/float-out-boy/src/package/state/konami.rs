//! Refloat footpad-sequence activation state.
//!
//! The timing and repeat-state rules mirror `third_party/float-out-boy/src/konami.c`.
//! The sequence itself is borrowed from a promoted static array, so this remains
//! allocation-free in the package image.

use super::FloatOutBoyPackageState;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyFootpadState,
    FloatOutBoyMode, FloatOutBoyRunState,
};
use crate::package::time::float_out_boy_ticks_elapsed_seconds;
use vescpkg_rs::ImuPitch;
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

    pub(super) fn check(&mut self, footpad: FloatOutBoyFootpadState, now: TimestampTicks) -> bool {
        if self.sequence.is_empty() {
            return false;
        }
        if self.state > 0 && float_out_boy_ticks_elapsed_seconds(now, self.timer, SEQUENCE_TIMEOUT)
        {
            self.reset();
        }

        if self.sequence.get(self.state).copied() == Some(footpad)
            && float_out_boy_ticks_elapsed_seconds(now, self.timer, STEP_TIMEOUT)
        {
            self.state = self.state.saturating_add(1);
            if self.state == self.sequence.len() {
                self.reset();
                return true;
            }
            self.timer = now;
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

    fn reset(&mut self) {
        self.state = 0;
    }
}

impl FloatOutBoyPackageState {
    pub(super) fn refresh_konami_runtime_state(
        &mut self,
        current_pitch: ImuPitch,
        system_time_ticks: TimestampTicks,
    ) {
        let base = self.all_data_payloads.base();
        let ride_state = base.status().ride_state();
        // C refreshes `d->imu.pitch` before entering the READY Konami branch at
        // `third_party/float-out-boy/src/main.c:775,947-953`.
        let pitch = crate::wire::degrees(current_pitch.angle());
        let footpad = base.footpad().state();

        if matches!(ride_state.run_state(), FloatOutBoyRunState::Ready)
            && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
            && (75.0..105.0).contains(&pitch)
            && self.flywheel_konami.check(footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            // C map: `main.c:85-89` and `main.c:945-949`; this is the same
            // armed default flywheel command used by the native handler.
            let command = [
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                FloatOutBoyAppDataCommand::Flywheel.id(),
                0x82,
                0,
                0,
                0,
                0,
                1,
            ];
            self.handle_flywheel_packet(&command);
        }

        if self.serialized_config.hardware_led_mode_id() == 0 {
            return;
        }
        let status = self.led_runtime_status();
        if !status.headlights_enabled()
            && self.headlights_on_konami.check(footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            self.set_led_runtime_overrides(None, Some(true));
        }
        if status.headlights_enabled()
            && self.headlights_off_konami.check(footpad, system_time_ticks)
        {
            self.start_internal_led_confirmation(system_time_ticks);
            self.set_led_runtime_overrides(None, Some(false));
        }
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
        assert!(!konami.check(
            FloatOutBoyFootpadState::Left,
            TimestampTicks::from_ticks(1_501)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::None,
            TimestampTicks::from_ticks(3_002)
        ));
        assert!(konami.check(
            FloatOutBoyFootpadState::Right,
            TimestampTicks::from_ticks(4_503)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::Right,
            TimestampTicks::from_ticks(6_004)
        ));
    }

    #[test]
    fn wrong_state_resets_but_repeated_previous_state_is_held() {
        let mut konami = FloatOutBoyKonami::new(SEQUENCE);
        assert!(!konami.check(
            FloatOutBoyFootpadState::Left,
            TimestampTicks::from_ticks(1_501)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::Left,
            TimestampTicks::from_ticks(2_000)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::Right,
            TimestampTicks::from_ticks(3_501)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::None,
            TimestampTicks::from_ticks(5_002)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::Left,
            TimestampTicks::from_ticks(6_503)
        ));
    }

    #[test]
    fn incomplete_sequence_expires_after_half_second() {
        let mut konami = FloatOutBoyKonami::new(SEQUENCE);
        assert!(!konami.check(
            FloatOutBoyFootpadState::Left,
            TimestampTicks::from_ticks(1_501)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::None,
            TimestampTicks::from_ticks(7_502)
        ));
        assert!(!konami.check(
            FloatOutBoyFootpadState::Right,
            TimestampTicks::from_ticks(9_003)
        ));
    }
}
