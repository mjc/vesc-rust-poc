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

mod balance;
mod config;
pub mod domain;
pub mod extensions;
mod motor_control;
pub mod package;

vescpkg_rs::package_start!(crate::package::start);

#[cfg(test)]
mod tests {
    mod package_author;

    use vescpkg_rs::ffi;

    #[test]
    fn package_lib_init_runs_refloat_start() {
        assert!(super::package_lib_init(
            core::ptr::null_mut::<ffi::LibInfo>()
        ));
    }
}

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
