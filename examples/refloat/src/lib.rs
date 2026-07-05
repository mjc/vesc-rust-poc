//! Refloat VESC package payload.
//!
//! This crate owns Refloat-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

pub mod app_data;
mod balance_loop;
mod config;
pub mod domain;
pub mod extensions;
pub mod init;
mod motor_control;
pub mod runtime;
mod state_transition;

pub use init::package_lib_init;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod package_author_tests;
