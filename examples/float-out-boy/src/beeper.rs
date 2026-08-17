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

const SHORT_BEEP_PERIOD: u16 = 80;
const LONG_BEEP_PERIOD: u16 = 300;

impl FloatOutBoyBeeperAlert {
    const fn sequence(self) -> (u8, u16) {
        match self {
            Self::Short(count) => (
                count.0.saturating_mul(2).saturating_add(1),
                SHORT_BEEP_PERIOD,
            ),
            Self::Long(count) => (
                count.0.saturating_mul(2).saturating_add(1),
                LONG_BEEP_PERIOD,
            ),
        }
    }
}

/// Float Out Boy's source-compatible external-beeper state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyBeeper {
    enabled: bool,
    transitions: u8,
    period: u16,
    countdown: u16,
    pending_level: Option<FloatOutBoyBeeperLevel>,
}

impl Default for FloatOutBoyBeeper {
    fn default() -> Self {
        Self::new(crate::config::FLOAT_OUT_BOY_DEFAULT_BEEPER_ENABLED)
    }
}

impl FloatOutBoyBeeper {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            transitions: 0,
            period: SHORT_BEEP_PERIOD,
            countdown: 0,
            pending_level: None,
        }
    }

    pub(crate) fn alert(&mut self, alert: FloatOutBoyBeeperAlert) {
        if !self.enabled || self.transitions != 0 {
            return;
        }

        (self.transitions, self.period) = alert.sequence();
        self.countdown = self.period;
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
        if self.enabled && self.transitions == 0 {
            self.pending_level = Some(FloatOutBoyBeeperLevel::High);
        }
    }

    pub(crate) fn force_on(&mut self) {
        if self.enabled {
            self.pending_level = Some(FloatOutBoyBeeperLevel::High);
        }
    }

    pub(crate) fn off(&mut self) {
        if self.transitions == 0 {
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
        if self.enabled && self.transitions != 0 {
            self.countdown = self.countdown.saturating_sub(1);
            if self.countdown == 0 {
                self.countdown = self.period;
                self.transitions = self.transitions.saturating_sub(1);
                self.pending_level = Some(if self.transitions & 1 == 1 {
                    FloatOutBoyBeeperLevel::High
                } else {
                    FloatOutBoyBeeperLevel::Low
                });
            }
        }

        self.take_level()
    }
}

#[cfg(test)]
mod tests;
