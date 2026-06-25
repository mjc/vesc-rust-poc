use std::path::PathBuf;
use std::{fs, io};

use crate::package_artifacts::PackageArtifactInspectionPlan;
use crate::package_assets::{PackageAssets, PackageProvenance};
use crate::package_conversion::{
    PackageBinaryConversionError, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::package_format::{write_vesc_package, VescPackageInput};
use crate::PackageLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBuildPlan {
    source_root: PathBuf,
    layout: PackageLayout,
    provenance: PackageProvenance,
}

impl PackageBuildPlan {
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::with_provenance(
            source_root,
            package_name,
            version,
            PackageProvenance::empty(),
        )
    }

    pub fn with_provenance(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
    ) -> Self {
        Self {
            source_root: source_root.into(),
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
            self.provenance.clone(),
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

    pub fn write_package_output(&self) -> io::Result<PathBuf> {
        let assets = self.assets();
        let staging = self.inspection_plan();
        let readme = fs::read_to_string(staging.readme_path())?;
        let descriptor = fs::read_to_string(staging.descriptor_path())?;
        let loader = fs::read_to_string(staging.loader_path())?;
        let output_path = self.source_root.join(self.package_output_path());
        let loader_path = self.source_root.join("package");

        let input = VescPackageInput {
            name: assets.package_name(),
            description_md: &readme,
            lisp_source: &loader,
            lisp_editor_path: &loader_path,
            qml_file: "",
            pkg_desc_qml: &descriptor,
            qml_is_fullscreen: false,
        };
        write_vesc_package(&output_path, &input)?;

        Ok(output_path)
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
}

#[cfg(test)]
mod tests {
    use super::PackageBuildPlan;
    use crate::package_conversion::{
        PackageBinaryConversionCommand, PackageBinaryConversionRunner,
    };
    use crate::{PackageProvenance, BLE_LOOPBACK_PACKAGE_NAME};
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
                "fixtures/native-lib-baseline/scripts/conv.py".to_owned(),
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
    fn renders_package_provenance_through_the_build_plan() {
        let plan = PackageBuildPlan::with_provenance(
            "fixtures/native-lib-baseline",
            BLE_LOOPBACK_PACKAGE_NAME,
            "0.1.0",
            PackageProvenance::new(Some("abc123"), Some("2026-06-25")),
        );

        let readme = plan.assets().render_readme();
        assert!(readme.contains("Version: 0.1.0"));
        assert!(readme.contains("Git commit: abc123"));
        assert!(readme.contains("Build date: 2026-06-25"));
        assert!(plan
            .inspection_plan()
            .assets()
            .render_readme()
            .contains("abc123"));
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
    fn writes_the_expected_package_output() {
        let root = unique_root();
        let plan = PackageBuildPlan::new(&root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");
        plan.stage_package_assets().expect("staged assets");
        fs::create_dir_all(root.join("target/native-lib-baseline")).expect("native payload dir");
        fs::write(
            root.join("target/native-lib-baseline/package_lib.bin"),
            b"payload",
        )
        .expect("native payload");

        let output = plan.write_package_output().expect("package output");

        assert_eq!(output, root.join(plan.package_output_path()));
        assert!(output.exists(), "expected the final .vescpkg to exist");
        assert!(fs::metadata(&output).expect("package metadata").len() > 0);
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
