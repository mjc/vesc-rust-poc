//! Minimal target package proving Rust `alloc` can use the VESC allocator.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::vec::Vec;
#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use vescpkg_rs::VescAllocator;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

vescpkg_rs::package_start!(crate::start);

/// Initialize the alloc smoke package.
pub fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();
    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(1).is_err() {
        return false;
    }

    bytes.push(42);
    let slot = bytes.as_mut_ptr();
    // SAFETY: `push` made the first byte initialized and valid until `bytes` is dropped.
    unsafe {
        core::ptr::write_volatile(slot, 42);
        core::ptr::read_volatile(slot) == 42
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::package_lib_init;

    #[test]
    fn package_lib_init_uses_alloc_and_installs_stop_hook() {
        let mut info = vescpkg_rs::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(package_lib_init(&mut info));
        assert!(info.stop_fun.is_some());
    }
}
