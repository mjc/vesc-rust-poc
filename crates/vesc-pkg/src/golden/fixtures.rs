//! Compile-time pinned bytes for the BLE loopback package golden fixtures.

use std::path::PathBuf;

pub const VERSION: &str = "0.1.0";

const FIXTURE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0"
);

pub const PACKAGE_LIB: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/package_lib.bin"
));
pub const NATIVE_LIB_BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/native_lib.bin"
));
pub const NATIVE_LIB_ELF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/native_lib.elf"
));
pub const LISP_DATA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/lisp_data.bin"
));
pub const FINGERPRINTS_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/golden/ble-loopback-0.1.0/fingerprints.toml"
));

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(FIXTURE_ROOT)
}

pub fn probe_extension_name() -> &'static [u8] {
    b"ext-rust-probe-diag-v4\0"
}

pub fn payload_contains_probe_extension(payload: &[u8]) -> bool {
    payload
        .windows(probe_extension_name().len())
        .any(|window| window == probe_extension_name())
}
