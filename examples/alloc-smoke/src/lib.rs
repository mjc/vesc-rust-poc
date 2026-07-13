//! Target package proving ordinary Rust `alloc` use can run on the VESC allocator.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::vec::Vec;
#[cfg(not(test))]
use vescpkg_rs::VescAllocator;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vesc_protocol::ble_loopback::{LoopbackError, MAX_LOOPBACK_FRAME_BYTES, handle_loopback_frame};
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vesc_protocol::{WireCommand, WireVersion};

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;

vescpkg_rs::package_start!(crate::start);

#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_CANDIDATES: usize = 5;

/// Initialize the alloc smoke package.
pub fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    if start.install_stop_hook().is_err() {
        return false;
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let Some(callback) = start.app_data_callback::<AllocSmokeAppData>() else {
            return false;
        };
        if callback.register().is_err() {
            return false;
        }
    }

    true
}

#[cfg(all(not(test), target_arch = "arm"))]
struct AllocSmokeAppData;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::AppDataCallback for AllocSmokeAppData {
    fn handle(packet: vescpkg_rs::AppDataPacket<'_>) {
        match classify_alloc_smoke_app_data(packet.as_bytes()) {
            AllocSmokeAppDataAction::Ignore => {}
            AllocSmokeAppDataAction::Loopback => {
                let now_ms = u64::from(
                    vescpkg_rs::Firmware::new()
                        .app_data()
                        .system_time_ticks()
                        .as_ticks(),
                ) / 10;
                if let Ok((response, response_len)) =
                    alloc_smoke_loopback(packet.as_bytes(), now_ms)
                {
                    send_app_data(&response[..response_len]);
                }
            }
        }
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

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::PackageAppDataCallback for AllocSmokeAppData {
    fn image_address() -> usize {
        alloc_smoke_app_data_callback as *const () as usize
    }
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// # Safety
///
/// `data` must be null with `len == 0` or point to `len` readable bytes that
/// remain valid for the duration of this call.
#[unsafe(no_mangle)]
#[inline(never)]
#[cfg(all(not(test), target_arch = "arm"))]
pub unsafe extern "C" fn alloc_smoke_app_data_callback(data: *mut u8, len: u32) {
    let Some(bytes) = (!data.is_null() && len != 0)
        .then(|| unsafe { core::slice::from_raw_parts(data.cast_const(), len as usize) })
    else {
        return;
    };
    <AllocSmokeAppData as vescpkg_rs::AppDataCallback>::handle(
        vescpkg_rs::AppDataPacket::from_bytes(bytes),
    );
}

#[cfg(all(not(test), target_arch = "arm"))]
fn send_app_data(bytes: &[u8]) {
    let _ = vescpkg_rs::Firmware::new().app_data().send(bytes);
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
