use vescpkg_rs::prelude::{Frequency, Ratio, SampleRate, TimestampTicks};

pub(super) type FrequencyTracker = vescpkg_rs::SampleRateTracker;

pub(super) const TRACKING_POLICY: vescpkg_rs::SampleRateTrackingPolicy =
    vescpkg_rs::SampleRateTrackingPolicy::new(
        Frequency::from_hertz(1.0),
        1,
        Ratio::from_ratio_const(0.03),
    );

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FrequencyTrackers {
    pub(super) main: FrequencyTracker,
    pub(super) imu: FrequencyTracker,
}

impl Default for FrequencyTrackers {
    fn default() -> Self {
        let epoch = TimestampTicks::from_ticks(0);
        Self {
            main: FrequencyTracker::new(
                crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE,
                epoch,
                TRACKING_POLICY,
            ),
            imu: FrequencyTracker::new(SampleRate::from_hertz(620.0), epoch, TRACKING_POLICY),
        }
    }
}

#[cfg(any(test, target_arch = "arm"))]
pub(super) fn imu_start_frequency(configured: SampleRate) -> SampleRate {
    if configured.as_hertz() == 0.0 {
        // Refloat uses 620 Hz as an intentionally approximate seed on VESC
        // firmware 6.02, whose IMU sample-rate setting slot returns zero.
        SampleRate::from_hertz(620.0)
    } else {
        configured
    }
}
