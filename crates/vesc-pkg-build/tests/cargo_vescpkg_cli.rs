use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cargo_vescpkg_bin() -> &'static str {
    env!("CARGO_BIN_EXE_cargo-vescpkg")
}

#[test]
fn cargo_vescpkg_requires_the_build_subcommand() {
    let output = Command::new(cargo_vescpkg_bin())
        .current_dir(repo_root())
        .output()
        .expect("run cargo-vescpkg");

    assert!(
        !output.status.success(),
        "expected the CLI to reject empty args"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage: cargo vescpkg build"));
}

#[test]
fn cargo_vescpkg_build_package_only_writes_the_package_path() {
    let output = Command::new(cargo_vescpkg_bin())
        .current_dir(repo_root())
        .args(["build", "--package-only"])
        .output()
        .expect("run cargo-vescpkg build");

    assert!(
        output.status.success(),
        "expected cargo-vescpkg build to succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let package_path = stdout.trim();
    assert_eq!(
        package_path,
        "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
    );
    assert!(
        repo_root().join(package_path).exists(),
        "expected cargo-vescpkg to materialize the package file"
    );
}
