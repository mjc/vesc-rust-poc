use std::path::PathBuf;
use std::{fs, io};

use crate::package_artifacts::PackageArtifactInspectionPlan;
use crate::package_assets::{PackageAssets, PackageProvenance};
use crate::package_conversion::{
    PackageBinaryConversionError, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
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

    pub fn stage_package_assets(&self) -> io::Result<PackageAssets> {
        let assets = self.assets();
        let staging_dir = self.source_root.join(assets.staging_dir());
        let native_dir = self.source_root.join("target/native-lib-baseline");

        fs::create_dir_all(&staging_dir)?;
        fs::create_dir_all(&native_dir)?;
        fs::write(
            self.source_root.join(assets.readme_path()),
            assets.render_readme(),
        )?;
        fs::write(
            self.source_root.join(assets.descriptor_path()),
            assets.render_descriptor(),
        )?;
        fs::write(
            self.source_root.join(assets.loader_path()),
            assets.render_loader(),
        )?;
        Ok(assets)
    }

    pub fn conversion_plan(&self) -> PackageBinaryConversionPlan {
        PackageBinaryConversionPlan::new(
            self.source_root.clone(),
            self.layout.package_name(),
            self.layout.version(),
        )
    }

    pub fn convert_package_binary_with<R: PackageBinaryConversionRunner>(
        &self,
        runner: &R,
    ) -> Result<(), PackageBinaryConversionError> {
        self.conversion_plan().run_with(runner)
    }

    pub fn inspection_plan(&self) -> PackageArtifactInspectionPlan {
        PackageArtifactInspectionPlan::new(
            self.source_root.clone(),
            self.layout.package_name(),
            self.layout.version(),
            PackageProvenance::empty(),
        )
    }

    pub fn inspect_package_artifacts(
        &self,
    ) -> Result<(), crate::package_artifacts::PackageArtifactInspectionError> {
        self.inspection_plan().inspect()
    }

    pub fn inspect_package_output(
        &self,
    ) -> Result<(), crate::package_artifacts::PackageArtifactInspectionError> {
        self.inspection_plan().inspect_package_output()
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
    use crate::package_conversion::{
        PackageBinaryConversionCommand, PackageBinaryConversionRunner,
    };
    use crate::BLE_LOOPBACK_PACKAGE_NAME;
    use std::cell::RefCell;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct FakeRunner {
        calls: RefCell<Vec<PackageBinaryConversionCommand>>,
    }

    impl FakeRunner {
        fn calls(&self) -> Vec<PackageBinaryConversionCommand> {
            self.calls.borrow().clone()
        }
    }

    impl PackageBinaryConversionRunner for FakeRunner {
        fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
            self.calls.borrow_mut().push(command.clone());
            Ok(())
        }
    }

    fn unique_root() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vesc-rust-poc-build-{nanos}-{}",
            std::process::id()
        ))
    }

    fn write_artifact(root: &std::path::Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("artifact parent directory");
        }
        fs::write(path, contents).expect("artifact contents");
    }

    #[test]
    fn renders_the_expected_package_build_plan() {
        let plan = PackageBuildPlan::new(
            "fixtures/native-lib-baseline",
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
        );

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
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            plan.vesc_tool_args(),
            vec![
                "--buildPkgFromDesc".to_owned(),
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/pkgdesc.qml".to_owned(),
            ]
        );
        assert_eq!(
            plan.assets().asset_paths().collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from(
                    "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/README.md"
                ),
                std::path::PathBuf::from(
                    "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/pkgdesc.qml"
                ),
                std::path::PathBuf::from(
                    "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/code.lisp"
                ),
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

        let runner = FakeRunner::default();
        assert_eq!(plan.convert_package_binary_with(&runner), Ok(()));
        assert_eq!(runner.calls(), vec![plan.conversion_plan().command()]);
    }

    #[test]
    fn renders_the_expected_package_artifact_inspection_plan() {
        let plan = PackageBuildPlan::new(
            "fixtures/native-lib-baseline",
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
        );
        let inspection_plan = plan.inspection_plan();

        assert_eq!(
            inspection_plan.package_output_path(),
            std::path::PathBuf::from(
                "fixtures/native-lib-baseline/target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/"
            )
            .join("Rust-BLE-loopback-test-package-0.1.0.vescpkg")
        );
    }

    #[test]
    fn inspect_package_artifacts_reports_missing_staging_files() {
        let root = unique_root();
        let plan = PackageBuildPlan::new(&root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");
        fs::create_dir_all(plan.inspection_plan().staging_dir_path()).expect("staging root");

        let error = plan
            .inspect_package_artifacts()
            .expect_err("missing artifacts");
        assert_eq!(error.problems().len(), 4);
    }

    #[test]
    fn inspect_package_artifacts_accepts_rendered_artifacts() {
        let root = unique_root();
        let plan = PackageBuildPlan::new(&root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");
        let inspection_plan = plan.inspection_plan();
        let assets = inspection_plan.assets();

        write_artifact(
            &inspection_plan.staging_dir_path(),
            "README.md",
            &assets.render_readme(),
        );
        write_artifact(
            &inspection_plan.staging_dir_path(),
            "pkgdesc.qml",
            &assets.render_descriptor(),
        );
        write_artifact(
            &inspection_plan.staging_dir_path(),
            "code.lisp",
            &assets.render_loader(),
        );
        write_artifact(
            &root,
            "target/native-lib-baseline/package_lib.bin",
            "payload",
        );

        assert_eq!(plan.inspect_package_artifacts(), Ok(()));
    }
}
