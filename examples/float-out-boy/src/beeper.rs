//! Float Out Boy external-beeper sequencing.

pub(crate) use vescpkg_rs::DigitalOutputLevel as FloatOutBoyBeeperLevel;
use vescpkg_rs::{SYSTEM_TICK_RATE_HZ, TimestampTicks};

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

// Refloat main uses 0.05 and 0.25 seconds after `0274273c`.
const SHORT_BEEP_PERIOD: u32 = crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ / 20);
const LONG_BEEP_PERIOD: u32 = crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ / 4);

impl FloatOutBoyBeeperAlert {
    const fn sequence(self) -> (u8, u32) {
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
    period: u32,
    timer: TimestampTicks,
    now: TimestampTicks,
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
            timer: TimestampTicks::from_ticks(0),
            now: TimestampTicks::from_ticks(0),
            pending_level: None,
        }
    }

    pub(crate) fn alert(&mut self, alert: FloatOutBoyBeeperAlert) {
        if !self.enabled || self.transitions != 0 {
            return;
        }

        (self.transitions, self.period) = alert.sequence();
        self.timer = self.now;
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

    pub(crate) fn take_level(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        self.pending_level.take()
    }

    pub(crate) fn tick_at(&mut self, now: TimestampTicks) -> Option<FloatOutBoyBeeperLevel> {
        self.now = now;
        // C map: Refloat's `beeper_update` advances once and refreshes its
        // timer to `now`; a delayed loop therefore stretches the sequence.
        if self.enabled
            && self.transitions != 0
            && now.wrapping_duration_since(self.timer).as_ticks() > self.period
        {
            self.timer = now;
            self.transitions = self.transitions.saturating_sub(1);
            self.pending_level = Some(if self.transitions & 1 == 1 {
                FloatOutBoyBeeperLevel::High
            } else {
                FloatOutBoyBeeperLevel::Low
            });
        }

        self.take_level()
    }

    #[cfg(test)]
    pub(crate) fn tick(&mut self) -> Option<FloatOutBoyBeeperLevel> {
        let loop_ticks = crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ / 100);
        self.tick_at(TimestampTicks::from_ticks(
            self.now.as_ticks().wrapping_add(loop_ticks),
        ))
    }
}

#[cfg(test)]
mod tests;
