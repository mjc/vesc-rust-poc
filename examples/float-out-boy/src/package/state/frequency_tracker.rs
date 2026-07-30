use vescpkg_rs::prelude::{Frequency, SampleRate, TimestampTicks, VescSeconds};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::timer_older_whole_seconds;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FrequencyTracker {
    elapsed: VescSeconds,
    frequency: SampleRate,
    filter_frequency: SampleRate,
    alpha: f32,
    filter_last_update: TimestampTicks,
    first: bool,
    running: bool,
    recalculations: u32,
}

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
            ),
            imu: FrequencyTracker::new(SampleRate::from_hertz(620.0), epoch),
        }
    }
}

impl FrequencyTracker {
    pub(super) fn new(frequency: SampleRate, now: TimestampTicks) -> Self {
        Self {
            elapsed: VescSeconds::from_seconds(0.0),
            frequency,
            filter_frequency: frequency,
            alpha: vescpkg_rs::ema_alpha(Frequency::from_hertz(1.0), frequency),
            filter_last_update: now,
            first: true,
            running: false,
            recalculations: 0,
        }
    }

    pub(super) fn update(&mut self, elapsed: VescSeconds) {
        self.elapsed = elapsed;
        let target = 1.0 / elapsed.as_seconds();
        self.frequency = SampleRate::from_hertz(
            self.frequency.as_hertz() + self.alpha * (target - self.frequency.as_hertz()),
        );
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn check(&mut self, running: bool, now: TimestampTicks) -> Option<SampleRate> {
        if !self.running && running {
            self.filter_last_update = now;
        }
        self.running = running;

        let change = (1.0 - self.frequency.as_hertz() / self.filter_frequency.as_hertz()).abs();
        if (running || self.first)
            && timer_older_whole_seconds(now, self.filter_last_update, 1)
            && change > 0.03
        {
            self.filter_frequency = self.frequency;
            self.alpha = vescpkg_rs::ema_alpha(Frequency::from_hertz(1.0), self.filter_frequency);
            self.filter_last_update = now;
            self.first = false;
            self.recalculations = self.recalculations.saturating_add(1);
            Some(self.filter_frequency)
        } else {
            None
        }
    }

    pub(super) const fn filter_frequency(self) -> SampleRate {
        self.filter_frequency
    }

    pub(super) const fn elapsed(self) -> VescSeconds {
        self.elapsed
    }

    pub(super) const fn frequency(self) -> SampleRate {
        self.frequency
    }

    #[cfg(test)]
    pub(super) const fn recalculations(self) -> u32 {
        self.recalculations
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
