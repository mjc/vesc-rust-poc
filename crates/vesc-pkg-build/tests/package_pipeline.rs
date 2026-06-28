#![cfg(feature = "test-support")]

use vesc_pkg_build::package_wire::wire_snapshot_report;
use vesc_pkg_build::test_support::{FakeConversionRunner, PackageTestHarness};
use vesc_pkg_build::{PackageTargetMode, PackageTargetPlan, BLE_LOOPBACK_PACKAGE_NAME};

#[test]
fn package_pipeline() {
    let harness = PackageTestHarness::new();
    let runner = FakeConversionRunner::materializing();
    let target = PackageTargetPlan::new(
        harness.root(),
        BLE_LOOPBACK_PACKAGE_NAME,
        "0.1.0",
        PackageTargetMode::PackageOnly,
    );

    let output = target.execute_with(&runner).expect("package pipeline");
    assert_eq!(output, target.package_output_path());
    assert_eq!(
        runner.calls(),
        vec![target.build_plan().conversion_plan().command()]
    );

    let package_bytes = std::fs::read(harness.root().join(&output)).expect("read emitted .vescpkg");
    assert!(!package_bytes.is_empty());

    let report = wire_snapshot_report(&package_bytes).expect("wire snapshot report");
    insta::assert_snapshot!("package_pipeline_wire", report);
}
