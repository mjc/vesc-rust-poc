use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const TRACKED_LOCKFILES: [&str; 2] = ["Cargo.lock", "flake.lock"];
pub const ROOT_MAKEFILE_PATH: &str = "Makefile";
pub const GENERATED_PACKAGE_PATHS: [&str; 4] = [
    "target/native-lib-baseline/native_lib.bin",
    "target/native-lib-baseline/native_lib.elf",
    "target/native-lib-baseline/package_lib.bin",
    "target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg",
];

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn git_check_ignore(path: &str) -> bool {
    Command::new("git")
        .arg("check-ignore")
        .arg("-q")
        .arg(path)
        .current_dir(repo_root())
        .status()
        .is_ok_and(|status| status.success())
}

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

pub fn git_tracks(path: &str) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch", path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(repo_root())
        .status()
        .is_ok_and(|status| status.success())
}

pub const MAKE_DRY_RUN_TARGETS: &[&str] = &["check", "check-full"];

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
        let vesc_ffi_manifest = repo_root().join("crates/vesc-ffi/Cargo.toml");
        let manifest = fs::read_to_string(&vesc_ffi_manifest).expect("vesc-ffi manifest");
        assert!(
            !manifest.contains("test-support"),
            "vesc-ffi must not declare test-support; mock table is cfg(test) only"
        );
        let vesc_ffi_lib = fs::read_to_string(repo_root().join("crates/vesc-ffi/src/lib.rs"))
            .expect("vesc-ffi lib.rs");
        assert!(
            vesc_ffi_lib.contains("#[cfg(test)]\npub mod test_support"),
            "expected test_support to be cfg(test) gated"
        );
    }
}
