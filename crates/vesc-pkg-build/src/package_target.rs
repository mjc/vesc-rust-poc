use std::path::PathBuf;

use crate::package_artifacts::PackageArtifactInspectionError;
use crate::package_build::PackageBuildPlan;
use crate::package_conversion::{PackageBinaryConversionError, PackageBinaryConversionRunner};
use crate::PackageProvenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageTargetMode {
    Package,
    PackageOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageTargetError {
    Stage { path: PathBuf, reason: String },
    Conversion(PackageBinaryConversionError),
    Inspection(PackageArtifactInspectionError),
    PackageOutput { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTargetPlan {
    build_plan: PackageBuildPlan,
    mode: PackageTargetMode,
}

impl PackageTargetPlan {
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        mode: PackageTargetMode,
    ) -> Self {
        Self::with_provenance(
            source_root,
            package_name,
            version,
            PackageProvenance::empty(),
            mode,
        )
    }

    pub fn with_provenance(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
        mode: PackageTargetMode,
    ) -> Self {
        Self {
            build_plan: PackageBuildPlan::with_provenance(
                source_root,
                package_name,
                version,
                provenance,
            ),
            mode,
        }
    }

    pub fn build_plan(&self) -> &PackageBuildPlan {
        &self.build_plan
    }

    pub fn mode(&self) -> PackageTargetMode {
        self.mode
    }

    pub fn package_output_path(&self) -> PathBuf {
        self.build_plan.package_output_path()
    }

    pub fn execute_with<C>(&self, conversion_runner: &C) -> Result<PathBuf, PackageTargetError>
    where
        C: PackageBinaryConversionRunner,
    {
        self.build_plan
            .convert_package_binary_with(conversion_runner)
            .map_err(PackageTargetError::Conversion)?;
        self.build_plan
            .stage_package_assets()
            .map_err(|error| PackageTargetError::Stage {
                path: self.build_plan.inspection_plan().staging_dir_path(),
                reason: error.to_string(),
            })?;
        self.build_plan
            .inspect_package_artifacts()
            .map_err(PackageTargetError::Inspection)?;

        self.build_plan.write_package_output().map_err(|error| {
            PackageTargetError::PackageOutput {
                path: self.build_plan.package_output_path(),
                reason: error.to_string(),
            }
        })?;
        self.build_plan
            .inspect_package_output()
            .map_err(PackageTargetError::Inspection)?;

        Ok(self.package_output_path())
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageTargetMode, PackageTargetPlan};
    use crate::package_conversion::{
        PackageBinaryConversionCommand, PackageBinaryConversionRunner,
    };
    use crate::test_support::PackageTestHarness;
    use crate::BLE_LOOPBACK_PACKAGE_NAME;
    use std::cell::RefCell;
    use std::fs;

    #[derive(Default)]
    struct FakeConversionRunner {
        calls: RefCell<Vec<PackageBinaryConversionCommand>>,
    }

    impl FakeConversionRunner {
        fn calls(&self) -> Vec<PackageBinaryConversionCommand> {
            self.calls.borrow().clone()
        }
    }

    impl PackageBinaryConversionRunner for FakeConversionRunner {
        fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
            self.calls.borrow_mut().push(command.clone());
            if let Some(parent) = command.package_binary_path().parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(command.package_binary_path(), b"payload").map_err(|error| error.to_string())
        }
    }

    #[test]
    fn package_only_stages_inspects_and_writes_the_output() {
        let harness = PackageTestHarness::new();
        let root = harness.root();
        let target = PackageTargetPlan::new(
            root,
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageTargetMode::PackageOnly,
        );
        let conversion_runner = FakeConversionRunner::default();

        let output = target
            .execute_with(&conversion_runner)
            .expect("package target");

        assert_eq!(output, target.package_output_path());
        assert_eq!(
            conversion_runner.calls(),
            vec![target.build_plan().conversion_plan().command()]
        );
        assert!(target.build_plan().inspect_package_artifacts().is_ok());
        assert!(target.build_plan().inspect_package_output().is_ok());
        assert!(root
            .join("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/README.md")
            .exists());
        assert!(root
            .join("target/native-lib-baseline/package_lib.bin")
            .exists());
        assert!(root
            .join("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg")
            .exists());
        let _harness = harness;
    }

    #[test]
    fn package_mode_still_writes_the_package_output() {
        let harness = PackageTestHarness::new();
        let root = harness.root();
        let target = PackageTargetPlan::new(
            root,
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageTargetMode::Package,
        );
        let conversion_runner = FakeConversionRunner::default();

        let output = target
            .execute_with(&conversion_runner)
            .expect("package target");

        assert_eq!(output, target.package_output_path());
        assert!(
            root.join(target.package_output_path()).exists(),
            "expected the package target to materialize the final .vescpkg"
        );
    }

    #[test]
    fn package_output_remains_small_enough_to_upload() {
        let harness = PackageTestHarness::new();
        let root = harness.root();
        let target = PackageTargetPlan::new(
            root,
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageTargetMode::Package,
        );
        let conversion_runner = FakeConversionRunner::default();

        assert_eq!(
            target.execute_with(&conversion_runner),
            Ok(target.package_output_path())
        );
        let size = fs::metadata(root.join(target.package_output_path()))
            .expect("package metadata")
            .len();
        assert!(
            size < 128 * 1024,
            "expected the final package to stay below the VESC upload block limit, got {size} bytes"
        );
    }
}
