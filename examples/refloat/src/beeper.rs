//! Refloat external-beeper sequencing.

/// External-beeper output level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefloatBeeperLevel {
    Low,
    High,
}

/// Source-defined alert sequences used by Refloat's BMS paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefloatBeeperAlert {
    ThreeShort,
    ThreeLong,
    FourShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperTransitions(u8);

impl RefloatBeeperTransitions {
    const NONE: Self = Self(0);
    const THREE_BEEPS: Self = Self(7);
    const FOUR_BEEPS: Self = Self(9);

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn advance(&mut self) -> RefloatBeeperLevel {
        self.0 -= 1;
        if self.0 & 1 == 1 {
            RefloatBeeperLevel::High
        } else {
            RefloatBeeperLevel::Low
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperPeriod(u16);

impl RefloatBeeperPeriod {
    const SHORT: Self = Self(80);
    const LONG: Self = Self(300);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperCountdown(u16);

impl RefloatBeeperCountdown {
    const IDLE: Self = Self(0);

    fn tick(&mut self) -> bool {
        self.0 -= 1;
        self.0 == 0
    }

    fn restart(&mut self, period: RefloatBeeperPeriod) {
        self.0 = period.0;
    }
}

/// Refloat's source-compatible external-beeper state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefloatBeeper {
    enabled: bool,
    transitions: RefloatBeeperTransitions,
    period: RefloatBeeperPeriod,
    countdown: RefloatBeeperCountdown,
}

impl RefloatBeeper {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            transitions: RefloatBeeperTransitions::NONE,
            period: RefloatBeeperPeriod::SHORT,
            countdown: RefloatBeeperCountdown::IDLE,
        }
    }

    pub(crate) fn alert(&mut self, alert: RefloatBeeperAlert) {
        if !self.enabled || !self.transitions.is_empty() {
            return;
        }

        match alert {
            RefloatBeeperAlert::ThreeShort => {
                self.transitions = RefloatBeeperTransitions::THREE_BEEPS;
                self.period = RefloatBeeperPeriod::SHORT;
            }
            RefloatBeeperAlert::ThreeLong => {
                self.transitions = RefloatBeeperTransitions::THREE_BEEPS;
                self.period = RefloatBeeperPeriod::LONG;
            }
            RefloatBeeperAlert::FourShort => {
                self.transitions = RefloatBeeperTransitions::FOUR_BEEPS;
                self.period = RefloatBeeperPeriod::SHORT;
            }
        }
        self.countdown.restart(self.period);
    }

    pub(crate) fn tick(&mut self) -> Option<RefloatBeeperLevel> {
        if !self.enabled || self.transitions.is_empty() || !self.countdown.tick() {
            return None;
        }

        self.countdown.restart(self.period);
        Some(self.transitions.advance())
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::{RefloatBeeper, RefloatBeeperAlert, RefloatBeeperLevel};
    use crate::config::RefloatConfigImage;

    #[test]
    fn beeper_enable_decodes_exact_refloat_generated_offset() {
        let mut config = RefloatConfigImage::defaults();
        assert!(!config.beeper_enabled());

        assert!(config.editor().set_beeper_enabled(true));

        assert!(config.beeper_enabled());
        assert_eq!(config.as_bytes()[242], 1);
    }

    #[test]
    fn three_short_alert_matches_refloat_transition_sequence() {
        let mut beeper = RefloatBeeper::new(true);
        beeper.alert(RefloatBeeperAlert::ThreeShort);

        let changes: Vec<_> = (1..=560)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(
            changes,
            [
                (80, RefloatBeeperLevel::Low),
                (160, RefloatBeeperLevel::High),
                (240, RefloatBeeperLevel::Low),
                (320, RefloatBeeperLevel::High),
                (400, RefloatBeeperLevel::Low),
                (480, RefloatBeeperLevel::High),
                (560, RefloatBeeperLevel::Low),
            ]
        );
    }

    #[test]
    fn three_long_alert_uses_refloat_long_period() {
        let mut beeper = RefloatBeeper::new(true);
        beeper.alert(RefloatBeeperAlert::ThreeLong);

        let changes: Vec<_> = (1..=2_100)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(
            changes,
            [
                (300, RefloatBeeperLevel::Low),
                (600, RefloatBeeperLevel::High),
                (900, RefloatBeeperLevel::Low),
                (1_200, RefloatBeeperLevel::High),
                (1_500, RefloatBeeperLevel::Low),
                (1_800, RefloatBeeperLevel::High),
                (2_100, RefloatBeeperLevel::Low),
            ]
        );
    }

    #[test]
    fn four_short_alert_uses_refloat_transition_count() {
        let mut beeper = RefloatBeeper::new(true);
        beeper.alert(RefloatBeeperAlert::FourShort);

        let changes: Vec<_> = (1..=720)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(changes.len(), 9);
        assert_eq!(changes.last(), Some(&(720, RefloatBeeperLevel::Low)));
    }
}
