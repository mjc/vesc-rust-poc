use vescpkg_rs::prelude::{Frequency, Ratio, Rpm, SampleRate, VescSeconds};

// The package schema caps loop frequency at 4 kHz; the 8 Hz source formula
// needs 221 samples there. One spare slot keeps the fixed storage bounded
// without importing the branch's full u8 address space into package state.
const MAX_WINDOW: usize = 222;
const MAX_WINDOW_U32: u32 = 222;
const ABS_ERPM_CUTOFF: Frequency = Frequency::from_hertz(10.0);
const ACCELERATION_CUTOFF_HZ: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MotorKinematicsTracker(vescpkg_rs::MotorKinematics<MAX_WINDOW>);

#[cfg(target_arch = "arm")]
const _: [(); 912] = [(); core::mem::size_of::<MotorKinematicsTracker>()];

impl Default for MotorKinematicsTracker {
    fn default() -> Self {
        let mut tracker = Self(vescpkg_rs::MotorKinematics::default());
        tracker.configure(SampleRate::from_hertz(832.0));
        tracker
    }
}

impl MotorKinematicsTracker {
    pub(super) fn configure(&mut self, sample_rate: SampleRate) {
        let smoothing = Ratio::clamped(vescpkg_rs::ema_alpha(ABS_ERPM_CUTOFF, sample_rate));
        let normalized_cutoff = ACCELERATION_CUTOFF_HZ / sample_rate.as_hertz();
        let window = usize::try_from(
            crate::wire::saturating_trunc_f32_to_u32(
                vescpkg_rs::sqrt(0.196_202 + normalized_cutoff * normalized_cutoff)
                    / normalized_cutoff,
            )
            .min(MAX_WINDOW_U32),
        )
        .unwrap_or(MAX_WINDOW);
        self.0.configure(smoothing, window);
    }

    pub(super) fn record(&mut self, motor_erpm: Rpm, elapsed: VescSeconds) {
        self.0.record(motor_erpm, elapsed);
    }

    pub(super) fn reset_acceleration(&mut self) {
        self.0.reset_acceleration();
    }

    pub(super) const fn average(&self) -> Rpm {
        self.0.average()
    }

    pub(super) const fn smoothed_abs_erpm(&self) -> Rpm {
        self.0.smoothed_abs_erpm()
    }
}

#[cfg(test)]
mod tests;
