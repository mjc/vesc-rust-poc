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
use vescpkg_rs::{ffi::LibInfo, init as pkg_init};

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

static ALLOC_SMOKE_LEN: AtomicUsize = AtomicUsize::new(0);

/// VESC loader anchor in `.program_ptr`; value is unused but the section must exist.
#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
static prog_ptr: u32 = 0;

/// Initialize the alloc smoke package.
///
/// `info` must be the VESC-provided package loader information pointer.
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut LibInfo) -> bool {
    let _ = pkg_init::install_stop_hook(info);
    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(1).is_ok() {
        bytes.push(42);
    }
    ALLOC_SMOKE_LEN.store(bytes.len(), Ordering::Relaxed);
    core::mem::drop(bytes);
    true
}

/// ARM package loader entrypoint placed in `.init_fun` for VESC firmware loading.
///
/// `info` must be the VESC-provided package loader information pointer.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut LibInfo) -> bool {
    package_lib_init(info)
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
    use vescpkg_rs::ffi::LibInfo;

    #[test]
    fn package_lib_init_uses_alloc_and_installs_stop_hook() {
        let mut info = LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(package_lib_init(&mut info));
        assert_eq!(ALLOC_SMOKE_LEN.load(Ordering::Relaxed), 1);
        assert!(info.stop_fun.is_some());
    }
}
