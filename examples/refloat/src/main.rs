//! Refloat VESC package payload.
//!
//! This crate owns Refloat-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds stay `no_std`; startup state is allocated directly by firmware.
//!
//! Source map: package initialization mirrors Refloat's `start`/`stop` wiring at
//! `third_party/refloat/src/main.c:2401-2460`.

#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]
#![forbid(unsafe_code)]
#![forbid(unused_extern_crates)]

#[cfg(any(test, not(target_arch = "arm")))]
extern crate std;

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

vescpkg_rs::package_start!(crate::package::start, crate::package::RefloatPackageState);

#[cfg(test)]
mod tests {
    mod package_author;

    use vescpkg_rs::test_support::LoaderInfo;

    #[test]
    fn package_lib_init_runs_refloat_start() {
        assert!(super::package_lib_init(core::ptr::null_mut::<LoaderInfo>()));
    }
}
