use vescpkg_rs::prelude::{Frequency, Ratio, SampleRate, VescSeconds};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(crate) struct EmaAlpha(Ratio);

impl EmaAlpha {
    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_elapsed(cutoff: Frequency, elapsed: VescSeconds) -> Self {
        Self::from_omega(core::f32::consts::TAU * cutoff.as_hertz() * elapsed.as_seconds())
    }

    #[must_use]
    pub(crate) fn from_sample_rate(cutoff: Frequency, sample_rate: SampleRate) -> Self {
        Self::from_omega(core::f32::consts::TAU * cutoff.as_hertz() / sample_rate.as_hertz())
    }

    #[must_use]
    pub(crate) fn from_time_constant(time_constant: VescSeconds, sample_rate: SampleRate) -> Self {
        Self::from_omega(1.0 / (time_constant.as_seconds() * sample_rate.as_hertz()))
    }

    fn from_omega(omega: f32) -> Self {
        let bounded = omega.min(0.5);
        Self(Ratio::clamped(bounded - 0.5 * bounded * bounded))
    }

    #[must_use]
    pub(crate) const fn scaled(self, factor: f32) -> Self {
        Self(Ratio::clamped(self.factor() * factor))
    }

    #[must_use]
    pub(crate) const fn factor(self) -> f32 {
        self.0.as_ratio()
    }

    #[must_use]
    pub(crate) const fn retained(self) -> f32 {
        self.0.complement().as_ratio()
    }

    #[must_use]
    pub(crate) const fn is_zero(self) -> bool {
        self.0.is_zero()
    }
}

#[cfg(test)]
mod tests;
