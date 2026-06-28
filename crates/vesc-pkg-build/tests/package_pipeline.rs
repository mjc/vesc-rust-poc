#![cfg(feature = "test-support")]

use vesc_pkg_build::package_wire::{field_bytes, parse_lisp_imports, parse_vescpkg};
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

    let fields = parse_vescpkg(&package_bytes).expect("parse vescpkg");
    assert_eq!(
        fields
            .iter()
            .map(|field| field.key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "description_md",
            "lispData",
            "pkgDescQml",
            "qmlIsFullscreen",
        ]
    );
    assert_eq!(
        field_bytes(&fields, "name").expect("name field"),
        BLE_LOOPBACK_PACKAGE_NAME.as_bytes()
    );

    let lisp_data = field_bytes(&fields, "lispData").expect("lispData field");
    let (loader, imports) = parse_lisp_imports(lisp_data).expect("lisp imports");
    assert!(
        loader.contains("(import \"src/package_lib.bin\" 'package-lib)"),
        "loader should reference the staged native import"
    );
    assert_eq!(imports.len(), 1);
    assert!(imports[0].payload.starts_with(b"payload"));
    assert!(imports[0].payload[b"payload".len()..]
        .iter()
        .all(|byte| *byte == 0));
}
