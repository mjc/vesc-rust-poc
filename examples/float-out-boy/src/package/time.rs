//! Float Out Boy package time helpers.
//!
//! C map: upstream `timer_older` lives in `third_party/float-out-boy/src/time.h:46-48`.

use vescpkg_rs::prelude::{SYSTEM_TICK_RATE_HZ, TimestampTicks, VescSeconds};

#[cfg(any(test, target_arch = "arm"))]
#[inline]
pub(super) fn float_out_boy_expire_timer(now: TimestampTicks, seconds: u32) -> TimestampTicks {
    TimestampTicks::from_ticks(now.as_ticks().wrapping_sub(
        seconds.saturating_mul(crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ)),
    ))
}

pub(super) fn float_out_boy_ticks_elapsed(
    now: TimestampTicks,
    then: TimestampTicks,
    seconds: u32,
) -> bool {
    // C map: `timer_older` uses a strict `>` comparison against
    // `SYSTEM_TICK_RATE_HZ` ticks per second at `third_party/float-out-boy/src/time.h:46-48`.
    now.wrapping_duration_since(then).as_ticks()
        > seconds.saturating_mul(crate::wire::truncating_u64_to_u32(SYSTEM_TICK_RATE_HZ))
}

pub(super) fn float_out_boy_ticks_elapsed_seconds(
    now: TimestampTicks,
    then: TimestampTicks,
    seconds: VescSeconds,
) -> bool {
    // C map: `timer_older` casts seconds times `SYSTEM_TICK_RATE_HZ` to the
    // integer tick type before strict comparison at `third_party/float-out-boy/src/time.h:46-48`.
    seconds
        .to_system_ticks_saturating()
        .is_some_and(|timeout| now.wrapping_duration_since(then) > timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expire_timer_subtracts_seconds_in_vesc_system_ticks_with_wraparound() {
        assert_eq!(
            float_out_boy_expire_timer(TimestampTicks::from_ticks(1_000_000), 60),
            TimestampTicks::from_ticks(400_000)
        );
        assert_eq!(
            float_out_boy_expire_timer(TimestampTicks::from_ticks(0), 60),
            TimestampTicks::from_ticks(0_u32.wrapping_sub(600_000))
        );
    }
}
