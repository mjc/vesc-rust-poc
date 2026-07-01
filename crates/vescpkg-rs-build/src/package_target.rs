use std::path::PathBuf;

use crate::PackageProvenance;
use crate::package_artifacts::PackageArtifactInspectionError;
use crate::package_build::{PackageBuildPlan, PackageExample};
use crate::package_conversion::{PackageBinaryConversionError, PackageBinaryConversionRunner};

/// The two supported package target modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageTargetMode {
    /// Build a package and keep the package-only artifacts.
    Package,
    /// Build a package without any extra target-specific packaging.
    PackageOnly,
}

/// Errors returned while staging or materializing a package target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageTargetError {
    /// Preparing the staging directory failed.
    Stage {
        /// Path that could not be staged.
        path: PathBuf,
        /// Human-readable staging failure reason.
        reason: String,
    },
    /// Converting the native binary into a package payload failed.
    Conversion(PackageBinaryConversionError),
    /// Inspecting staged artifacts or package output failed.
    Inspection(PackageArtifactInspectionError),
    /// Writing or validating the final package output failed.
    PackageOutput {
        /// Output path that failed.
        path: PathBuf,
        /// Human-readable package-output failure reason.
        reason: String,
    },
}

/// End-to-end package build plan with target mode and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTargetPlan {
    build_plan: PackageBuildPlan,
    mode: PackageTargetMode,
}

impl PackageTargetPlan {
    /// Build a target plan without provenance metadata.
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        mode: PackageTargetMode,
    ) -> Self {
        Self::for_example(
            source_root,
            package_name,
            version,
            PackageExample::Loopback,
            mode,
        )
    }

    /// Build a target plan for a selected package example without provenance metadata.
    pub fn for_example(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        example: PackageExample,
        mode: PackageTargetMode,
    ) -> Self {
        Self::with_provenance_for_example(
            source_root,
            package_name,
            version,
            PackageProvenance::empty(),
            example,
            mode,
        )
    }

    /// Build a target plan with explicit provenance metadata.
    pub fn with_provenance(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
        mode: PackageTargetMode,
    ) -> Self {
        Self::with_provenance_for_example(
            source_root,
            package_name,
            version,
            provenance,
            PackageExample::Loopback,
            mode,
        )
    }

    /// Build a target plan with explicit provenance metadata for a selected package example.
    pub fn with_provenance_for_example(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
        example: PackageExample,
        mode: PackageTargetMode,
    ) -> Self {
        Self {
            build_plan: PackageBuildPlan::with_provenance_for_example(
                source_root,
                package_name,
                version,
                provenance,
                example,
            ),
            mode,
        }
    }

    /// Return the underlying package build plan.
    pub fn build_plan(&self) -> &PackageBuildPlan {
        &self.build_plan
    }

    /// Return the selected target mode.
    pub fn mode(&self) -> PackageTargetMode {
        self.mode
    }

    /// Return the final package output path.
    pub fn package_output_path(&self) -> PathBuf {
        self.build_plan.package_output_path()
    }

    /// Execute the target plan using the supplied conversion runner.
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
    use crate::BLE_LOOPBACK_PACKAGE_NAME;

    #[test]
    fn package_target_plan_delegates_output_path_and_mode() {
        let package_only = PackageTargetPlan::new(
            "fixtures/native-lib-baseline",
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageTargetMode::PackageOnly,
        );
        let package = PackageTargetPlan::new(
            "fixtures/native-lib-baseline",
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageTargetMode::Package,
        );

        assert_eq!(package_only.mode(), PackageTargetMode::PackageOnly);
        assert_eq!(package.mode(), PackageTargetMode::Package);
        assert_eq!(
            package_only.package_output_path(),
            package_only.build_plan().package_output_path()
        );
        assert_eq!(
            package.package_output_path(),
            package.build_plan().package_output_path()
        );
    }
}
