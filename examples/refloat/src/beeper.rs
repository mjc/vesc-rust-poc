//! Refloat external-beeper sequencing.

/// External-beeper output level.
#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefloatBeeperLevel {
    Low,
    High,
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatBeeperLevel {
    pub(crate) const fn digital_output(self) -> vescpkg_rs::DigitalOutputLevel {
        match self {
            Self::Low => vescpkg_rs::DigitalOutputLevel::Low,
            Self::High => vescpkg_rs::DigitalOutputLevel::High,
        }
    }
}

/// Source-defined alert sequences used by Refloat's BMS paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefloatBeeperAlert {
    ThreeShort,
    ThreeLong,
    Long(RefloatBeeperCount),
    #[cfg(any(test, target_arch = "arm"))]
    FourShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefloatBeeperCount(u8);

impl RefloatBeeperCount {
    pub(crate) const SEVEN: Self = Self(7);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperTransitions(u8);

impl RefloatBeeperTransitions {
    const NONE: Self = Self(0);
    const THREE_BEEPS: Self = Self(7);
    #[cfg(any(test, target_arch = "arm"))]
    const FOUR_BEEPS: Self = Self(9);

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    const fn from_beeps(count: RefloatBeeperCount) -> Self {
        Self(count.0 * 2 + 1)
    }

    #[cfg(any(test, target_arch = "arm"))]
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

impl RefloatBeeperAlert {
    const fn sequence(self) -> (RefloatBeeperTransitions, RefloatBeeperPeriod) {
        match self {
            Self::ThreeShort => (
                RefloatBeeperTransitions::THREE_BEEPS,
                RefloatBeeperPeriod::SHORT,
            ),
            Self::ThreeLong => (
                RefloatBeeperTransitions::THREE_BEEPS,
                RefloatBeeperPeriod::LONG,
            ),
            Self::Long(count) => (
                RefloatBeeperTransitions::from_beeps(count),
                RefloatBeeperPeriod::LONG,
            ),
            #[cfg(any(test, target_arch = "arm"))]
            Self::FourShort => (
                RefloatBeeperTransitions::FOUR_BEEPS,
                RefloatBeeperPeriod::SHORT,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperCountdown(u16);

impl RefloatBeeperCountdown {
    const IDLE: Self = Self(0);

    #[cfg(any(test, target_arch = "arm"))]
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

        (self.transitions, self.period) = alert.sequence();
        self.countdown.restart(self.period);
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[cfg(any(test, target_arch = "arm"))]
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

    use super::{
        RefloatBeeper, RefloatBeeperAlert, RefloatBeeperCount, RefloatBeeperLevel,
    };
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

    #[test]
    fn seven_long_alert_uses_refloat_capped_transition_count() {
        let mut beeper = RefloatBeeper::new(true);
        beeper.alert(RefloatBeeperAlert::Long(RefloatBeeperCount::SEVEN));

        let changes: Vec<_> = (1..=4_500)
            .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
            .collect();

        assert_eq!(changes.len(), 15);
        assert_eq!(changes.last(), Some(&(4_500, RefloatBeeperLevel::Low)));
    }
}
