//! Usage-shaped port of the official VESC thread example.
//!
//! The C example allocates a state record and retains a raw thread handle in
//! `lib_info`. Package runtime state and `PackageStart::spawn_threads` provide
//! the same lifetime and shutdown behavior without exposing those pointers.

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
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register(
    start: &mut vescpkg_rs::PackageStart<'_>,
) -> Result<(), vescpkg_rs::PackageStartError> {
    let stack = ThreadWorkingAreaSize::try_from_bytes(1_024)
        .expect("official thread example stack satisfies ChibiOS alignment");
    start.spawn_threads(
        [vescpkg_rs::ThreadSpec::<crate::LoopbackState>::stateless::<
            LoopbackWorker,
        >(stack, vescpkg_rs::thread_name!("Loopback Worker"))],
    )
}
