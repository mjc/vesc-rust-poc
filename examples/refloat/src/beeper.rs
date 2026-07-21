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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatBeeperTransitions(u8);

impl RefloatBeeperTransitions {
    const NONE: Self = Self(0);
    const THREE_BEEPS: Self = Self(7);

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
}
