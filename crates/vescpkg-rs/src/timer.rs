//! Wrapping VESC system-tick timer operations.

use core::num::NonZeroU32;
use core::time::Duration;

use crate::{SYSTEM_TICK_RATE_HZ, SampleRate, SystemTicks, TimestampTicks, VescSeconds};

/// System-tick timing for a fixed-rate package loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRateLoopTiming {
    nominal_ticks: NonZeroU32,
}

impl FixedRateLoopTiming {
    /// Build loop timing from a requested sample rate.
    #[must_use]
    pub fn from_sample_rate(sample_rate: SampleRate) -> Self {
        let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).map_or(f32::NAN, f32::from);
        let ticks =
            crate::protocol_buffer::saturating_trunc_f32_to_u32(tick_rate / sample_rate.as_hertz())
                .max(1);
        Self {
            nominal_ticks: NonZeroU32::new(ticks).unwrap_or(NonZeroU32::MIN),
        }
    }

    /// Return the nominal sleep duration before accounting for loop work.
    #[must_use]
    pub fn nominal_sleep(self) -> Duration {
        ticks_to_duration(self.nominal_ticks.get())
    }

    /// Return the remaining sleep after rounding work time to system ticks.
    #[must_use]
    pub fn sleep_after_work(self, elapsed: VescSeconds) -> Duration {
        let elapsed = elapsed.as_seconds();
        let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).map_or(f32::NAN, f32::from);
        if !elapsed.is_finite() || elapsed < 0.0 {
            return self.nominal_sleep();
        }
        let work_ticks =
            crate::protocol_buffer::saturating_trunc_f32_to_u32(elapsed * tick_rate + 0.5);
        ticks_to_duration(self.nominal_ticks.get().saturating_sub(work_ticks).max(1))
    }
}

fn ticks_to_duration(ticks: u32) -> Duration {
    Duration::from_micros(u64::from(ticks).saturating_mul(1_000_000) / SYSTEM_TICK_RATE_HZ)
}

/// A restartable timer backed by the wrapping VESC system clock.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct WrappingTimer(TimestampTicks);

impl WrappingTimer {
    /// Start a timer at one VESC system-clock timestamp.
    #[must_use]
    pub const fn started_at(started: TimestampTicks) -> Self {
        Self(started)
    }

    /// Return when this timer was last started.
    #[must_use]
    pub const fn started(self) -> TimestampTicks {
        self.0
    }

    /// Return the wrapping elapsed system ticks at one timestamp.
    #[must_use]
    pub const fn elapsed(self, now: TimestampTicks) -> SystemTicks {
        now.wrapping_duration_since(self.0)
    }

    /// Restart this timer at the supplied timestamp.
    pub fn restart(&mut self, now: TimestampTicks) {
        self.0 = now;
    }

    /// Return whether this timer is strictly older than a typed duration.
    #[must_use]
    pub fn older_than(self, now: TimestampTicks, duration: VescSeconds) -> bool {
        timer_older(now, self.0, duration)
    }

    /// Return whether this timer is strictly older than whole seconds.
    #[must_use]
    pub fn older_than_secs(self, now: TimestampTicks, seconds: u32) -> bool {
        timer_older_whole_seconds(now, self.0, seconds)
    }

    /// Force this timer to be a whole duration in the past.
    pub fn expire_whole_seconds(&mut self, now: TimestampTicks, seconds: u32) {
        self.0 = expire_timer_whole_seconds(now, seconds);
    }
}

/// Move a timer into the past by a whole number of seconds, with firmware wrapping.
#[must_use]
pub fn expire_timer_whole_seconds(now: TimestampTicks, seconds: u32) -> TimestampTicks {
    let ticks_per_second = u32::try_from(SYSTEM_TICK_RATE_HZ).unwrap_or(u32::MAX);
    TimestampTicks::from_ticks(
        now.as_ticks()
            .wrapping_sub(seconds.saturating_mul(ticks_per_second)),
    )
}

/// Return whether a whole-second timer is strictly older than its duration.
#[must_use]
pub fn timer_older_whole_seconds(now: TimestampTicks, then: TimestampTicks, seconds: u32) -> bool {
    let ticks_per_second = u32::try_from(SYSTEM_TICK_RATE_HZ).unwrap_or(u32::MAX);
    now.wrapping_duration_since(then).as_ticks() > seconds.saturating_mul(ticks_per_second)
}

/// Return whether a typed timer is strictly older than its firmware duration.
#[must_use]
pub fn timer_older(now: TimestampTicks, then: TimestampTicks, duration: VescSeconds) -> bool {
    let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).unwrap_or(u16::MAX);
    let duration_ticks = crate::protocol_buffer::saturating_trunc_f32_to_u32(
        duration.as_seconds() * f32::from(tick_rate),
    );
    now.wrapping_duration_since(then).as_ticks() > duration_ticks
}

#[cfg(test)]
mod tests {
    use super::{
        FixedRateLoopTiming, WrappingTimer, expire_timer_whole_seconds, timer_older,
        timer_older_whole_seconds,
    };
    use crate::{SampleRate, TimestampTicks, VescSeconds};
    use core::time::Duration;

    #[test]
    fn fixed_rate_loop_timing_rounds_work_to_system_ticks() {
        let timing = FixedRateLoopTiming::from_sample_rate(SampleRate::from_hertz(832.0));

        assert_eq!(timing.nominal_sleep(), Duration::from_micros(1_200));
        assert_eq!(
            timing.sleep_after_work(VescSeconds::from_seconds(0.000_31)),
            Duration::from_micros(900),
        );
    }

    #[test]
    fn fixed_rate_loop_timing_retains_one_tick_after_delays() {
        let timing = FixedRateLoopTiming::from_sample_rate(SampleRate::from_hertz(500.0));

        assert_eq!(
            timing.sleep_after_work(VescSeconds::from_seconds(0.025)),
            Duration::from_micros(100),
        );
        assert_eq!(
            timing.sleep_after_work(VescSeconds::from_seconds(f32::NAN)),
            timing.nominal_sleep(),
        );
    }

    #[test]
    fn whole_second_timers_preserve_strict_boundaries_and_wrapping() {
        let then = TimestampTicks::from_ticks(u32::MAX - 5_000);
        assert!(!timer_older_whole_seconds(
            TimestampTicks::from_ticks(4_999),
            then,
            1,
        ));
        assert!(timer_older_whole_seconds(
            TimestampTicks::from_ticks(5_000),
            then,
            1,
        ));
        assert_eq!(
            expire_timer_whole_seconds(TimestampTicks::from_ticks(0), 60),
            TimestampTicks::from_ticks(0_u32.wrapping_sub(600_000)),
        );
    }

    #[test]
    fn typed_duration_timer_truncates_to_firmware_ticks_before_comparing() {
        let then = TimestampTicks::from_ticks(100);
        let duration = VescSeconds::from_seconds(0.000_19);

        assert!(!timer_older(
            TimestampTicks::from_ticks(101),
            then,
            duration
        ));
        assert!(timer_older(TimestampTicks::from_ticks(102), then, duration));
    }

    #[test]
    fn wrapping_timer_is_timestamp_sized_and_owns_restart_and_expiry() {
        assert_eq!(
            core::mem::size_of::<WrappingTimer>(),
            core::mem::size_of::<TimestampTicks>(),
        );

        let mut timer = WrappingTimer::started_at(TimestampTicks::from_ticks(u32::MAX - 5_000));
        assert!(!timer.older_than_secs(TimestampTicks::from_ticks(4_999), 1));
        assert!(timer.older_than_secs(TimestampTicks::from_ticks(5_000), 1));

        timer.restart(TimestampTicks::from_ticks(42));
        assert_eq!(timer.started(), TimestampTicks::from_ticks(42));
        timer.expire_whole_seconds(TimestampTicks::from_ticks(0), 60);
        assert_eq!(
            timer.started(),
            TimestampTicks::from_ticks(0_u32.wrapping_sub(600_000)),
        );
    }
}
