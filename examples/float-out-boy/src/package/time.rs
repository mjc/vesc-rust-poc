//! Float Out Boy package time helpers.
//!
//! C map: upstream `timer_older` lives in `third_party/float-out-boy/src/time.h:46-48`.

use vescpkg_rs::prelude::{SYSTEM_TICK_RATE_HZ, TimestampTicks, VescSeconds};

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
    let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).unwrap_or(u16::MAX);
    now.wrapping_duration_since(then).as_ticks()
        > crate::wire::saturating_trunc_f32_to_u32(seconds.as_seconds() * f32::from(tick_rate))
}
