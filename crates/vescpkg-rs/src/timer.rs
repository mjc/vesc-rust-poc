//! Wrapping VESC system-tick timer operations.

use crate::{SYSTEM_TICK_RATE_HZ, TimestampTicks, VescSeconds};

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
    use super::{expire_timer_whole_seconds, timer_older, timer_older_whole_seconds};
    use crate::{TimestampTicks, VescSeconds};

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
}
