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
        git_check_ignore, git_tracks, repo_root, GENERATED_PACKAGE_PATHS, TRACKED_LOCKFILES,
    };
    use std::fs;

    #[test]
    fn generated_package_paths_are_ignored() {
        assert!(
            GENERATED_PACKAGE_PATHS
                .iter()
                .all(|path| git_check_ignore(path)),
            "expected generated package outputs to be ignored"
        );
    }

    #[test]
    fn lockfiles_remain_tracked() {
        assert!(
            TRACKED_LOCKFILES.iter().all(|path| git_tracks(path)),
            "expected lockfiles to stay tracked"
        );
    }

    #[test]
    fn makefile_defines_the_package_targets() {
        let source = fs::read_to_string(repo_root().join("Makefile")).expect("root Makefile");

        assert!(source.contains("package: check"));
        assert!(source.contains("package-only:"));
        assert!(source.contains("run -p vesc-pkg-build --bin vesc-pkg -- package"));
        assert!(source.contains("run -p vesc-pkg-build --bin vesc-pkg -- package-only"));
    }
}
