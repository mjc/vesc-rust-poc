use std::fs;
use std::path::PathBuf;

use crate::package_assets::{PackageAssets, PackageProvenance};
use crate::PackageLayout;

pub const STAGING_README_PATH: &str = "README.md";
pub const STAGING_DESCRIPTOR_PATH: &str = "pkgdesc.qml";
pub const STAGING_LOADER_PATH: &str = "code.lisp";
pub const NATIVE_PAYLOAD_PATH: &str = "target/native-lib-baseline/package_lib.bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageArtifactProblem {
    MissingPath {
        path: PathBuf,
    },
    ContentMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageArtifactInspectionError {
    problems: Vec<PackageArtifactProblem>,
}

impl PackageArtifactInspectionError {
    pub fn new(problems: Vec<PackageArtifactProblem>) -> Self {
        Self { problems }
    }

    pub fn problems(&self) -> &[PackageArtifactProblem] {
        &self.problems
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageArtifactInspectionPlan {
    root: PathBuf,
    layout: PackageLayout,
    provenance: PackageProvenance,
}

impl PackageArtifactInspectionPlan {
    pub fn new(
        root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
    ) -> Self {
        Self {
            root: root.into(),
            layout: PackageLayout::new(package_name, version),
            provenance,
        }
    }

    pub fn layout(&self) -> &PackageLayout {
        &self.layout
    }

    pub fn assets(&self) -> PackageAssets {
        PackageAssets::new(self.layout.clone(), self.provenance.clone())
    }

    pub fn staging_dir_path(&self) -> PathBuf {
        self.root.join(self.layout.staging_dir())
    }

    pub fn readme_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_README_PATH)
    }

    pub fn descriptor_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_DESCRIPTOR_PATH)
    }

    pub fn loader_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_LOADER_PATH)
    }

    pub fn native_payload_path(&self) -> PathBuf {
        self.root.join(NATIVE_PAYLOAD_PATH)
    }

    pub fn package_output_path(&self) -> PathBuf {
        self.staging_dir_path().join(self.layout.artifact_name())
    }

    pub fn inspect(&self) -> Result<(), PackageArtifactInspectionError> {
        let mut problems = Vec::new();
        self.inspect_text_file(
            self.readme_path(),
            self.assets().render_readme(),
            &mut problems,
        );
        self.inspect_text_file(
            self.descriptor_path(),
            self.assets().render_descriptor(),
            &mut problems,
        );
        self.inspect_text_file(
            self.loader_path(),
            self.assets().render_loader(),
            &mut problems,
        );
        self.inspect_native_payload(&mut problems);

        if problems.is_empty() {
            Ok(())
        } else {
            Err(PackageArtifactInspectionError::new(problems))
        }
    }

    fn inspect_text_file(
        &self,
        path: PathBuf,
        expected: String,
        problems: &mut Vec<PackageArtifactProblem>,
    ) {
        let Ok(actual) = fs::read_to_string(&path) else {
            problems.push(PackageArtifactProblem::MissingPath { path });
            return;
        };

        if actual != expected {
            problems.push(PackageArtifactProblem::ContentMismatch {
                path,
                expected,
                actual,
            });
        }
    }

    fn inspect_native_payload(&self, problems: &mut Vec<PackageArtifactProblem>) {
        let path = self.native_payload_path();
        let Ok(bytes) = fs::read(&path) else {
            problems.push(PackageArtifactProblem::MissingPath { path });
            return;
        };

        if bytes.is_empty() {
            problems.push(PackageArtifactProblem::ContentMismatch {
                path,
                expected: "non-empty native payload".to_owned(),
                actual: "empty file".to_owned(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PackageArtifactInspectionError, PackageArtifactInspectionPlan, PackageArtifactProblem,
        NATIVE_PAYLOAD_PATH, STAGING_DESCRIPTOR_PATH, STAGING_LOADER_PATH, STAGING_README_PATH,
    };
    use crate::package_assets::PackageProvenance;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vesc-rust-poc-artifacts-{nanos}-{}",
            std::process::id()
        ))
    }

    fn write_artifact(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("artifact parent directory");
        }
        fs::write(path, contents).expect("artifact contents");
    }

    #[test]
    fn reports_missing_required_artifacts() {
        let root = unique_root();
        fs::create_dir_all(root.join("target/vescpkg/Rust-VESC-package-0.1.0"))
            .expect("staging root");
        let plan = PackageArtifactInspectionPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageProvenance::empty(),
        );

        assert_eq!(
            plan.inspect(),
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::MissingPath {
                    path: root
                        .join("target/vescpkg/Rust-VESC-package-0.1.0")
                        .join(STAGING_README_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: root
                        .join("target/vescpkg/Rust-VESC-package-0.1.0")
                        .join(STAGING_DESCRIPTOR_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: root
                        .join("target/vescpkg/Rust-VESC-package-0.1.0")
                        .join(STAGING_LOADER_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: root.join(NATIVE_PAYLOAD_PATH)
                },
            ]))
        );
    }

    #[test]
    fn reports_content_mismatches_with_exact_paths() {
        let root = unique_root();
        let staging_root = root.join("target/vescpkg/Rust-VESC-package-0.1.0");
        fs::create_dir_all(&staging_root).expect("staging root");
        write_artifact(&root, NATIVE_PAYLOAD_PATH, "payload");
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/README.md",
            "wrong readme",
        );
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml",
            "wrong descriptor",
        );
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/code.lisp",
            "; Auto-generated loader for Rust VESC package\n(load-native-lib \"src/package_lib.bin\")\n",
        );
        let plan = PackageArtifactInspectionPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageProvenance::empty(),
        );

        assert_eq!(
            plan.inspect(),
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::ContentMismatch {
                    path: staging_root.join(STAGING_README_PATH),
                    expected: plan.assets().render_readme(),
                    actual: "wrong readme".to_owned(),
                },
                PackageArtifactProblem::ContentMismatch {
                    path: staging_root.join(STAGING_DESCRIPTOR_PATH),
                    expected: plan.assets().render_descriptor(),
                    actual: "wrong descriptor".to_owned(),
                },
            ]))
        );
    }

    #[test]
    fn reports_empty_native_payloads() {
        let root = unique_root();
        let staging_root = root.join("target/vescpkg/Rust-VESC-package-0.1.0");
        fs::create_dir_all(&staging_root).expect("staging root");
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/README.md",
            &PackageArtifactInspectionPlan::new(
                &root,
                "Rust VESC package",
                "0.1.0",
                PackageProvenance::empty(),
            )
            .assets()
            .render_readme(),
        );
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml",
            &PackageArtifactInspectionPlan::new(
                &root,
                "Rust VESC package",
                "0.1.0",
                PackageProvenance::empty(),
            )
            .assets()
            .render_descriptor(),
        );
        write_artifact(
            &root,
            "target/vescpkg/Rust-VESC-package-0.1.0/code.lisp",
            &PackageArtifactInspectionPlan::new(
                &root,
                "Rust VESC package",
                "0.1.0",
                PackageProvenance::empty(),
            )
            .assets()
            .render_loader(),
        );
        write_artifact(&root, NATIVE_PAYLOAD_PATH, "");
        let plan = PackageArtifactInspectionPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageProvenance::empty(),
        );

        assert_eq!(
            plan.inspect(),
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::ContentMismatch {
                    path: root.join(NATIVE_PAYLOAD_PATH),
                    expected: "non-empty native payload".to_owned(),
                    actual: "empty file".to_owned(),
                },
            ]))
        );
    }
}
