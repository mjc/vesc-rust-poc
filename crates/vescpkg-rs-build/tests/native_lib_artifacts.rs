//! Native library artifact audit integration tests.

use std::path::PathBuf;

use tempfile::TempDir;
use vescpkg_rs_build::{
    NATIVE_LIB_BIN, NATIVE_LIB_ELF, NativeLibArtifactPaths, NativeLibLinkPlan, Package,
    PackageBinaryConversionPlan, PackageExample, PackageTargetMode, PackageTargetPlan,
    REFLOAT_PACKAGE_NAME, REFLOAT_PACKAGE_VERSION, RealPackageRunner, audit_native_lib_artifacts,
    audit_refloat_native_lib_artifacts, ensure_native_lib_artifacts,
    ensure_repo_native_lib_artifacts, native_binary_comparison_report, native_lib_link_plan,
    semantic_snapshot_report, wire_comparison_report,
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
fn current_refloat_native_lib_is_loader_only_containment_candidate() {
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

#[test]
#[ignore = "requires target/refloat-1.2.1-upstream.vescpkg captured from Refloat v1.2.1"]
fn captured_refloat_package_comparison_reports_native_payload_delta() {
    let root = repo_root();
    let baseline = std::fs::read(root.join("target/refloat-1.2.1-upstream.vescpkg"))
        .expect("read Refloat v1.2.1 VESC Tool package capture");
    let target = PackageTargetPlan::for_example(
        &root,
        REFLOAT_PACKAGE_NAME,
        REFLOAT_PACKAGE_VERSION,
        PackageExample::Refloat,
        PackageTargetMode::PackageOnly,
    );

    let output = target
        .execute_with(&RealPackageRunner)
        .expect("build Rust Refloat package");
    let rust_package = std::fs::read(root.join(output)).expect("read Rust Refloat package");
    let report = wire_comparison_report(&baseline, &rust_package).expect("wire comparison report");

    insta::assert_snapshot!("captured_refloat_package_comparison", report);
}

#[test]
#[ignore = "requires target/refloat-1.2.1-upstream.vescpkg captured from official Refloat v1.2.1"]
fn captured_refloat_qml_matches_rust_package() {
    let root = repo_root();
    let baseline = Package::read(root.join("target/refloat-1.2.1-upstream.vescpkg"))
        .expect("read official Refloat package capture");
    let target = PackageTargetPlan::for_example(
        &root,
        REFLOAT_PACKAGE_NAME,
        REFLOAT_PACKAGE_VERSION,
        PackageExample::Refloat,
        PackageTargetMode::PackageOnly,
    );

    let output = target
        .execute_with(&RealPackageRunner)
        .expect("build Rust Refloat package");
    let rust_package = Package::read(root.join(output)).expect("read Rust Refloat package");

    println!(
        "official qml len={} fullscreen={}; rust qml len={} fullscreen={}",
        baseline.qml_file.len(),
        baseline.qml_is_fullscreen,
        rust_package.qml_file.len(),
        rust_package.qml_is_fullscreen
    );

    if baseline.qml_file != rust_package.qml_file {
        let first_diff = baseline
            .qml_file
            .bytes()
            .zip(rust_package.qml_file.bytes())
            .position(|(left, right)| left != right);
        let diff =
            first_diff.unwrap_or_else(|| baseline.qml_file.len().min(rust_package.qml_file.len()));
        let start = diff.saturating_sub(120);
        let official_context = baseline
            .qml_file
            .get(start..baseline.qml_file.len().min(diff + 240))
            .unwrap_or("");
        let rust_context = rust_package
            .qml_file
            .get(start..rust_package.qml_file.len().min(diff + 240))
            .unwrap_or("");
        panic!(
            "Refloat QML differs: first_diff={first_diff:?} official_len={} rust_len={} official_context={official_context:?} rust_context={rust_context:?}",
            baseline.qml_file.len(),
            rust_package.qml_file.len(),
        );
    }

    assert_eq!(
        baseline.qml_is_fullscreen, rust_package.qml_is_fullscreen,
        "QML fullscreen flag should match official package"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
