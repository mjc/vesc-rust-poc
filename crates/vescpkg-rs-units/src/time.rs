//! Time aliases for VESC's 10 kHz system tick clock and VESC float seconds.
//!
//! Prefer [`SystemTicks`] and [`SystemInstant`] for normal package time arithmetic.
//! [`VescSeconds`] models firmware APIs that expose durations as `f32` seconds.

use crate::{scalar_int_unit, scalar_unit};

/// VESC system tick rate: 100 us resolution.
pub const SYSTEM_TICK_RATE_HZ: u64 = 10_000;

/// Duration measured in VESC 100 us system ticks.
pub type SystemTicks = fugit::TimerDurationU32<SYSTEM_TICK_RATE_HZ>;

/// Instant measured in VESC 100 us system ticks.
pub type SystemInstant = fugit::TimerInstantU32<SYSTEM_TICK_RATE_HZ>;

scalar_int_unit!(TimestampTicks, from_ticks, as_ticks, u32, "system ticks");
scalar_unit!(VescSeconds, from_seconds, as_seconds, "VESC float seconds");
scalar_unit!(Frequency, from_hertz, as_hertz, "hertz");
scalar_unit!(SampleRate, from_hertz, as_hertz, "hertz");

impl SampleRate {
    /// Return the duration of one sample at this rate.
    pub fn sample_period(self) -> Option<VescSeconds> {
        let seconds = 1.0 / self.as_hertz();
        (seconds.is_finite() && seconds > 0.0).then(|| VescSeconds::from_seconds(seconds))
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn system_ticks_as_secs_f32(ticks: SystemTicks) -> f32 {
    ticks.as_ticks() as f32 / SYSTEM_TICK_RATE_HZ as f32
}
