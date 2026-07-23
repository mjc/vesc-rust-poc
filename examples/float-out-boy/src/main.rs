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
    fn package_lib_init_runs_float_out_boy_start() {
        assert!(super::package_lib_init(core::ptr::null_mut::<LoaderInfo>()));
    }
}
