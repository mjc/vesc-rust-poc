//! Native library artifact audit integration tests.

use std::path::PathBuf;

use tempfile::TempDir;
use vescpkg_rs_build::{
    NATIVE_LIB_BIN, NATIVE_LIB_ELF, NativeLibArtifactPaths, NativeLibLinkPlan,
    PackageBinaryConversionPlan, PackageExample, REFLOAT_PACKAGE_NAME, REFLOAT_PACKAGE_VERSION,
    audit_native_lib_artifacts, audit_refloat_native_lib_artifacts, ensure_native_lib_artifacts,
    ensure_repo_native_lib_artifacts, native_binary_comparison_report, native_lib_link_plan,
    semantic_snapshot_report,
};

fn write_fixture_artifacts(dir: &std::path::Path) -> PathBuf {
    let elf = dir.join("native_lib.elf");
    std::fs::write(&elf, NATIVE_LIB_ELF).expect("write fixture elf");
    std::fs::write(dir.join("native_lib.bin"), NATIVE_LIB_BIN).expect("write fixture bin");
    elf
}

#[test]
fn native_lib_semantics() {
    let workspace = TempDir::new().expect("temp workspace");
    let elf = write_fixture_artifacts(workspace.path());
    let report = semantic_snapshot_report(&elf);
    insta::assert_snapshot!("native_lib_semantics", report);
}

#[test]
fn current_native_lib_preserves_known_good_loader_contract() {
    let plan = native_lib_link_plan();
    ensure_repo_native_lib_artifacts(plan.root());
    audit_native_lib_artifacts(&NativeLibArtifactPaths::from_link_plan(&plan));
}

#[test]
fn current_refloat_native_lib_preserves_loader_contract() {
    let plan = NativeLibLinkPlan::for_example(repo_root(), PackageExample::Refloat);
    ensure_native_lib_artifacts(&PackageBinaryConversionPlan::for_example(
        plan.root(),
        REFLOAT_PACKAGE_NAME,
        REFLOAT_PACKAGE_VERSION,
        PackageExample::Refloat,
    ));
    audit_refloat_native_lib_artifacts(&NativeLibArtifactPaths::from_link_plan(&plan));
}

#[test]
fn native_binary_comparison_report_highlights_refloat_loader_delta() {
    let root = repo_root();
    let loopback = native_lib_link_plan();
    let refloat = NativeLibLinkPlan::for_example(root, PackageExample::Refloat);
    ensure_repo_native_lib_artifacts(loopback.root());
    ensure_native_lib_artifacts(&PackageBinaryConversionPlan::for_example(
        refloat.root(),
        REFLOAT_PACKAGE_NAME,
        REFLOAT_PACKAGE_VERSION,
        PackageExample::Refloat,
    ));

    let report = native_binary_comparison_report(
        "loopback",
        &loopback.elf_path(),
        "refloat",
        &refloat.elf_path(),
    );

    insta::assert_snapshot!("native_binary_refloat_comparison", report);
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
