//! Target package proving ordinary Rust `alloc` use can run on the VESC allocator.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use core::ffi::CStr;
#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use vescpkg_rs::VescAllocator;
use vescpkg_rs::ffi;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

vescpkg_rs::package_start!(crate::start);

const ALLOC_SMOKE_TEXT: &str = "vesc-alloc";
#[cfg(any(test, all(not(test), target_arch = "arm")))]
const EXT_ALLOC_SMOKE_NAME: &CStr = c"ext-rust-alloc-smoke";

struct ExtAllocSmoke;

impl vescpkg_rs::LbmExtension for ExtAllocSmoke {
    fn call(args: vescpkg_rs::LbmExtensionArgs<'_>) -> ffi::LbmValue {
        if exercise_alloc() {
            args.true_value()
        } else {
            args.nil_value()
        }
    }
}

/// Device extension that reruns the alloc smoke after package startup.
///
/// # Safety
///
/// `args` must be null with `arg_count == 0` or point to `arg_count` LispBM
/// values that stay valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ext_rust_alloc_smoke(args: *mut u32, arg_count: u32) -> u32 {
    unsafe { vescpkg_rs::lbm_extension_handler::<ExtAllocSmoke>(args, arg_count) }
}

/// Initialize the alloc smoke package.
pub fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();
    #[cfg(all(not(test), target_arch = "arm"))]
    if !start.register_extensions(package_extension_descriptors()) {
        return false;
    }

    true
}

fn exercise_alloc() -> bool {
    let boxed = Box::new(0x51_u32);
    if *boxed != 0x51 {
        return false;
    }

    let mut bytes = Vec::new();
    for value in 0_u8..32 {
        if bytes.try_reserve_exact(1).is_err() {
            return false;
        }
        bytes.push(value);
    }

    let mut text = String::new();
    if text.try_reserve_exact(ALLOC_SMOKE_TEXT.len()).is_err() {
        return false;
    }
    text.push_str("vesc");
    text.push('-');
    text.push_str("alloc");

    let mut zeroes = alloc::vec![0_u8; 16];
    if zeroes.iter().any(|byte| *byte != 0) {
        return false;
    }
    zeroes[15] = 7;

    bytes.first() == Some(&0)
        && bytes.last() == Some(&31)
        && text == ALLOC_SMOKE_TEXT
        && zeroes[15] == 7
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn package_extension_descriptors() -> [ffi::ExtensionDescriptor; 1] {
    [ffi::ExtensionDescriptor::new(
        EXT_ALLOC_SMOKE_NAME,
        ext_rust_alloc_smoke,
    )]
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
    use super::{
        exercise_alloc, ext_rust_alloc_smoke, package_extension_descriptors, package_lib_init,
    };

    #[test]
    fn package_lib_init_installs_stop_hook() {
        let mut info = vescpkg_rs::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(package_lib_init(&mut info));
        assert!(info.stop_fun.is_some());
    }

    #[test]
    fn alloc_smoke_exercises_box_vec_growth_and_string() {
        assert!(exercise_alloc());
    }

    #[test]
    fn extension_descriptor_registers_the_alloc_smoke_callback() {
        let [descriptor] = package_extension_descriptors();

        assert_eq!(descriptor.name().to_bytes(), b"ext-rust-alloc-smoke");
    }

    #[test]
    fn alloc_smoke_extension_runs_alloc_after_startup() {
        assert_eq!(unsafe { ext_rust_alloc_smoke(core::ptr::null_mut(), 0) }, 1);
    }
}
