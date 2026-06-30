//! Time aliases for VESC's 10 kHz system tick clock.

use crate::scalar_int_unit;

/// VESC system tick rate: 100 us resolution.
pub const SYSTEM_TICK_RATE_HZ: u64 = 10_000;

/// Duration measured in VESC 100 us system ticks.
pub type SystemTicks = fugit::TimerDurationU32<SYSTEM_TICK_RATE_HZ>;

/// Instant measured in VESC 100 us system ticks.
pub type SystemInstant = fugit::TimerInstantU32<SYSTEM_TICK_RATE_HZ>;

scalar_int_unit!(TimestampTicks, from_ticks, as_ticks, u32, "system ticks");
