//! Float Out Boy external-beeper sequencing.

#![deny(clippy::arithmetic_side_effects)]

/// External-beeper output level.
#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyBeeperLevel {
    Low,
    High,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyBeeperLevel {
    pub(crate) const fn digital_output(self) -> vescpkg_rs::DigitalOutputLevel {
        match self {
            Self::Low => vescpkg_rs::DigitalOutputLevel::Low,
            Self::High => vescpkg_rs::DigitalOutputLevel::High,
        }
    }
}

/// Source-defined alert sequences used by Float Out Boy's BMS paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatOutBoyBeeperAlert {
    Short(FloatOutBoyBeeperCount),
    Long(FloatOutBoyBeeperCount),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyBeeperCount(u8);

impl FloatOutBoyBeeperCount {
    pub(crate) const ONE: Self = Self(1);
    pub(crate) const TWO: Self = Self(2);
    pub(crate) const THREE: Self = Self(3);
    pub(crate) const FOUR: Self = Self(4);
    pub(crate) const FIVE: Self = Self(5);
    pub(crate) const SIX: Self = Self(6);
    pub(crate) const SEVEN: Self = Self(7);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyBeeperTransitions(u8);

impl FloatOutBoyBeeperTransitions {
    const NONE: Self = Self(0);

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    const fn from_beeps(count: FloatOutBoyBeeperCount) -> Self {
        Self(count.0.saturating_mul(2).saturating_add(1))
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn advance(&mut self) -> FloatOutBoyBeeperLevel {
        self.0 = self.0.saturating_sub(1);
        if self.0 & 1 == 1 {
            FloatOutBoyBeeperLevel::High
        } else {
            FloatOutBoyBeeperLevel::Low
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyBeeperPeriod(u16);

impl FloatOutBoyBeeperPeriod {
    const SHORT: Self = Self(80);
    const LONG: Self = Self(300);
}

impl FloatOutBoyBeeperAlert {
    const fn sequence(self) -> (FloatOutBoyBeeperTransitions, FloatOutBoyBeeperPeriod) {
        match self {
            Self::Short(count) => (
                FloatOutBoyBeeperTransitions::from_beeps(count),
                FloatOutBoyBeeperPeriod::SHORT,
            ),
            Self::Long(count) => (
                FloatOutBoyBeeperTransitions::from_beeps(count),
                FloatOutBoyBeeperPeriod::LONG,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyBeeperCountdown(u16);

impl FloatOutBoyBeeperCountdown {
    const IDLE: Self = Self(0);

    #[cfg(any(test, target_arch = "arm"))]
    fn tick(&mut self) -> bool {
        self.0 = self.0.saturating_sub(1);
        self.0 == 0
    }

    fn restart(&mut self, period: FloatOutBoyBeeperPeriod) {
        self.0 = period.0;
    }
}

/// Float Out Boy's source-compatible external-beeper state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyBeeper {
    enabled: bool,
    transitions: FloatOutBoyBeeperTransitions,
    period: FloatOutBoyBeeperPeriod,
    countdown: FloatOutBoyBeeperCountdown,
    pending_high: Option<bool>,
}

impl FloatOutBoyBeeper {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            transitions: FloatOutBoyBeeperTransitions::NONE,
            period: FloatOutBoyBeeperPeriod::SHORT,
            countdown: FloatOutBoyBeeperCountdown::IDLE,
            pending_high: None,
        }
    }

    pub(crate) fn alert(&mut self, alert: FloatOutBoyBeeperAlert) {
        if !self.enabled || !self.transitions.is_empty() {
            return;
        }

        (self.transitions, self.period) = alert.sequence();
        self.countdown.restart(self.period);
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(crate) fn on(&mut self, force: bool) {
        if self.enabled && (force || self.transitions.is_empty()) {
            self.pending_high = Some(true);
        }
    }

    pub(crate) fn off(&mut self, force: bool) {
        if force || self.transitions.is_empty() {
            self.pending_high = Some(false);
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn take_level(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.pending_high.take().map(|high| {
            if high {
                FloatOutBoyBeeperLevel::High
            } else {
                FloatOutBoyBeeperLevel::Low
            }
        })
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn tick(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        if self.enabled && !self.transitions.is_empty() && self.countdown.tick() {
            self.countdown.restart(self.period);
            self.pending_high = Some(matches!(
                self.transitions.advance(),
                FloatOutBoyBeeperLevel::High
            ));
        }

        self.take_level()
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::{
        FloatOutBoyBeeper, FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount,
        FloatOutBoyBeeperCountdown, FloatOutBoyBeeperLevel, FloatOutBoyBeeperTransitions,
    };
    use crate::config::FloatOutBoyConfigImage;

    #[test]
    fn idle_countdown_tick_saturates_instead_of_panicking() {
        let mut countdown = FloatOutBoyBeeperCountdown::IDLE;

        assert!(countdown.tick());
    }

    #[test]
    fn empty_transition_advance_saturates_instead_of_panicking() {
        let mut transitions = FloatOutBoyBeeperTransitions::NONE;

        assert_eq!(transitions.advance(), FloatOutBoyBeeperLevel::Low);
        assert!(transitions.is_empty());
    }

    #[test]
    fn beeper_enable_decodes_exact_float_out_boy_generated_offset() {
        let mut config = FloatOutBoyConfigImage::defaults();
        assert!(!config.beeper_enabled());

        assert!(config.editor().set_beeper_enabled(true));

        assert!(config.beeper_enabled());
        assert_eq!(config.as_bytes()[242], 1);
    }

    #[test]
    fn continuous_warning_flags_decode_exact_float_out_boy_generated_offsets() {
        let config = FloatOutBoyConfigImage::defaults();

        assert!(config.foot_beep_enabled());
        assert!(!config.duty_beep_enabled());
        assert_eq!(config.as_bytes()[28], 1);
        assert_eq!(config.as_bytes()[50], 0);
    }

    #[test]
    fn three_short_alert_matches_float_out_boy_transition_sequence() {
        let mut beeper = FloatOutBoyBeeper::new(true);
        beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));

        let changes: Vec<_> = (1..=560)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(
            changes,
            [
                (80, FloatOutBoyBeeperLevel::Low),
                (160, FloatOutBoyBeeperLevel::High),
                (240, FloatOutBoyBeeperLevel::Low),
                (320, FloatOutBoyBeeperLevel::High),
                (400, FloatOutBoyBeeperLevel::Low),
                (480, FloatOutBoyBeeperLevel::High),
                (560, FloatOutBoyBeeperLevel::Low),
            ]
        );
    }

    #[test]
    fn three_long_alert_uses_float_out_boy_long_period() {
        let mut beeper = FloatOutBoyBeeper::new(true);
        beeper.alert(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));

        let changes: Vec<_> = (1..=2_100)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(
            changes,
            [
                (300, FloatOutBoyBeeperLevel::Low),
                (600, FloatOutBoyBeeperLevel::High),
                (900, FloatOutBoyBeeperLevel::Low),
                (1_200, FloatOutBoyBeeperLevel::High),
                (1_500, FloatOutBoyBeeperLevel::Low),
                (1_800, FloatOutBoyBeeperLevel::High),
                (2_100, FloatOutBoyBeeperLevel::Low),
            ]
        );
    }

    #[test]
    fn four_short_alert_uses_float_out_boy_transition_count() {
        let mut beeper = FloatOutBoyBeeper::new(true);
        beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::FOUR));

        let changes: Vec<_> = (1..=720)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(changes.len(), 9);
        assert_eq!(changes.last(), Some(&(720, FloatOutBoyBeeperLevel::Low)));
    }

    #[test]
    fn seven_long_alert_uses_float_out_boy_capped_transition_count() {
        let mut beeper = FloatOutBoyBeeper::new(true);
        beeper.alert(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::SEVEN));

        let changes: Vec<_> = (1..=4_500)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(changes.len(), 15);
        assert_eq!(changes.last(), Some(&(4_500, FloatOutBoyBeeperLevel::Low)));
    }

    #[test]
    fn continuous_beeper_respects_alert_guard_and_force_like_float_out_boy() {
        let mut beeper = FloatOutBoyBeeper::new(true);
        beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));

        beeper.off(false);
        assert_eq!(beeper.take_level(), None);
        beeper.on(true);
        assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::High));

        let changes: Vec<_> = (1..=240)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();
        assert_eq!(
            changes,
            [
                (80, FloatOutBoyBeeperLevel::Low),
                (160, FloatOutBoyBeeperLevel::High),
                (240, FloatOutBoyBeeperLevel::Low),
            ]
        );

        beeper.on(false);
        assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::High));
        beeper.off(false);
        assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::Low));
    }

    #[test]
    fn disabled_beeper_rejects_on_but_still_allows_forced_off_like_float_out_boy() {
        let mut beeper = FloatOutBoyBeeper::new(false);

        beeper.on(true);
        assert_eq!(beeper.take_level(), None);
        beeper.off(true);
        assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::Low));
    }
}
