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
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(test))]
use vescpkg_rs::VescAllocator;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

static ALLOC_SMOKE_LEN: AtomicUsize = AtomicUsize::new(0);

vescpkg_rs::package_start!(crate::start);

/// Initialize the alloc smoke package.
pub fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();
    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(1).is_ok() {
        bytes.push(42);
    }
    ALLOC_SMOKE_LEN.store(bytes.len(), Ordering::Relaxed);
    core::mem::drop(bytes);
    true
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
    use super::{ALLOC_SMOKE_LEN, package_lib_init};
    use core::sync::atomic::Ordering;

    #[test]
    fn package_lib_init_uses_alloc_and_installs_stop_hook() {
        let mut info = vescpkg_rs::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(package_lib_init(&mut info));
        assert_eq!(ALLOC_SMOKE_LEN.load(Ordering::Relaxed), 1);
        assert!(info.stop_fun.is_some());
    }
}
