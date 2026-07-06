//! Target package proving ordinary Rust `alloc` use can run on the VESC allocator.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::boxed::Box;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::string::String;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::vec::Vec;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use core::alloc::Layout;
#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use vescpkg_rs::VescAllocator;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

vescpkg_rs::package_start!(crate::start);

#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_TEXT: &str = "vesc-alloc";
#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_PING_REQUEST: &[u8] = b"alloc-ping?";
#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_PONG: &[u8] = b"alloc-pong";
#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_PROBE_REQUEST: &[u8] = b"alloc-smoke?";
#[cfg(all(not(test), target_arch = "arm"))]
const ALLOC_SMOKE_PROBE_OK: &[u8] = b"alloc-ok";
#[cfg(all(not(test), target_arch = "arm"))]
const ALLOC_SMOKE_PROBE_FAIL: &[u8] = b"alloc-fail";

/// Initialize the alloc smoke package.
pub fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    if !start.install_stop_hook() {
        return false;
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let Some(info) = start.loader_info_mut() else {
            return false;
        };
        if unsafe {
            !vescpkg_rs::ffi::raw::vesc_set_app_data_handler(alloc_smoke_app_data_handler(info))
        } {
            return false;
        }
    }

    true
}

#[cfg(all(not(test), target_arch = "arm"))]
fn alloc_smoke_app_data_handler(
    info: &vescpkg_rs::ffi::LibInfo,
) -> vescpkg_rs::ffi::AppDataHandler {
    let handler_addr = vescpkg_rs::ffi::NativeImage::from_info(info)
        .rebase_addr(alloc_smoke_app_data_callback as *const () as usize);
    unsafe { core::mem::transmute::<usize, vescpkg_rs::ffi::AppDataHandler>(handler_addr) }
}

#[cfg(all(not(test), target_arch = "arm"))]
fn send_app_data(bytes: &[u8]) {
    unsafe { vescpkg_rs::ffi::raw::vesc_send_app_data(bytes.as_ptr(), bytes.len() as u32) };
}

/// App-data callback that lets host tooling prove post-start Rust allocation.
///
/// # Safety
///
/// `data` must be null with `len == 0` or point to `len` readable bytes that
/// stay valid for this call.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn alloc_smoke_app_data_callback(data: *mut u8, len: u32) {
    let Some(packet) = vescpkg_rs::app_data_packet(data, len) else {
        return;
    };

    match classify_alloc_smoke_app_data(packet.0) {
        AllocSmokeAppDataAction::Ignore => {}
        AllocSmokeAppDataAction::Reply(response) => send_app_data(response),
        AllocSmokeAppDataAction::RequestAlloc => {
            let response = if exercise_alloc() {
                ALLOC_SMOKE_PROBE_OK
            } else {
                ALLOC_SMOKE_PROBE_FAIL
            };
            send_app_data(response);
        }
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn exercise_alloc() -> bool {
    let Some(boxed) = fallible_boxed_u32(0x51) else {
        return false;
    };
    if *boxed != 0x51 {
        return false;
    }

    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(32).is_err() {
        return false;
    }
    for value in 0_u8..32 {
        bytes.push(value);
    }

    let mut text = String::new();
    if text.try_reserve_exact(ALLOC_SMOKE_TEXT.len()).is_err() {
        return false;
    }
    text.push_str("vesc");
    text.push('-');
    text.push_str("alloc");

    let mut zeroes = Vec::new();
    if zeroes.try_reserve_exact(16).is_err() {
        return false;
    }
    zeroes.resize(16, 0_u8);
    if zeroes.iter().any(|byte| *byte != 0) {
        return false;
    }
    let Some(last_zero) = zeroes.last_mut() else {
        return false;
    };
    *last_zero = 7;

    bytes.first() == Some(&0)
        && bytes.last() == Some(&31)
        && text == ALLOC_SMOKE_TEXT
        && zeroes.last() == Some(&7)
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn fallible_boxed_u32(value: u32) -> Option<Box<u32>> {
    let ptr = unsafe { alloc::alloc::alloc(Layout::new::<u32>()) }.cast::<u32>();
    let ptr = core::ptr::NonNull::new(ptr)?;
    unsafe {
        ptr.as_ptr().write(value);
        Some(Box::from_raw(ptr.as_ptr()))
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocSmokeAppDataAction {
    Ignore,
    Reply(&'static [u8]),
    RequestAlloc,
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn classify_alloc_smoke_app_data(packet: &[u8]) -> AllocSmokeAppDataAction {
    if packet == ALLOC_SMOKE_PING_REQUEST {
        return AllocSmokeAppDataAction::Reply(ALLOC_SMOKE_PONG);
    }

    if packet == ALLOC_SMOKE_PROBE_REQUEST {
        return AllocSmokeAppDataAction::RequestAlloc;
    }

    AllocSmokeAppDataAction::Ignore
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
        ALLOC_SMOKE_PING_REQUEST, ALLOC_SMOKE_PONG, ALLOC_SMOKE_PROBE_REQUEST,
        AllocSmokeAppDataAction, classify_alloc_smoke_app_data, exercise_alloc, package_lib_init,
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
    fn alloc_smoke_app_data_ping_replies_without_allocating() {
        assert_eq!(
            classify_alloc_smoke_app_data(ALLOC_SMOKE_PING_REQUEST),
            AllocSmokeAppDataAction::Reply(ALLOC_SMOKE_PONG)
        );
    }

    #[test]
    fn alloc_smoke_app_data_probe_runs_alloc_on_request() {
        assert_eq!(
            classify_alloc_smoke_app_data(ALLOC_SMOKE_PROBE_REQUEST),
            AllocSmokeAppDataAction::RequestAlloc
        );
    }

    #[test]
    fn alloc_smoke_app_data_ignores_unrelated_requests() {
        assert_eq!(
            classify_alloc_smoke_app_data(b"hello?"),
            AllocSmokeAppDataAction::Ignore
        );
    }
}
