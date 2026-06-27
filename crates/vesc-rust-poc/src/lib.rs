//! BLE loopback proof-of-concept package payload.
//!
//! This crate is the linkable staticlib artifact (`libvesc_rust_poc.a`). All loader,
//! lifecycle, and firmware wrapper code lives in `vesc-package`.

#![cfg_attr(not(test), no_std)]

pub use vesc_package::*;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
