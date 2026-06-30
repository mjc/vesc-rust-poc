use std::fs;
use std::path::{Path, PathBuf};

/// Version string used by the checked-in golden package fixture.
pub const GOLDEN_FIXTURE_VERSION: &str = "0.1.0";
/// Relative path to the checked-in golden package fixture directory.
pub const GOLDEN_FIXTURE_DIR: &str = "fixtures/golden/ble-loopback-0.1.0";
/// File name of the golden native package payload.
pub const GOLDEN_PACKAGE_LIB_BIN: &str = "package_lib.bin";
/// File name of the golden packed Lisp payload.
pub const GOLDEN_LISP_DATA_BIN: &str = "lisp_data.bin";
/// File name of the golden fingerprint metadata.
pub const GOLDEN_FINGERPRINTS_TOML: &str = "fingerprints.toml";

/// Returns the absolute path to the checked-in golden fixture directory.

pub fn golden_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(GOLDEN_FIXTURE_DIR)
}

/// Returns the golden native package payload path.

pub fn golden_package_lib_path() -> PathBuf {
    golden_fixture_root().join(GOLDEN_PACKAGE_LIB_BIN)
}

/// Returns the golden packed Lisp payload path.

pub fn golden_lisp_data_path() -> PathBuf {
    golden_fixture_root().join(GOLDEN_LISP_DATA_BIN)
}

/// Reads the checked-in golden native package payload.

pub fn read_golden_package_lib() -> Vec<u8> {
    fs::read(golden_package_lib_path()).unwrap_or_else(|error| {
        panic!(
            "missing golden fixture {}: {error}",
            golden_package_lib_path().display()
        )
    })
}

/// Reads the checked-in golden packed Lisp payload.

pub fn read_golden_lisp_data() -> Vec<u8> {
    fs::read(golden_lisp_data_path()).unwrap_or_else(|error| {
        panic!(
            "missing golden fixture {}: {error}",
            golden_lisp_data_path().display()
        )
    })
}

/// Returns the repository root used to locate generated package artifacts.

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Returns the generated package payload path under `root`.

pub fn package_lib_output_path(root: &Path) -> PathBuf {
    root.join("target/native-lib-baseline/package_lib.bin")
}

/// Returns the generated native-lib payload path under `root`.

pub fn native_lib_output_path(root: &Path) -> PathBuf {
    root.join("target/native-lib-baseline/native_lib.bin")
}

/// Returns the staged package payload path under `root`.

pub fn staged_package_lib_path(root: &Path) -> PathBuf {
    root.join("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/src/package_lib.bin")
}

/// Builds the package payload and copies it into the golden-output location under `root`.

pub fn build_and_copy_package_lib_bin(root: &Path) -> Vec<u8> {
    let native_bin = native_lib_output_path(root);
    crate::symbol_audit::build_final_native_lib_binary(&native_bin);

    let package_bin = package_lib_output_path(root);
    if let Some(parent) = package_bin.parent() {
        fs::create_dir_all(parent).expect("package_lib.bin parent directory");
    }
    fs::copy(&native_bin, &package_bin).expect("copy native_lib.bin to package_lib.bin");

    fs::read(&package_bin).expect("package_lib.bin bytes")
}
