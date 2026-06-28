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

    pub fn staged_native_payload_path(&self) -> PathBuf {
        self.staging_dir_path().join("src/package_lib.bin")
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
        self.inspect_staged_native_payload(&mut problems);

        if problems.is_empty() {
            Ok(())
        } else {
            Err(PackageArtifactInspectionError::new(problems))
        }
    }

    pub fn inspect_package_output(&self) -> Result<(), PackageArtifactInspectionError> {
        let path = self.package_output_path();
        let Ok(bytes) = fs::read(&path) else {
            return Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::MissingPath { path },
            ]));
        };

        if bytes.is_empty() {
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::ContentMismatch {
                    path,
                    expected: "non-empty package artifact".to_owned(),
                    actual: "empty file".to_owned(),
                },
            ]))
        } else {
            Ok(())
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
        inspect_non_empty_binary(path, "non-empty native payload", problems);
    }

    fn inspect_staged_native_payload(&self, problems: &mut Vec<PackageArtifactProblem>) {
        let path = self.staged_native_payload_path();
        inspect_non_empty_binary(path, "non-empty staged native payload", problems);
    }
}

fn inspect_non_empty_binary(
    path: PathBuf,
    expected: &str,
    problems: &mut Vec<PackageArtifactProblem>,
) {
    let Ok(bytes) = fs::read(&path) else {
        problems.push(PackageArtifactProblem::MissingPath { path });
        return;
    };

    if bytes.is_empty() {
        problems.push(PackageArtifactProblem::ContentMismatch {
            path,
            expected: expected.to_owned(),
            actual: "empty file".to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PackageArtifactInspectionError, PackageArtifactInspectionPlan, PackageArtifactProblem,
        NATIVE_PAYLOAD_PATH, STAGING_DESCRIPTOR_PATH, STAGING_LOADER_PATH, STAGING_README_PATH,
    };
    use crate::package_assets::PackageProvenance;
    use crate::test_support::PackageTestHarness;
    use crate::BLE_LOOPBACK_PACKAGE_NAME;

    #[test]
    fn reports_missing_required_artifacts() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let root = harness.root();
        let staging_root = harness.loopback_staging_dir();
        let plan = PackageArtifactInspectionPlan::new(
            root,
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageProvenance::empty(),
        );

        assert_eq!(
            plan.inspect(),
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::MissingPath {
                    path: staging_root.join(STAGING_README_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: staging_root.join(STAGING_DESCRIPTOR_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: staging_root.join(STAGING_LOADER_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: root.join(NATIVE_PAYLOAD_PATH)
                },
                PackageArtifactProblem::MissingPath {
                    path: staging_root.join("src/package_lib.bin")
                },
            ]))
        );
    }

    #[test]
    fn reports_content_mismatches_with_exact_paths() {
        let harness = PackageTestHarness::new()
            .ensure_loopback_staging()
            .write_text(NATIVE_PAYLOAD_PATH, "payload")
            .write_loopback_staging_text("src/package_lib.bin", "payload")
            .write_loopback_staging_text(STAGING_README_PATH, "wrong readme")
            .write_loopback_staging_text(STAGING_DESCRIPTOR_PATH, "wrong descriptor")
            .write_loopback_staging_text(
                STAGING_LOADER_PATH,
                "; Auto-generated loader for Rust BLE loopback test package\n(load-native-lib \"src/package_lib.bin\")\n",
            );
        let staging_root = harness.loopback_staging_dir();
        let plan = PackageArtifactInspectionPlan::new(
            harness.root(),
            BLE_LOOPBACK_PACKAGE_NAME,
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
                PackageArtifactProblem::ContentMismatch {
                    path: staging_root.join(STAGING_LOADER_PATH),
                    expected: plan.assets().render_loader(),
                    actual: "; Auto-generated loader for Rust BLE loopback test package\n(load-native-lib \"src/package_lib.bin\")\n".to_owned(),
                },
            ]))
        );
    }

    #[test]
    fn reports_empty_native_payloads() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let plan = PackageArtifactInspectionPlan::new(
            harness.root(),
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageProvenance::empty(),
        );
        let native_payload_path = harness.root().join(NATIVE_PAYLOAD_PATH);
        let _harness = harness
            .write_loopback_staging_text(STAGING_README_PATH, &plan.assets().render_readme())
            .write_loopback_staging_text(
                STAGING_DESCRIPTOR_PATH,
                &plan.assets().render_descriptor(),
            )
            .write_loopback_staging_text(STAGING_LOADER_PATH, &plan.assets().render_loader())
            .write_text(NATIVE_PAYLOAD_PATH, "")
            .write_loopback_staging_text("src/package_lib.bin", "payload");

        assert_eq!(
            plan.inspect(),
            Err(PackageArtifactInspectionError::new(vec![
                PackageArtifactProblem::ContentMismatch {
                    path: native_payload_path,
                    expected: "non-empty native payload".to_owned(),
                    actual: "empty file".to_owned(),
                },
            ]))
        );
    }
}
