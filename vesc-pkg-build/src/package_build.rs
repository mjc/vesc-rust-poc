use std::path::PathBuf;

use crate::package_assets::{PackageAssets, PackageProvenance};
use crate::package_conversion::PackageBinaryConversionPlan;
use crate::PackageLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBuildPlan {
    source_root: PathBuf,
    layout: PackageLayout,
}

impl PackageBuildPlan {
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            source_root: source_root.into(),
            layout: PackageLayout::new(package_name, version),
        }
    }

    pub fn layout(&self) -> &PackageLayout {
        &self.layout
    }

    pub fn assets(&self) -> PackageAssets {
        PackageAssets::new(self.layout.clone(), PackageProvenance::empty())
    }

    pub fn conversion_plan(&self) -> PackageBinaryConversionPlan {
        PackageBinaryConversionPlan::new(
            self.source_root.clone(),
            self.layout.package_name(),
            self.layout.version(),
        )
    }

    pub fn package_input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            "package/code.lisp",
            "package/pkgdesc.qml",
            "package/README.md",
        ]
        .into_iter()
        .map(move |relative| self.source_root.join(relative))
    }

    pub fn descriptor_path(&self) -> PathBuf {
        self.layout.descriptor_path()
    }

    pub fn package_output_path(&self) -> PathBuf {
        self.layout.staging_dir().join(self.layout.artifact_name())
    }

    pub fn vesc_tool_args(&self) -> Vec<String> {
        vec![
            "--buildPkgFromDesc".to_owned(),
            self.descriptor_path().to_string_lossy().into_owned(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::PackageBuildPlan;

    #[test]
    fn renders_the_expected_package_build_plan() {
        let plan =
            PackageBuildPlan::new("fixtures/native-lib-baseline", "Rust VESC package", "0.1.0");

        assert_eq!(
            plan.package_input_paths().collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from("fixtures/native-lib-baseline/package/code.lisp"),
                std::path::PathBuf::from("fixtures/native-lib-baseline/package/pkgdesc.qml"),
                std::path::PathBuf::from("fixtures/native-lib-baseline/package/README.md"),
            ]
        );
        assert_eq!(
            plan.package_output_path(),
            std::path::PathBuf::from(
                "target/vescpkg/Rust-VESC-package-0.1.0/Rust-VESC-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            plan.vesc_tool_args(),
            vec![
                "--buildPkgFromDesc".to_owned(),
                "target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml".to_owned(),
            ]
        );
        assert_eq!(
            plan.assets().asset_paths().collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/README.md"),
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml"),
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/code.lisp"),
            ]
        );
        assert_eq!(
            plan.conversion_plan().conversion_command_args(),
            vec![
                "fixtures/native-lib-baseline/src/conv.py".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/package_lib.bin"
                    .to_owned(),
            ]
        );
    }
}
