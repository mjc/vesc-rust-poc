//! Target package proving ordinary Rust `alloc` use can run on the VESC allocator.

#![no_std]
#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

extern crate alloc;

#[cfg(any(test, not(target_arch = "arm")))]
extern crate std;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use alloc::vec::Vec;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::VescAllocator;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vesc_protocol::ble_loopback::{LoopbackError, MAX_LOOPBACK_FRAME_BYTES, handle_loopback_frame};
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::PackageStart;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::{AppDataHandler, AppDataPacket, Firmware};

#[cfg(all(not(test), target_arch = "arm"))]
#[global_allocator]
static ALLOCATOR: VescAllocator = VescAllocator;
#[cfg(all(not(test), not(target_arch = "arm")))]
#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;

vescpkg_rs::package_start!(crate::start);

#[cfg(any(test, all(not(test), target_arch = "arm")))]
const ALLOC_SMOKE_CANDIDATES: usize = 5;

#[cfg(all(not(test), target_arch = "arm"))]
struct AllocSmokeAppData;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
struct AllocSmokeState;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
static ALLOC_SMOKE_STATE: vescpkg_rs::PackageStateStore<AllocSmokeState> =
    vescpkg_rs::PackageStateStore::new();

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl vescpkg_rs::PackageRuntimeState for AllocSmokeState {
    fn runtime_store() -> &'static vescpkg_rs::PackageStateStore<Self> {
        &ALLOC_SMOKE_STATE
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
impl AppDataHandler for AllocSmokeAppData {
    type State = AllocSmokeState;

    fn handle(_state: &mut Self::State, packet: AppDataPacket<'_>) {
        let firmware = Firmware::new();
        let app_data = firmware.app_data();
        let now_ms = u64::from(firmware.clock().now().as_ticks()) / 10;
        let Ok((response, response_len)) = alloc_smoke_loopback(packet.as_bytes(), now_ms) else {
            return;
        };
        let _ = response
            .get(..response_len)
            .is_some_and(|response| app_data.send(response).is_ok());
    }
}

vescpkg_rs::firmware_stateful_app_data_callback!(alloc_smoke_app_data_callback, AllocSmokeAppData);

/// Initialize the alloc smoke package.
#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn start(start: &mut PackageStart) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(AllocSmokeState)?;
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        start
            .app_data_callback::<AllocSmokeAppData>()
            .ok_or(vescpkg_rs::PackageStartError::StateTypeMismatch)?
            .register()?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    #[test]
    fn alloc_smoke_loopback_allocates_and_preserves_echo() {
        let request = [1, 2, 2, 9, 8];
        let (response, response_len) = super::alloc_smoke_loopback(&request, 0).expect("loopback");
        assert_eq!(&response[..response_len], &[1, 2, 2, 9, 8]);
    }
}
