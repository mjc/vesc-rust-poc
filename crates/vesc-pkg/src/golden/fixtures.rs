//! Compile-time pinned bytes for the BLE loopback package golden fixtures.

use std::path::PathBuf;

/// Fixture package version used by the golden-packaging helpers.
pub const VERSION: &str = "0.1.0";

const FIXTURE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0"
);

/// Reference package library bytes captured from the golden fixture build.
pub const PACKAGE_LIB: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/package_lib.bin"
));
/// Reference native library binary payload from the golden fixture build.
pub const NATIVE_LIB_BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/native_lib.bin"
));
/// Reference native library ELF payload from the golden fixture build.
pub const NATIVE_LIB_ELF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/native_lib.elf"
));
/// Reference packed Lisp payload from the golden fixture build.
pub const LISP_DATA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/lisp_data.bin"
));
/// Recorded package fingerprint metadata for the golden fixture build.
pub const FINGERPRINTS_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/fingerprints.toml"
));

/// Returns the root directory that holds checked-in golden package fixtures.

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(FIXTURE_ROOT)
}

/// Returns the null-terminated extension name embedded in the probe fixture.

pub fn probe_extension_name() -> &'static [u8] {
    b"ext-rust-probe-diag-v4\0"
}

/// Returns whether a payload contains the probe extension marker.

pub fn payload_contains_probe_extension(payload: &[u8]) -> bool {
    payload
        .windows(probe_extension_name().len())
        .any(|window| window == probe_extension_name())
}
