//! Float math entrypoints for native package code.
//!
//! VESC firmware links packages with the C/newlib math library. ARM package
//! builds call those symbols directly; host builds use Rust `libm` so ordinary
//! tests do not depend on an embedded linker.

#[cfg(target_arch = "arm")]
unsafe extern "C" {
    #[link_name = "asinf"]
    fn c_asinf(x: f32) -> f32;
    #[link_name = "cosf"]
    fn c_cosf(x: f32) -> f32;
    #[link_name = "sinf"]
    fn c_sinf(x: f32) -> f32;
    #[link_name = "sqrtf"]
    fn c_sqrtf(x: f32) -> f32;
}

/// Computes arcsine for a single-precision input.
#[cfg(target_arch = "arm")]
#[inline(always)]
#[must_use]
pub fn asin(x: f32) -> f32 {
    // SAFETY: the package final link resolves this symbol from the VESC C math
    // environment. `asinf` has the standard C ABI signature `float(float)`.
    unsafe { c_asinf(x) }
}

/// Computes arcsine for a single-precision input.
#[cfg(not(target_arch = "arm"))]
#[inline(always)]
#[must_use]
pub fn asin(x: f32) -> f32 {
    libm::asinf(x)
}

/// Computes cosine for a single-precision input.
#[cfg(target_arch = "arm")]
#[inline(always)]
#[must_use]
pub fn cos(x: f32) -> f32 {
    // SAFETY: see `asinf`.
    unsafe { c_cosf(x) }
}

/// Computes cosine for a single-precision input.
#[cfg(not(target_arch = "arm"))]
#[inline(always)]
#[must_use]
pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

/// Computes sine for a single-precision input.
#[cfg(target_arch = "arm")]
#[inline(always)]
#[must_use]
pub fn sin(x: f32) -> f32 {
    // SAFETY: see `asinf`.
    unsafe { c_sinf(x) }
}

/// Computes sine for a single-precision input.
#[cfg(not(target_arch = "arm"))]
#[inline(always)]
#[must_use]
pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

/// Computes square root for a single-precision input.
#[cfg(target_arch = "arm")]
#[inline(always)]
#[must_use]
pub fn sqrt(x: f32) -> f32 {
    // SAFETY: see `asinf`.
    unsafe { c_sqrtf(x) }
}

/// Computes square root for a single-precision input.
#[cfg(not(target_arch = "arm"))]
#[inline(always)]
#[must_use]
pub fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

#[cfg(test)]
mod tests {
    use super::{asin, cos, sin, sqrt};

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
