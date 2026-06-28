use std::path::PathBuf;

use vesc_pkg_build::package_conversion::PackageBinaryConversionPlan;
use vesc_pkg_build::{
    BLE_LOOPBACK_PACKAGE_NAME, NativeLibArtifactPaths, audit_native_lib_artifacts,
    ensure_native_lib_artifacts, native_lib_link_plan,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn native_lib_artifacts() {
    let root = repo_root();
    let plan = PackageBinaryConversionPlan::new(root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");

    ensure_native_lib_artifacts(&plan);
    let paths = NativeLibArtifactPaths::from_link_plan(&native_lib_link_plan());
    let report = audit_native_lib_artifacts(&paths);

    insta::assert_snapshot!("native_lib_semantics", report);

    if std::env::var("VESC_PKG_DISASM").ok().as_deref() == Some("1") {
        eprintln!(
            "{}",
            vesc_pkg_build::native_disasm::elf_disassembly(&paths.elf)
        );
    }

    let expected_native = std::fs::read(&paths.bin).expect("native bin");
    let expected_package = std::fs::read(plan.package_binary_path()).expect("package bin");
    ensure_native_lib_artifacts(&plan);
    assert_eq!(
        std::fs::read(&paths.bin).expect("native bin after second ensure"),
        expected_native,
        "native-lib materialization should be idempotent"
    );
    assert_eq!(
        std::fs::read(plan.package_binary_path()).expect("package bin after second ensure"),
        expected_package,
        "package-lib copy should be idempotent"
    );
}
