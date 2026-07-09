//! BLE loopback VESC package payload.
//!
//! This crate is the package library linked into the Cargo-owned final ELF. Generic loader,
//! lifecycle, and firmware wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

mod app_data;
pub mod extensions;

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

vescpkg_rs::package_start!(crate::start);

#[cfg(test)]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();
    true
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start.install_stop_hook();

    let _ = app_data::register(start);
    let _ = start.register_extensions(extensions::package_extension_descriptors());

    // Extension registration can run other firmware setup; register again so the
    // loopback handler remains the active app-data callback (refloat pattern).
    let _ = app_data::register(start);

    true
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
}
