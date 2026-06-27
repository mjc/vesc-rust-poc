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

#[cfg(test)]
mod tests {
    use super::{
        git_check_ignore_all, git_tracks_all, repo_root, GENERATED_PACKAGE_PATHS, TRACKED_LOCKFILES,
    };
    use std::fs;

    #[test]
    fn generated_package_paths_are_ignored() {
        assert!(
            git_check_ignore_all(&GENERATED_PACKAGE_PATHS),
            "expected generated package outputs to be ignored"
        );
    }

    #[test]
    fn lockfiles_remain_tracked() {
        assert!(
            git_tracks_all(&TRACKED_LOCKFILES),
            "expected lockfiles to stay tracked"
        );
    }

    #[test]
    fn makefile_defines_the_package_targets() {
        let source = fs::read_to_string(repo_root().join("Makefile")).expect("root Makefile");

        assert!(source.contains("check: fmt clippy test"));
        assert!(source.contains("test-changed -r nextest"));
        assert!(source.contains("test-all:"));
        assert!(source.contains("nextest run --workspace --no-fail-fast --features test-support"));
        assert!(source.contains("package: check"));
        assert!(source.contains("package-only:"));
        assert!(source.contains("run -p vesc-pkg-build --bin vesc-pkg -- package"));
        assert!(source.contains("run -p vesc-pkg-build --bin vesc-pkg -- package-only"));
    }
}
