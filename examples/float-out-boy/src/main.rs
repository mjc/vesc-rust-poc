//! Float Out Boy VESC package payload.
//!
//! This crate owns Float Out Boy-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds stay `no_std`; startup state is allocated directly by firmware.
//!
//! Source map: package initialization mirrors Float Out Boy's `start`/`stop` wiring at
//! `third_party/float-out-boy/src/main.c:2401-2460`.

#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]
#![deny(warnings, clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]
#![forbid(unused_extern_crates)]
// An embedded package cannot unwind or print a useful panic report. Keep
// explicit crash shortcuts out of the production entrypoint and its modules.
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unwrap_used
    )
)]

#[cfg(any(test, not(target_arch = "arm")))]
extern crate std;

#[cfg(test)]
macro_rules! assert_f32_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual: f32 = $actual;
        let expected: f32 = $expected;
        let tolerance = f32::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 4.0;
        let exactly_equal = !actual.is_nan() && actual.to_bits() == expected.to_bits();
        assert!(
            exactly_equal
                || (actual.is_finite()
                    && expected.is_finite()
                    && (actual - expected).abs() <= tolerance),
            "expected {expected:?}, got {actual:?} (tolerance {tolerance:?})"
        );
    }};
}

#[cfg(test)]
macro_rules! assert_f32_ne {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual: f32 = $actual;
        let expected: f32 = $expected;
        let tolerance = f32::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 4.0;
        let exactly_equal = !actual.is_nan() && actual.to_bits() == expected.to_bits();
        assert!(
            !exactly_equal
                && (!actual.is_finite()
                    || !expected.is_finite()
                    || (actual - expected).abs() > tolerance),
            "expected values to differ by more than {tolerance:?}, both were near {actual:?}"
        );
    }};
}

#[cfg(not(target_arch = "arm"))]
fn main() {}

#[cfg(all(not(test), not(target_arch = "arm")))]
#[global_allocator]
static HOST_ALLOCATOR: std::alloc::System = std::alloc::System;

mod balance;
mod beeper;
pub mod bms;
mod config;
pub mod domain;
pub mod extensions;
pub mod footpad;
pub mod lcm;
pub mod leds;
mod motor_control;
pub mod package;
mod wire;

vescpkg_rs::package_start!(
    crate::package::start,
    crate::package::FloatOutBoyPackageState
);

#[cfg(test)]
mod tests {
    mod package_author;

    use vescpkg_rs::test_support::LoaderInfo;

    #[test]
    fn float_assertions_handle_non_finite_values_without_nan_arithmetic() {
        assert_f32_eq!(f32::INFINITY, f32::INFINITY);
        assert_f32_eq!(f32::NEG_INFINITY, f32::NEG_INFINITY);
        assert_f32_eq!(0.0, -0.0);
        assert_f32_ne!(f32::INFINITY, f32::NEG_INFINITY);
        assert_f32_ne!(f32::NAN, f32::NAN);
    }

    #[test]
    fn package_lib_init_runs_float_out_boy_start() {
        assert!(super::package_lib_init(core::ptr::null_mut::<LoaderInfo>()));
    }
}
