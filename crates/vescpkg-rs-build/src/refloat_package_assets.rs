use std::path::{Path, PathBuf};

use crate::{Package, PackageError};

/// Fixed build metadata used when rendering Refloat's generated package assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatBuildInfo {
    build_date: String,
    git_commit: String,
}

impl RefloatBuildInfo {
    /// Create build metadata matching Refloat's Makefile-generated README fields.
    pub fn new(build_date: impl Into<String>, git_commit: impl Into<String>) -> Self {
        Self {
            build_date: build_date.into(),
            git_commit: git_commit.into(),
        }
    }
}

/// Refloat source-tree asset generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatSourceAssets {
    source_root: PathBuf,
}

impl RefloatSourceAssets {
    /// Create a generator rooted at a Refloat source checkout.
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self {
            source_root: source_root.into(),
        }
    }

    /// Render and write Refloat's generated package README and UI files.
    pub fn materialize_generated_inputs(
        &self,
        build_info: &RefloatBuildInfo,
    ) -> Result<RefloatGeneratedAssets, PackageError> {
        let generated = RefloatGeneratedAssets::new(&self.source_root);
        std::fs::write(generated.readme_path(), self.render_readme(build_info)?)?;
        std::fs::write(generated.ui_path(), self.render_ui()?)?;
        Ok(generated)
    }

    /// Materialize generated inputs and write the Refloat `.vescpkg` artifact.
    pub fn write_package(&self, build_info: &RefloatBuildInfo) -> Result<PathBuf, PackageError> {
        self.materialize_generated_inputs(build_info)?;
        Package::write_from_manifest(self.source_root.join("pkgdesc.qml"))
    }

    fn render_readme(&self, build_info: &RefloatBuildInfo) -> Result<String, PackageError> {
        let readme = self.read_text("package_README.md")?;
        let version = self.read_trimmed("version")?;

        Ok(format!(
            "{readme}\n### Build Info\n- Version: {version}\n- Build Date: {}\n- Git Commit: #{}\n",
            build_info.build_date, build_info.git_commit
        ))
    }

    fn render_ui(&self) -> Result<String, PackageError> {
        let template = self.read_text("ui.qml.in")?;
        let package_name = truncate_chars(&self.read_trimmed("package_name")?, 20);
        let version = self.read_trimmed("version")?;

        Ok(template
            .replace("{{PACKAGE_NAME}}", &package_name)
            .replace("{{VERSION}}", &version))
    }

    fn read_text(&self, relative_path: &str) -> Result<String, PackageError> {
        std::fs::read_to_string(self.source_root.join(relative_path)).map_err(Into::into)
    }

    fn read_trimmed(&self, relative_path: &str) -> Result<String, PackageError> {
        Ok(self.read_text(relative_path)?.trim().to_owned())
    }
}

/// Paths written by Refloat source asset generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatGeneratedAssets {
    source_root: PathBuf,
}

impl RefloatGeneratedAssets {
    fn new(source_root: impl AsRef<Path>) -> Self {
        Self {
            source_root: source_root.as_ref().to_path_buf(),
        }
    }

    /// Return the generated package README path.
    pub fn readme_path(&self) -> PathBuf {
        self.source_root.join("package_README-gen.md")
    }

    /// Return the generated QML UI path.
    pub fn ui_path(&self) -> PathBuf {
        self.source_root.join("ui.qml")
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::{RefloatBuildInfo, RefloatSourceAssets};
    use crate::Package;
    use crate::package_wire::parse_lisp_imports;
    use crate::test_support::PackageTestHarness;

    #[test]
    fn materializes_refloat_makefile_generated_readme_and_ui() {
        let harness = PackageTestHarness::new()
            .write_text(
                "package_README.md",
                "# Refloat\n\nGenerated package documentation.\n",
            )
            .write_text("package_name", "Refloat Long Package Name\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "Item {\n    property string title: \"{{PACKAGE_NAME}}\"\n    property string version: \"{{VERSION}}\"\n}\n",
            );
        let root = harness.root();

        let generated = RefloatSourceAssets::new(root)
            .materialize_generated_inputs(&RefloatBuildInfo::new(
                "2026-07-02 06:00:00-06:00",
                "0ef6e99",
            ))
            .expect("generated inputs");

        assert_eq!(generated.readme_path(), root.join("package_README-gen.md"));
        assert_eq!(generated.ui_path(), root.join("ui.qml"));
        assert_eq!(
            std::fs::read_to_string(generated.readme_path()).expect("generated readme"),
            "# Refloat\n\nGenerated package documentation.\n\n### Build Info\n- Version: 1.2.1\n- Build Date: 2026-07-02 06:00:00-06:00\n- Git Commit: #0ef6e99\n"
        );
        assert_eq!(
            std::fs::read_to_string(generated.ui_path()).expect("generated ui"),
            "Item {\n    property string title: \"Refloat Long Package\"\n    property string version: \"1.2.1\"\n}\n"
        );
    }

    #[test]
    fn writes_refloat_package_from_generated_assets_and_existing_native_payload() {
        let harness = PackageTestHarness::new()
            .write_text("package_README.md", "# Refloat\n")
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "Item { property string title: \"{{PACKAGE_NAME}} {{VERSION}}\" }\n",
            )
            .write_text(
                "pkgdesc.qml",
                "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
            )
            .write_text(
                "lisp/package.lisp",
                "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n",
            )
            .write_bytes("src/package_lib.bin", b"refloat-native\0");
        let root = harness.root();

        let output = RefloatSourceAssets::new(root)
            .write_package(&RefloatBuildInfo::new(
                "2026-07-02 06:00:00-06:00",
                "0ef6e99",
            ))
            .expect("refloat package");

        assert_eq!(output, root.join("refloat.vescpkg"));
        let package = Package::read(&output).expect("written package");
        assert_eq!(package.name, "Refloat");
        assert!(package.description_md.contains("- Version: 1.2.1"));
        assert_eq!(
            package.qml_file,
            "Item { property string title: \"Refloat 1.2.1\" }\n"
        );
        let (_code, imports) = parse_lisp_imports(&package.lisp_data).expect("lisp imports");
        assert_eq!(imports[0].payload, b"refloat-native\0");
    }
}
