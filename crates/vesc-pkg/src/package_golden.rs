use std::fs;
use std::path::{Path, PathBuf};

pub const GOLDEN_FIXTURE_VERSION: &str = "0.1.0";
pub const GOLDEN_FIXTURE_DIR: &str = "fixtures/golden/ble-loopback-0.1.0";
pub const GOLDEN_PACKAGE_LIB_BIN: &str = "package_lib.bin";
pub const GOLDEN_LISP_DATA_BIN: &str = "lisp_data.bin";
pub const GOLDEN_FINGERPRINTS_TOML: &str = "fingerprints.toml";

pub fn golden_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(GOLDEN_FIXTURE_DIR)
}

pub fn golden_package_lib_path() -> PathBuf {
    golden_fixture_root().join(GOLDEN_PACKAGE_LIB_BIN)
}

pub fn golden_lisp_data_path() -> PathBuf {
    golden_fixture_root().join(GOLDEN_LISP_DATA_BIN)
}

pub fn read_golden_package_lib() -> Vec<u8> {
    fs::read(golden_package_lib_path()).unwrap_or_else(|error| {
        panic!(
            "missing golden fixture {}: {error}",
            golden_package_lib_path().display()
        )
    })
}

pub fn read_golden_lisp_data() -> Vec<u8> {
    fs::read(golden_lisp_data_path()).unwrap_or_else(|error| {
        panic!(
            "missing golden fixture {}: {error}",
            golden_lisp_data_path().display()
        )
    })
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn package_lib_output_path(root: &Path) -> PathBuf {
    root.join("target/native-lib-baseline/package_lib.bin")
}

pub fn native_lib_output_path(root: &Path) -> PathBuf {
    root.join("target/native-lib-baseline/native_lib.bin")
}

pub fn staged_package_lib_path(root: &Path) -> PathBuf {
    root.join("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/src/package_lib.bin")
}

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

#[cfg(test)]
mod tests {
    use super::{
        GOLDEN_FIXTURE_VERSION, build_and_copy_package_lib_bin, golden_fixture_root,
        read_golden_lisp_data, read_golden_package_lib, repo_root, staged_package_lib_path,
    };
    use crate::package_format::build_lisp_data;
    use crate::package_format_decode::{
        assert_bytes_eq, extract_field, parse_lisp_imports,
        payload_matches_native_with_only_nul_tail,
    };
    use crate::package_runner::RealPackageRunner;
    use crate::package_target::{PackageTargetMode, PackageTargetPlan};
    use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageAssets, PackageLayout, PackageProvenance};
    use std::fs;

    fn ble_loopback_assets() -> PackageAssets {
        PackageAssets::new(
            PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, GOLDEN_FIXTURE_VERSION),
            PackageProvenance::empty(),
        )
    }

    fn staging_dir(root: &std::path::Path) -> std::path::PathBuf {
        root.join(ble_loopback_assets().staging_dir())
    }

    fn build_lisp_data_from_staged_layout(root: &std::path::Path) -> Vec<u8> {
        let assets = ble_loopback_assets();
        build_lisp_data(&assets.render_loader(), &staging_dir(root)).expect("lisp data")
    }

    fn build_lisp_data_from_golden_binary(root: &std::path::Path) -> Vec<u8> {
        let golden = read_golden_package_lib();
        let src_dir = staging_dir(root).join("src");
        fs::create_dir_all(&src_dir).expect("staging src dir");
        fs::write(src_dir.join("package_lib.bin"), &golden).expect("staged golden native payload");
        build_lisp_data_from_staged_layout(root)
    }

    #[test]
    fn package_lib_bin_is_byte_identical_to_golden() {
        let root = repo_root();
        let actual = build_and_copy_package_lib_bin(&root);
        let expected = read_golden_package_lib();

        assert_bytes_eq(&actual, &expected, "package_lib.bin");
        assert!(
            actual
                .windows(b"ext-rust-probe-diag-v4\0".len())
                .any(|window| window == b"ext-rust-probe-diag-v4\0"),
            "golden native payload must retain the Rust probe extension identity"
        );
    }

    #[test]
    fn native_lib_bin_is_byte_identical_to_golden() {
        let root = repo_root();
        let native_bin = super::native_lib_output_path(&root);
        crate::symbol_audit::build_final_native_lib_binary(&native_bin);
        let actual = fs::read(&native_bin).expect("native_lib.bin bytes");
        let expected = read_golden_package_lib();

        assert_bytes_eq(&actual, &expected, "native_lib.bin");
    }

    #[test]
    fn lisp_data_is_byte_identical_to_golden() {
        let root = repo_root();
        build_and_copy_package_lib_bin(&root);
        let actual = build_lisp_data_from_staged_layout(&root);
        let expected = read_golden_lisp_data();

        assert_bytes_eq(&actual, &expected, "lispData");
    }

    #[test]
    fn lisp_data_embeds_golden_native_import_payload() {
        let root = repo_root();
        let golden_native = read_golden_package_lib();
        let lisp_data = build_lisp_data_from_golden_binary(&root);
        let (_, imports) = parse_lisp_imports(&lisp_data);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].tag, "package-lib");
        assert!(
            payload_matches_native_with_only_nul_tail(&imports[0].payload, &golden_native),
            "embedded package-lib import payload must match golden native bytes with only nul padding"
        );
    }

    #[test]
    fn lisp_data_is_idempotent() {
        let root = repo_root();
        build_and_copy_package_lib_bin(&root);

        let first = build_lisp_data_from_staged_layout(&root);
        let second = build_lisp_data_from_staged_layout(&root);

        assert_bytes_eq(&first, &second, "lispData idempotency");
    }

    #[test]
    fn full_package_target_lisp_and_binary_match_golden() {
        let root = repo_root();
        let target = PackageTargetPlan::with_provenance(
            &root,
            BLE_LOOPBACK_PACKAGE_NAME,
            GOLDEN_FIXTURE_VERSION,
            PackageProvenance::empty(),
            PackageTargetMode::PackageOnly,
        );
        let runner = RealPackageRunner;

        target.execute_with(&runner).expect("package target");

        let staged_binary =
            fs::read(staged_package_lib_path(&root)).expect("staged package_lib.bin");
        let golden_binary = read_golden_package_lib();
        assert_bytes_eq(&staged_binary, &golden_binary, "staged src/package_lib.bin");

        let package_bytes =
            fs::read(root.join(target.package_output_path())).expect("assembled .vescpkg bytes");
        let lisp_data = extract_field(&package_bytes, "lispData");
        let golden_lisp_data = read_golden_lisp_data();
        assert_bytes_eq(&lisp_data, &golden_lisp_data, "assembled lispData");
    }

    #[test]
    fn golden_fixture_directory_exists() {
        assert!(
            golden_fixture_root().is_dir(),
            "expected golden fixtures at {}",
            golden_fixture_root().display()
        );
    }
}
