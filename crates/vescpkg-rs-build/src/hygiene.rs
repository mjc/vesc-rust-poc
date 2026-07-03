use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Lockfiles that should stay checked into the repository root.
pub const TRACKED_LOCKFILES: [&str; 2] = ["Cargo.lock", "flake.lock"];
/// Repository-root Makefile path used by hygiene checks.
pub const ROOT_MAKEFILE_PATH: &str = "Makefile";
/// Generated package artifacts that must remain ignored by git.
pub const GENERATED_PACKAGE_PATHS: [&str; 4] = [
    "target/native-lib-baseline/native_lib.bin",
    "target/native-lib-baseline/native_lib.elf",
    "target/native-lib-baseline/package_lib.bin",
    "target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg",
];

/// Returns the workspace root that owns the package fixtures and Make targets.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Returns whether `git check-ignore` reports `path` as ignored.
pub fn git_check_ignore(path: &str) -> bool {
    Command::new("git")
        .arg("check-ignore")
        .arg("-q")
        .arg(path)
        .current_dir(repo_root())
        .status()
        .is_ok_and(|status| status.success())
}

/// Returns whether every path is reported as ignored by `git check-ignore`.
pub fn git_check_ignore_all(paths: &[&str]) -> bool {
    let output = Command::new("git")
        .arg("check-ignore")
        .args(paths)
        .current_dir(repo_root())
        .output()
        .expect("git check-ignore");

    if !output.status.success() {
        return false;
    }

    let ignored = String::from_utf8_lossy(&output.stdout);
    paths
        .iter()
        .all(|path| ignored.lines().any(|line| line == *path))
}

/// Returns whether every path is tracked by git.
pub fn git_tracks_all(paths: &[&str]) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch"])
        .args(paths)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(repo_root())
        .status()
        .is_ok_and(|status| status.success())
}

/// Returns whether git tracks `path`.
pub fn git_tracks(path: &str) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch", path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(repo_root())
        .status()
        .is_ok_and(|status| status.success())
}

/// Make targets that should resolve successfully in dry-run mode.
pub const MAKE_DRY_RUN_TARGETS: &[&str] = &["check", "check-full"];

/// Returns whether `make -n <target>` succeeds from the repository root.
pub fn make_dry_run_succeeds(target: &str) -> bool {
    Command::new("make")
        .arg("-n")
        .arg(target)
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Returns the normal direct reverse dependency tree for `vescpkg-rs-sys`.
pub fn vescpkg_sys_direct_normal_dependents() -> Vec<String> {
    cargo_tree_package_names([
        "tree",
        "--workspace",
        "--invert",
        "vescpkg-rs-sys",
        "--edges",
        "normal",
        "--prefix",
        "none",
        "--depth",
        "1",
    ])
}

/// Returns the normal no-default-features dependency tree for `vescpkg-rs-sys`.
pub fn vescpkg_sys_no_default_normal_dependencies() -> Vec<String> {
    cargo_tree_package_names([
        "tree",
        "-p",
        "vescpkg-rs-sys",
        "--edges",
        "normal",
        "--no-default-features",
        "--prefix",
        "none",
    ])
}

fn cargo_tree_package_names<const N: usize>(args: [&str; N]) -> Vec<String> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("cargo tree");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once(' ').map(|(package, _)| package.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        GENERATED_PACKAGE_PATHS, MAKE_DRY_RUN_TARGETS, TRACKED_LOCKFILES, git_check_ignore_all,
        git_tracks_all, make_dry_run_succeeds, repo_root, vescpkg_sys_direct_normal_dependents,
        vescpkg_sys_no_default_normal_dependencies,
    };
    use std::fs;

    #[test]
    fn repo_workspace_hygiene() {
        assert!(
            git_check_ignore_all(&GENERATED_PACKAGE_PATHS),
            "expected generated package outputs to be ignored"
        );
        assert!(
            git_tracks_all(&TRACKED_LOCKFILES),
            "expected lockfiles to stay tracked"
        );
        assert!(
            fs::metadata(repo_root().join(".config/nextest.toml")).is_ok(),
            "expected nextest profiles at .config/nextest.toml"
        );
        assert_eq!(
            MAKE_DRY_RUN_TARGETS,
            ["check", "check-full"],
            "workspace hygiene should dry-run the repo-facing check targets only"
        );
        for target in MAKE_DRY_RUN_TARGETS {
            assert!(
                make_dry_run_succeeds(target),
                "make -n {target} should succeed from repo root"
            );
        }
        assert_eq!(
            vescpkg_sys_no_default_normal_dependencies(),
            ["vescpkg-rs-sys"],
            "vescpkg-rs-sys should stay dependency-free without default features"
        );
        assert_eq!(
            vescpkg_sys_direct_normal_dependents(),
            ["vescpkg-rs-sys", "vescpkg-rs"],
            "only vescpkg-rs should depend directly on the raw sys crate"
        );
    }
}
