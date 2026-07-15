//! End-to-end flat-image relocation rejection.

use std::path::Path;
use std::process::Command;

#[test]
fn rejects_a_pointer_bearing_loadable_static() {
    let manifest =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/relocation-package/Cargo.toml");
    let target = tempfile::tempdir().expect("fixture target directory");
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-vescpkg"))
        .args(["build", "-p", "relocation-package", "--manifest-path"])
        .arg(manifest)
        .env("CARGO_TARGET_DIR", target.path())
        .output()
        .expect("run cargo-vescpkg fixture build");

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("unsupported absolute relocation"), "{error}");
    assert!(
        error.contains("pointer-bearing loadable statics"),
        "{error}"
    );
}
