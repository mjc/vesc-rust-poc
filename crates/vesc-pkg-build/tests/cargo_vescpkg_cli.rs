use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cargo_vescpkg_bin() -> &'static str {
    env!("CARGO_BIN_EXE_cargo-vescpkg")
}

struct TempWorkspace {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl TempWorkspace {
    fn with_repo_fixture_layout() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        link_repo_fixtures(&root);
        Self { _temp: temp, root }
    }
}

#[cfg(unix)]
fn link_repo_fixtures(root: &Path) {
    let repo = repo_root();
    for entry in ["Cargo.toml", "Cargo.lock", "crates", "fixtures", "scripts"] {
        symlink(repo.join(entry), root.join(entry)).expect("workspace symlink");
    }
    symlink(
        repo.join("fixtures/native-lib-baseline/package"),
        root.join("package"),
    )
    .expect("package symlink");
}

#[test]
fn cargo_vescpkg_requires_the_build_subcommand() {
    let workspace = TempWorkspace::with_repo_fixture_layout();
    let output = Command::new(cargo_vescpkg_bin())
        .current_dir(&workspace.root)
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
    let workspace_root = repo_root();
    let output = Command::new(cargo_vescpkg_bin())
        .current_dir(&workspace_root)
        .args(["build", "--package-only"])
        .output()
        .expect("run cargo-vescpkg build");

    assert!(
        output.status.success(),
        "expected cargo-vescpkg build to succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let package_path = stdout.trim();
    assert_eq!(
        package_path,
        "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
    );
    assert!(
        workspace_root.join(package_path).exists(),
        "expected cargo-vescpkg to materialize the package file"
    );
}
