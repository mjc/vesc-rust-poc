//! Target package proving ordinary Rust `alloc` use can run on the VESC allocator.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::vec::Vec;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::VescAllocator;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vesc_protocol::ble_loopback::{LoopbackError, MAX_LOOPBACK_FRAME_BYTES, handle_loopback_frame};
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vesc_protocol::{WireCommand, WireVersion};

#[cfg(not(test))]
use vescpkg_rs::{ffi, init as pkg_init};

#[cfg(all(not(test), target_arch = "arm"))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".program_ptr")]
static prog_ptr: u32 = 0;

/// Package loader entrypoint that installs the stop hook.
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) -> bool {
    pkg_init::install_stop_hook(info)
}

/// Test-build package loader entrypoint.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn package_lib_init(info: *mut vescpkg_rs::ffi::LibInfo) -> bool {
    vescpkg_rs::init::install_stop_hook(info)
}

/// Firmware package entrypoint that registers the alloc-smoke app-data probe.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_fun")]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    if !package_lib_init(info) {
        return false;
    }
    unsafe { ffi::raw::vesc_set_app_data_handler(alloc_smoke_app_data_callback) }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_CANDIDATES: usize = 5;

/// Initialize the alloc smoke package.
#[cfg(all(not(test), target_arch = "arm"))]
unsafe extern "C" fn alloc_smoke_app_data_callback(data: *mut u8, len: u32) {
    if data.is_null() || len == 0 {
        return;
    }
    let bytes = unsafe { core::slice::from_raw_parts(data.cast_const(), len as usize) };
    if classify_alloc_smoke_app_data(bytes) == AllocSmokeAppDataAction::Ignore {
        return;
    }
    let now_ms = u64::from(unsafe { ffi::raw::vesc_system_time_ticks() }) / 10;
    let Ok((response, response_len)) = alloc_smoke_loopback(bytes, now_ms) else {
        return;
    };
    unsafe {
        ffi::raw::vesc_send_app_data(response.as_ptr().cast_mut(), response_len as u32);
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn alloc_smoke_loopback(
    packet: &[u8],
    now_ms: u64,
) -> Result<([u8; MAX_LOOPBACK_FRAME_BYTES], usize), LoopbackError> {
    let (response, response_len) = handle_loopback_frame(packet, now_ms)?;
    let mut candidates = Vec::with_capacity(ALLOC_SMOKE_CANDIDATES);
    for _ in 0..ALLOC_SMOKE_CANDIDATES {
        candidates.push(response[..response_len].to_vec());
    }

    let rotation = response_len % candidates.len();
    candidates.rotate_left(rotation);
    let selected = candidates.first().map(Vec::as_slice).unwrap_or_default();
    let mut output = [0_u8; MAX_LOOPBACK_FRAME_BYTES];
    for (destination, source) in output.iter_mut().zip(selected) {
        *destination = *source;
    }
    Ok((output, selected.len()))
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocSmokeAppDataAction {
    Ignore,
    Loopback,
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn classify_alloc_smoke_app_data(packet: &[u8]) -> AllocSmokeAppDataAction {
    if packet.len() >= 3
        && packet.first().copied() == Some(WireVersion::CURRENT.get())
        && packet
            .get(1)
            .and_then(|command| WireCommand::try_from(*command).ok())
            .is_some()
    {
        AllocSmokeAppDataAction::Loopback
    } else {
        AllocSmokeAppDataAction::Ignore
    }
}

#[cfg(test)]
mod tests {
    use super::{AllocSmokeAppDataAction, classify_alloc_smoke_app_data};

    #[test]
    fn alloc_smoke_app_data_accepts_loopback_frames() {
        assert_eq!(
            classify_alloc_smoke_app_data(&[1, 1, 0]),
            AllocSmokeAppDataAction::Loopback
        );
    }

    #[test]
    fn alloc_smoke_app_data_ignores_unrelated_requests() {
        assert_eq!(
            classify_alloc_smoke_app_data(b"hello?"),
            AllocSmokeAppDataAction::Ignore
        );
    }

    #[test]
    fn alloc_smoke_loopback_allocates_and_preserves_echo() {
        let request = [1, 2, 2, 9, 8];
        let (response, response_len) = super::alloc_smoke_loopback(&request, 0).expect("loopback");
        assert_eq!(&response[..response_len], &[1, 2, 2, 9, 8]);
    }
}
