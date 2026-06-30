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

#[cfg(test)]
mod tests {
    use super::{
        GENERATED_PACKAGE_PATHS, MAKE_DRY_RUN_TARGETS, TRACKED_LOCKFILES, git_check_ignore_all,
        git_tracks_all, make_dry_run_succeeds, repo_root,
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
        let vescpkg_sys_manifest = repo_root().join("crates/vescpkg-sys/Cargo.toml");
        let manifest = fs::read_to_string(&vescpkg_sys_manifest).expect("vescpkg-sys manifest");
        assert!(
            !manifest.contains("test-support"),
            "vescpkg-sys must not declare test-support; mock table is cfg(test) only"
        );
        let vescpkg_sys_lib = fs::read_to_string(repo_root().join("crates/vescpkg-sys/src/lib.rs"))
            .expect("vescpkg-sys lib.rs");
        assert!(
            vescpkg_sys_lib.contains("#[cfg(test)]\npub mod test_support"),
            "expected test_support to be cfg(test) gated"
        );
    }
}
