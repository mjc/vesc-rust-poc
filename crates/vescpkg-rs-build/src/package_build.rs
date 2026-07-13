use std::path::PathBuf;
use std::{fs, io};

use crate::PackageLayout;
use crate::package_artifacts::PackageArtifactInspectionPlan;
use crate::package_assets::{PackageAssets, PackageProvenance};
use crate::package_conversion::{
    PackageBinaryConversionError, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::package_format::{LispImportPolicy, VescPackageInput, write_vesc_package};

/// Package example artifact profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageExample {
    /// BLE loopback package example.
    Loopback,
}

impl PackageExample {
    /// Return the example source directory.
    pub fn source_path(self) -> PathBuf {
        PathBuf::from("examples/loopback")
    }

    /// Return the built staticlib input path for this example.
    pub fn native_artifact_input_path(self) -> PathBuf {
        PathBuf::from("target/thumbv7em-none-eabihf/release/libvesc_example_loopback.a")
    }

    /// Return the Cargo package that builds this example staticlib.
    pub fn cargo_package_name(self) -> &'static str {
        "vesc-example-loopback"
    }

    /// Return the native build artifact directory for this example.
    pub fn native_build_dir(self) -> PathBuf {
        PathBuf::from("target/native-lib-baseline")
    }
}

/// End-to-end package build plan from source tree to `.vescpkg` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBuildPlan {
    source_root: PathBuf,
    layout: PackageLayout,
    provenance: PackageProvenance,
    example: PackageExample,
}

impl PackageBuildPlan {
    /// Build a plan without provenance metadata.
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::for_example(source_root, package_name, version, PackageExample::Loopback)
    }

    /// Build a plan for a selected package example without provenance metadata.
    pub fn for_example(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        example: PackageExample,
    ) -> Self {
        Self::with_provenance_for_example(
            source_root,
            package_name,
            version,
            PackageProvenance::empty(),
            example,
        )
    }

    /// Build a plan with explicit provenance metadata.
    pub fn with_provenance(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
    ) -> Self {
        Self::with_provenance_for_example(
            source_root,
            package_name,
            version,
            provenance,
            PackageExample::Loopback,
        )
    }

    /// Build a plan with explicit provenance metadata for a selected package example.
    pub fn with_provenance_for_example(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        provenance: PackageProvenance,
        example: PackageExample,
    ) -> Self {
        Self {
            source_root: source_root.into(),
            layout: PackageLayout::new(package_name, version),
            provenance,
            example,
        }
    }

    /// Return the package layout used by this plan.
    pub fn layout(&self) -> &PackageLayout {
        &self.layout
    }

    /// Return the selected package example profile.
    pub fn example(&self) -> PackageExample {
        self.example
    }

    /// Return the example source path.
    pub fn example_source_path(&self) -> PathBuf {
        self.example.source_path()
    }

    /// Return the native staticlib artifact input path.
    pub fn native_artifact_input_path(&self) -> PathBuf {
        self.example.native_artifact_input_path()
    }

    /// Return the rendered package assets for this plan.
    pub fn assets(&self) -> PackageAssets {
        PackageAssets::new(self.layout.clone(), self.provenance.clone())
    }

    /// Render the package assets into the source tree.
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
        let native_payload_path = self.source_root.join(assets.native_payload_path());
        if let Some(parent) = native_payload_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(
            self.conversion_plan().package_binary_path(),
            native_payload_path,
        )?;
        Ok(assets)
    }

    /// Return the native-binary conversion plan.
    pub fn conversion_plan(&self) -> PackageBinaryConversionPlan {
        PackageBinaryConversionPlan::for_example(
            self.source_root.clone(),
            self.layout.package_name(),
            self.layout.version(),
            self.example,
        )
    }

    /// Run the native-binary conversion step with a custom runner.
    pub fn convert_package_binary_with<R: PackageBinaryConversionRunner>(
        &self,
        runner: &R,
    ) -> Result<(), PackageBinaryConversionError> {
        self.conversion_plan().run_with(runner)
    }

    /// Return the package-artifact inspection plan.
    pub fn inspection_plan(&self) -> PackageArtifactInspectionPlan {
        PackageArtifactInspectionPlan::for_example(
            self.source_root.clone(),
            self.layout.package_name(),
            self.layout.version(),
            self.provenance.clone(),
            self.example,
        )
    }

    /// Inspect the staged package artifacts.
    pub fn inspect_package_artifacts(
        &self,
    ) -> Result<(), crate::package_artifacts::PackageArtifactInspectionError> {
        self.inspection_plan().inspect()
    }

    /// Inspect the final package output.
    pub fn inspect_package_output(
        &self,
    ) -> Result<(), crate::package_artifacts::PackageArtifactInspectionError> {
        self.inspection_plan().inspect_package_output()
    }

    /// Write the final `.vescpkg` output file.
    pub fn write_package_output(&self) -> io::Result<PathBuf> {
        let assets = self.assets();
        let staging = self.inspection_plan();
        let readme = fs::read_to_string(staging.readme_path())?;
        let descriptor = fs::read_to_string(staging.descriptor_path())?;
        let loader = fs::read_to_string(staging.loader_path())?;
        let output_path = self.source_root.join(self.package_output_path());
        let loader_path = staging.staging_dir_path();

        let input = VescPackageInput {
            name: assets.package_name(),
            description_md: &readme,
            lisp_source: &loader,
            lisp_editor_path: &loader_path,
            lisp_import_path: None,
            lisp_import_policy: LispImportPolicy::HostPaths,
            qml_file: "",
            pkg_desc_qml: &descriptor,
            qml_is_fullscreen: false,
        };
        write_vesc_package(&output_path, &input)?;

        Ok(output_path)
    }

    /// Return the source files that feed the package output.
    pub fn package_input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            "package/code.lisp",
            "package/pkgdesc.qml",
            "package/README.md",
        ]
        .into_iter()
        .map(move |relative| self.source_root.join(relative))
    }

    /// Return the staged descriptor path.
    pub fn descriptor_path(&self) -> PathBuf {
        self.layout.descriptor_path()
    }

    /// Return the final `.vescpkg` output path.
    pub fn package_output_path(&self) -> PathBuf {
        self.layout.staging_dir().join(self.layout.artifact_name())
    }
}

#[cfg(test)]
mod tests {
    use super::PackageBuildPlan;
    use crate::test_support::{FakeConversionRunner, PackageTestHarness};
    use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageProvenance};

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
                std::path::PathBuf::from(
                    "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/src/package_lib.bin"
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

        let runner = FakeConversionRunner::recording();
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
        assert_eq!(
            readme,
            "Rust BLE loopback test package 0.1.0\ngit abc123\ndate 2026-06-25\n"
        );
        assert!(
            plan.inspection_plan()
                .assets()
                .render_readme()
                .contains("abc123")
        );
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
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let plan = PackageBuildPlan::new(harness.root(), BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");

        let error = plan
            .inspect_package_artifacts()
            .expect_err("missing artifacts");
        assert_eq!(error.problems().len(), 5);
    }
}
