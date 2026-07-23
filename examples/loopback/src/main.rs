//! BLE loopback VESC package payload.
//!
//! This crate is the Cargo-owned package program. Generic loader, lifecycle, and firmware
//! wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]
#![deny(warnings, clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]
#![forbid(unused_extern_crates)]
// An embedded package cannot unwind or print a useful panic report. Keep
// explicit crash shortcuts out of the production entrypoint and its modules.
#![cfg_attr(
    not(test),
    deny(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

#[cfg(test)]
extern crate std;

#[cfg(not(target_arch = "arm"))]
fn main() {}

mod app_data;
pub mod extensions;

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct LoopbackState;

#[cfg(any(test, target_arch = "arm"))]
vescpkg_rs::package_start!(crate::start, LoopbackState);

#[cfg(test)]
pub(crate) fn start(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(LoopbackState)
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(LoopbackState)?;
    app_data::register(start)?;
    if !start
        .register_extensions(extensions::package_extension_descriptors())?
        .is_complete()
    {
        return Err(vescpkg_rs::PackageStartError::ExtensionRegistrationIncomplete);
    }
    // Extension registration can run other firmware setup; register again so
    // the loopback handler remains the active app-data callback.
    app_data::register(start)
}

#[cfg(test)]
mod tests {
    use super::extensions;

    #[test]
    fn rust_add_stays_a_plain_integer_function() {
        assert_eq!(extensions::rust_add(1, 2), 3);
        assert_eq!(extensions::rust_add(-8, 11), 3);
    }

    #[test]
    fn package_lib_init_runs_the_device_loopback_entrypoint_path() {
        let mut info = vescpkg_rs::test_support::LoaderInfo::new();

        assert!(super::package_lib_init(&raw mut info));
        assert!(info.has_stop_handler());
    }
}
