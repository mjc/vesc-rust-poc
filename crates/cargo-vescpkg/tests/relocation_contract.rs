//! End-to-end flat-image rejection.

use std::path::Path;
use std::process::{Command, Output};

fn build_fixture(feature: Option<&str>) -> Output {
    let manifest =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/relocation-package/Cargo.toml");
    let target = tempfile::tempdir().expect("fixture target directory");
    let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-vescpkg"));
    command
        .args(["build", "-p", "relocation-package", "--manifest-path"])
        .arg(manifest)
        .env("CARGO_TARGET_DIR", target.path());
    if let Some(feature) = feature {
        command.args(["--features", feature]);
    }
    command.output().expect("run cargo-vescpkg fixture build")
}

#[test]
fn rejects_a_writable_pointer_bearing_static() {
    let output = build_fixture(None);

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("unmarked image-offset symbol"), "{error}");
}

#[test]
fn rejects_an_unmarked_pic_function_pointer() {
    let output = build_fixture(Some("unmarked-image-offset"));

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("unmarked image-offset symbol"), "{error}");
}

#[test]
fn accepts_an_explicit_image_offset_function() {
    let output = build_fixture(Some("marked-image-offset"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
