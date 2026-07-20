//! Float math entrypoints for native package code.
//!
//! Rust `libm` keeps package math position-independent on both firmware and host.

/// Computes arcsine for a single-precision input.
#[inline(always)]
#[must_use]
pub fn asin(x: f32) -> f32 {
    libm::asinf(x)
}

/// Computes cosine for a single-precision input.
#[inline(always)]
#[must_use]
pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

/// Computes sine for a single-precision input.
#[inline(always)]
#[must_use]
pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

/// Computes tangent for a single-precision input.
#[inline(always)]
#[must_use]
pub fn tan(x: f32) -> f32 {
    libm::tanf(x)
}

/// Computes square root for a single-precision input.
#[inline(always)]
#[must_use]
pub fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

#[cfg(test)]
mod tests {
    use super::{asin, cos, sin, sqrt, tan};

    fn approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() <= 1.0e-6,
            "left={left:?} right={right:?}"
        );
    }

    #[test]
    fn sinf_matches_host_libm_for_representative_angles() {
        for value in [
            0.0,
            core::f32::consts::FRAC_PI_6,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            -core::f32::consts::FRAC_PI_3,
            1.75,
        ] {
            approx_eq(sin(value), libm::sinf(value));
        }
    }

    #[test]
    fn cosf_matches_host_libm_for_representative_angles() {
        for value in [
            0.0,
            core::f32::consts::FRAC_PI_6,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            -core::f32::consts::FRAC_PI_3,
            1.75,
        ] {
            approx_eq(cos(value), libm::cosf(value));
        }
    }

    #[test]
    fn tanf_matches_host_libm_for_representative_angles() {
        for value in [0.0, core::f32::consts::FRAC_PI_6, -0.5, 0.75] {
            approx_eq(tan(value), libm::tanf(value));
        }
    }

    #[test]
    fn asinf_matches_host_libm_for_boundary_inputs() {
        for value in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            approx_eq(asin(value), libm::asinf(value));
        }
    }

    #[test]
    fn sqrtf_matches_host_libm_for_positive_inputs() {
        for value in [0.0, 0.5, 1.0, 2.0, 9.0, 123.456] {
            approx_eq(sqrt(value), libm::sqrtf(value));
        }
    }

    #[test]
    fn trig_wrappers_preserve_expected_symmetry() {
        for value in [
            core::f32::consts::FRAC_PI_8,
            core::f32::consts::FRAC_PI_6,
            core::f32::consts::FRAC_PI_4,
        ] {
            approx_eq(sin(-value), -sin(value));
            approx_eq(cos(-value), cos(value));
        }
    }

    #[test]
    fn asinf_preserves_boundary_signs() {
        assert!(asin(-1.0).is_sign_negative());
        approx_eq(asin(0.0), 0.0);
        assert!(asin(1.0).is_sign_positive());
    }

    #[test]
    fn sqrtf_reports_nan_for_negative_inputs() {
        assert!(sqrt(-1.0).is_nan());
    }
}
