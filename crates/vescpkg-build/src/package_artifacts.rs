use std::fs;
use std::path::PathBuf;

use crate::PackageLayout;
use crate::package_assets::{PackageAssets, PackageProvenance};

/// Staged README file name written into package source trees.
pub const STAGING_README_PATH: &str = "README.md";
/// Staged package descriptor file name written into package source trees.
pub const STAGING_DESCRIPTOR_PATH: &str = "pkgdesc.qml";
/// Staged Lisp loader file name written into package source trees.
pub const STAGING_LOADER_PATH: &str = "code.lisp";
/// Repository-relative native payload path copied into package source trees.
pub const NATIVE_PAYLOAD_PATH: &str = "target/native-lib-baseline/package_lib.bin";

/// A problem found while inspecting staged package artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageArtifactProblem {
    /// A required artifact path was missing.
    MissingPath {
        /// Missing path on disk.
        path: PathBuf,
    },
    /// An artifact's contents differed from the expected bytes or text.
    ContentMismatch {
        /// Path whose contents mismatched.
        path: PathBuf,
        /// Expected content summary.
        expected: String,
        /// Actual content summary.
        actual: String,
    },
}

/// Error containing every package artifact inspection problem found in one pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageArtifactInspectionError {
    problems: Vec<PackageArtifactProblem>,
}

impl PackageArtifactInspectionError {
    /// Creates an inspection error from the collected problems.
    pub fn new(problems: Vec<PackageArtifactProblem>) -> Self {
        Self { problems }
    }

    /// Returns the problems found during inspection.
    pub fn problems(&self) -> &[PackageArtifactProblem] {
        &self.problems
    }
}

/// Inspection plan for the generated package staging tree and package output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageArtifactInspectionPlan {
    root: PathBuf,
    layout: PackageLayout,
    provenance: PackageProvenance,
}

impl PackageArtifactInspectionPlan {
    /// Creates an inspection plan for one package layout under `root`.
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

    /// Returns the package layout inspected by this plan.
    pub fn layout(&self) -> &PackageLayout {
        &self.layout
    }

    /// Builds the expected package assets for this inspection plan.
    pub fn assets(&self) -> PackageAssets {
        PackageAssets::new(self.layout.clone(), self.provenance.clone())
    }

    /// Returns the package staging directory path.
    pub fn staging_dir_path(&self) -> PathBuf {
        self.root.join(self.layout.staging_dir())
    }

    /// Returns the expected staged README path.
    pub fn readme_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_README_PATH)
    }

    /// Returns the expected staged package descriptor path.
    pub fn descriptor_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_DESCRIPTOR_PATH)
    }

    /// Returns the expected staged Lisp loader path.
    pub fn loader_path(&self) -> PathBuf {
        self.staging_dir_path().join(STAGING_LOADER_PATH)
    }

    /// Returns the generated native payload path.
    pub fn native_payload_path(&self) -> PathBuf {
        self.root.join(NATIVE_PAYLOAD_PATH)
    }

    /// Returns the staged copy of the generated native payload path.
    pub fn staged_native_payload_path(&self) -> PathBuf {
        self.staging_dir_path().join("src/package_lib.bin")
    }

    /// Returns the final package artifact path.
    pub fn package_output_path(&self) -> PathBuf {
        self.staging_dir_path().join(self.layout.artifact_name())
    }

    /// Inspects all staged package source artifacts and native payloads.
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

    /// Inspects the final package output file.
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
        NATIVE_PAYLOAD_PATH, PackageArtifactInspectionError, PackageArtifactInspectionPlan,
        PackageArtifactProblem, STAGING_DESCRIPTOR_PATH, STAGING_LOADER_PATH, STAGING_README_PATH,
    };
    use crate::BLE_LOOPBACK_PACKAGE_NAME;
    use crate::package_assets::PackageProvenance;
    use crate::test_support::PackageTestHarness;

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
