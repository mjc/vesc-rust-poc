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
