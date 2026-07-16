//! BLE loopback VESC package payload.
//!
//! This crate is the package library linked into the Cargo-owned final ELF. Generic loader,
//! lifecycle, and firmware wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unsafe_code)]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

mod app_data;
pub mod extensions;

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(crate) struct LoopbackState;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(crate) static LOOPBACK_STATE: vescpkg_rs::PackageStateStore<LoopbackState> =
    vescpkg_rs::PackageStateStore::new();

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl vescpkg_rs::PackageRuntimeState for LoopbackState {
    fn runtime_store() -> &'static vescpkg_rs::PackageStateStore<Self> {
        &LOOPBACK_STATE
    }
}

vescpkg_rs::package_start!(crate::start);

#[cfg(test)]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    start.install_runtime_state(LoopbackState).is_ok()
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn register_required<S>(
    start: &mut S,
    install_stop: impl FnOnce(&mut S) -> bool,
    register_app_data: impl FnOnce(&mut S) -> bool,
    register_extensions: impl FnOnce(&mut S) -> bool,
    restore_app_data: impl FnOnce(&mut S) -> bool,
) -> bool {
    install_stop(start)
        && register_app_data(start)
        && register_extensions(start)
        && restore_app_data(start)
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    register_required(
        start,
        |start| start.install_runtime_state(LoopbackState).is_ok(),
        app_data::register,
        |start| {
            start
                .register_extensions(extensions::package_extension_descriptors())
                .is_ok()
        },
        // Extension registration can run other firmware setup; register again so
        // the loopback handler remains the active app-data callback.
        app_data::register,
    )
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
        let mut info = vescpkg_rs::LoaderInfo::new();

        assert!(super::package_lib_init(&mut info));
        assert!(info.has_stop_handler());
    }

    #[test]
    fn required_registration_propagates_every_failure() {
        fn step(state: &mut (usize, Option<usize>)) -> bool {
            let current = state.0;
            state.0 += 1;
            state.1 != Some(current)
        }

        for failed in 0..4 {
            let mut state = (0, Some(failed));
            assert!(!super::register_required(
                &mut state, step, step, step, step
            ));
            assert_eq!(state.0, failed + 1);
        }

        let mut state = (0, None);
        assert!(super::register_required(&mut state, step, step, step, step));
        assert_eq!(state.0, 4);
    }
}
