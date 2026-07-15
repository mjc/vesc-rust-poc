//! Refloat package time helpers.
//!
//! C map: upstream `timer_older` lives in `third_party/refloat/src/time.h:46-48`.

use vescpkg_rs::prelude::{SYSTEM_TICK_RATE_HZ, TimestampTicks, VescSeconds};

pub(super) fn refloat_ticks_elapsed(
    now: TimestampTicks,
    then: TimestampTicks,
    seconds: u32,
) -> bool {
    // C map: `timer_older` uses a strict `>` comparison against
    // `SYSTEM_TICK_RATE_HZ` ticks per second at `third_party/refloat/src/time.h:46-48`.
    now.wrapping_duration_since(then).as_ticks()
        > seconds.saturating_mul(SYSTEM_TICK_RATE_HZ as u32)
}

pub(super) fn refloat_ticks_elapsed_seconds(
    now: TimestampTicks,
    then: TimestampTicks,
    seconds: VescSeconds,
) -> bool {
    // C map: `timer_older` casts seconds times `SYSTEM_TICK_RATE_HZ` to the
    // integer tick type before strict comparison at `third_party/refloat/src/time.h:46-48`.
    now.wrapping_duration_since(then).as_ticks()
        > (seconds.as_seconds() * SYSTEM_TICK_RATE_HZ as f32) as u32
}
