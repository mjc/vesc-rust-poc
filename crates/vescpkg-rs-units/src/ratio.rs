//! Unitless ratio and percent newtypes.

use crate::bounded_unit;

bounded_unit!(
    Ratio,
    from_ratio,
    from_ratio_const,
    as_ratio,
    0.0,
    1.0,
    "normalized ratio"
);

impl Ratio {
    /// Return the remaining fraction to one.
    #[must_use]
    pub const fn complement(self) -> Self {
        Self(1.0 - self.0)
    }

    /// Return the smaller ratio.
    #[must_use]
    pub fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    /// Return the larger ratio.
    #[must_use]
    pub fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }

    /// Linearly interpolate toward another ratio and clamp the result.
    #[must_use]
    pub fn lerp(self, target: Self, progress: f32) -> Self {
        Self::clamped(self.0 + (target.0 - self.0) * progress)
    }

    /// Return whether this ratio is zero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 <= Self::MIN
    }

    /// Return whether this ratio is one.
    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0 >= Self::MAX
    }
}

impl core::ops::Mul for Ratio {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
bounded_unit!(
    SignedRatio,
    from_ratio,
    from_ratio_const,
    as_ratio,
    -1.0,
    1.0,
    "signed normalized ratio"
);
bounded_unit!(
    Percent,
    from_percent,
    from_percent_const,
    as_percent,
    0.0,
    100.0,
    "percent"
);
