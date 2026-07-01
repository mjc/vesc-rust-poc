//! Time aliases for VESC's 10 kHz system tick clock.

use crate::{scalar_int_unit, scalar_unit};

/// VESC system tick rate: 100 us resolution.
pub const SYSTEM_TICK_RATE_HZ: u64 = 10_000;

/// Duration measured in VESC 100 us system ticks.
pub type SystemTicks = fugit::TimerDurationU32<SYSTEM_TICK_RATE_HZ>;

/// Instant measured in VESC 100 us system ticks.
pub type SystemInstant = fugit::TimerInstantU32<SYSTEM_TICK_RATE_HZ>;

scalar_int_unit!(TimestampTicks, from_ticks, as_ticks, u32, "system ticks");
scalar_unit!(Seconds, from_seconds, as_seconds, "seconds");
scalar_unit!(Frequency, from_hertz, as_hertz, "hertz");
scalar_unit!(SampleRate, from_hertz, as_hertz, "hertz");

#[allow(clippy::cast_precision_loss)]
pub(crate) fn system_ticks_as_secs_f32(ticks: SystemTicks) -> f32 {
    ticks.as_ticks() as f32 / SYSTEM_TICK_RATE_HZ as f32
}
