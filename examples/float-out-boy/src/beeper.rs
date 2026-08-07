//! Float Out Boy external-beeper sequencing.

pub(crate) use vescpkg_rs::DigitalOutputLevel as FloatOutBoyBeeperLevel;

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
    pending_level: Option<FloatOutBoyBeeperLevel>,
}

impl FloatOutBoyBeeper {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            transitions: FloatOutBoyBeeperTransitions::NONE,
            period: FloatOutBoyBeeperPeriod::SHORT,
            countdown: FloatOutBoyBeeperCountdown::IDLE,
            pending_level: None,
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
        // Refloat's unused `beeper_enable` helper also drives low here, but
        // `configure` assigns the flag directly and can leave the pin high.
        // Keep the intended fail-safe behavior instead of reproducing that bug.
        if self.enabled && !enabled {
            self.pending_level = Some(FloatOutBoyBeeperLevel::Low);
        }
        self.enabled = enabled;
    }

    #[cfg(test)]
    pub(crate) fn on(&mut self) {
        if self.enabled && self.transitions.is_empty() {
            self.pending_level = Some(FloatOutBoyBeeperLevel::High);
        }
    }

    pub(crate) fn force_on(&mut self) {
        if self.enabled {
            self.pending_level = Some(FloatOutBoyBeeperLevel::High);
        }
    }

    pub(crate) fn off(&mut self) {
        if self.transitions.is_empty() {
            self.pending_level = Some(FloatOutBoyBeeperLevel::Low);
        }
    }

    #[cfg(test)]
    pub(crate) fn force_off(&mut self) {
        self.pending_level = Some(FloatOutBoyBeeperLevel::Low);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn take_level(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.pending_level.take()
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn tick(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        if self.enabled && !self.transitions.is_empty() && self.countdown.tick() {
            self.countdown.restart(self.period);
            self.pending_level = Some(self.transitions.advance());
        }

        self.take_level()
    }
}

#[cfg(test)]
mod tests;
