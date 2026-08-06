//! Usage-shaped port of the official VESC thread example.
//!
//! The C example allocates a state record and retains a raw thread handle in
//! `lib_info`. Package runtime state and `PackageStart::spawn_threads` provide
//! the same lifetime and shutdown behavior without exposing those pointers.
//! Source: VESC's official
//! [`c_libs/examples/thread`](https://github.com/vedderb/vesc_pkg/blob/ddf1e162d5b7d01d848263af317cc7f8f14c0d14/c_libs/examples/thread/code.c)
//! example.

#[cfg(target_arch = "arm")]
use core::time::Duration;
#[cfg(target_arch = "arm")]
use vescpkg_rs::{FirmwareThreads, ThreadWorkingAreaSize};

#[cfg(target_arch = "arm")]
struct LoopbackWorker;

#[cfg(target_arch = "arm")]
impl vescpkg_rs::StatelessFirmwareThread for LoopbackWorker {
    fn run(ctx: vescpkg_rs::StatelessThreadContext) {
        let threads = ctx.threads();
        while !threads.should_terminate() {
            threads.sleep_for(Duration::from_secs(1));
        }
    }
}

/// Start the official-example-shaped worker and retain it in package state.
///
/// # Errors
///
/// Returns an error when the working area is invalid or the firmware cannot
/// spawn and retain the worker.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register(
    start: &mut vescpkg_rs::PackageStart<'_>,
) -> Result<(), vescpkg_rs::PackageStartError> {
    let stack = ThreadWorkingAreaSize::try_from_bytes(1_024)
        .map_err(|_| vescpkg_rs::PackageStartError::ThreadSpawnFailed)?;
    start.spawn_threads(
        [vescpkg_rs::ThreadSpec::<crate::LoopbackState>::stateless::<
            LoopbackWorker,
        >(stack, vescpkg_rs::thread_name!("Loopback Worker"))],
    )
}
