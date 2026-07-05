use std::path::PathBuf;

use crate::PackageLayout;

/// Provenance metadata embedded in generated package assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProvenance {
    git_commit: Option<String>,
    build_date: Option<String>,
}

impl PackageProvenance {
    /// Return empty provenance metadata.
    pub fn empty() -> Self {
        Self {
            git_commit: None,
            build_date: None,
        }
    }

    /// Build provenance metadata from optional git commit and build date.
    pub fn new(
        git_commit: Option<impl Into<String>>,
        build_date: Option<impl Into<String>>,
    ) -> Self {
        Self {
            git_commit: git_commit.map(Into::into),
            build_date: build_date.map(Into::into),
        }
    }

    /// Return the recorded git commit, if any.
    pub fn git_commit(&self) -> Option<&str> {
        self.git_commit.as_deref()
    }

    /// Return the recorded build date, if any.
    pub fn build_date(&self) -> Option<&str> {
        self.build_date.as_deref()
    }
}

/// Rendered package asset paths and file contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageAssets {
    layout: PackageLayout,
    provenance: PackageProvenance,
    profile: PackageAssetProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageAssetProfile {
    Generic,
    Refloat,
}

impl PackageAssets {
    /// Construct the asset set for one package layout and provenance.
    pub fn new(layout: PackageLayout, provenance: PackageProvenance) -> Self {
        Self {
            layout,
            provenance,
            profile: PackageAssetProfile::Generic,
        }
    }

    /// Construct the Refloat asset set for one package layout and provenance.
    pub fn refloat(layout: PackageLayout, provenance: PackageProvenance) -> Self {
        Self {
            layout,
            provenance,
            profile: PackageAssetProfile::Refloat,
        }
    }

    /// Return the package name used by the assets.
    pub fn package_name(&self) -> &str {
        self.layout.package_name()
    }

    /// Return the package version used by the assets.
    pub fn version(&self) -> &str {
        self.layout.version()
    }

    /// Return the package staging directory.
    pub fn staging_dir(&self) -> PathBuf {
        self.layout.staging_dir()
    }

    /// Return the generated README path.
    pub fn readme_path(&self) -> PathBuf {
        self.staging_dir().join("README.md")
    }

    /// Return the generated descriptor path.
    pub fn descriptor_path(&self) -> PathBuf {
        self.staging_dir().join("pkgdesc.qml")
    }

    /// Return the generated loader path.
    pub fn loader_path(&self) -> PathBuf {
        self.staging_dir().join("code.lisp")
    }

    /// Return the generated native payload path.
    pub fn native_payload_path(&self) -> PathBuf {
        self.staging_dir().join("src/package_lib.bin")
    }

    /// Return all generated asset paths.
    pub fn asset_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            self.readme_path(),
            self.descriptor_path(),
            self.loader_path(),
            self.native_payload_path(),
        ]
        .into_iter()
    }

    /// Render the package README contents.
    pub fn render_readme(&self) -> String {
        let mut output = format!("{} {}\n", self.package_name(), self.version());

        if let Some(commit) = self.provenance.git_commit() {
            output.push_str(&format!("git {commit}\n"));
        }
        if let Some(build_date) = self.provenance.build_date() {
            output.push_str(&format!("date {build_date}\n"));
        }

        output
    }

    /// Render the package descriptor QML.
    pub fn render_descriptor(&self) -> String {
        match self.profile {
            PackageAssetProfile::Generic => format!(
                "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"{}\"\n}}\n",
                self.package_name(),
                self.layout.artifact_name()
            ),
            PackageAssetProfile::Refloat => concat!(
                "import QtQuick 2.15\n\n",
                "Item {\n",
                "    property string pkgName: \"Refloat\"\n",
                "    property string pkgDescriptionMd: \"package_README-gen.md\"\n",
                "    property string pkgLisp: \"lisp/package.lisp\"\n",
                "    property string pkgQml: \"ui.qml\"\n",
                "    property bool pkgQmlIsFullscreen: false\n",
                "    property string pkgOutput: \"refloat.vescpkg\"\n\n",
                "    function isCompatible (fwRxParams) {\n",
                "        if (fwRxParams.hwTypeStr().toLowerCase() != \"vesc\") {\n",
                "            return false;\n",
                "        }\n\n",
                "        return true;\n",
                "    }\n",
                "}\n",
            )
            .to_owned(),
        }
    }

    /// Render the loader script that boots the package.
    pub fn render_loader(&self) -> String {
        match self.profile {
            PackageAssetProfile::Generic => concat!(
                "(import \"src/package_lib.bin\" 'package-lib)\n",
                "(print \"vesc-rust-load-v7\")\n",
                "(print (load-native-lib package-lib))\n",
            ),
            PackageAssetProfile::Refloat => {
                "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n"
            }
        }
        .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageAssets, PackageProvenance};
    use crate::{
        BLE_LOOPBACK_PACKAGE_NAME, PackageLayout, REFLOAT_PACKAGE_NAME, REFLOAT_PACKAGE_VERSION,
    };

    #[test]
    fn renders_the_expected_package_assets() {
        let assets = PackageAssets::new(
            PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, "0.1.0"),
            PackageProvenance::new(Some("abc123"), Some("2026-06-25")),
        );

        assert_eq!(
            assets.asset_paths().collect::<Vec<_>>(),
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
            assets.render_readme(),
            "Rust BLE loopback test package 0.1.0\ngit abc123\ndate 2026-06-25\n"
        );
        let descriptor = assets.render_descriptor();
        assert!(descriptor.contains("pkgName: \"Rust BLE loopback test package\""));
        assert!(descriptor.contains("pkgDescriptionMd: \"README.md\""));
        assert!(descriptor.contains("pkgLisp: \"code.lisp\""));
        assert!(descriptor.contains("pkgOutput: \"Rust-BLE-loopback-test-package-0.1.0.vescpkg\""));
        assert!(
            !descriptor.contains("packageName"),
            "expected vesc_tool schema, not legacy POC dialect"
        );
        assert_eq!(
            assets.render_loader(),
            "(import \"src/package_lib.bin\" 'package-lib)\n(print \"vesc-rust-load-v7\")\n(print (load-native-lib package-lib))\n"
        );
        assert_eq!(assets.render_loader().matches("load-native-lib").count(), 1);
        assert!(assets.render_loader().contains("vesc-rust-load-v7"));
        assert!(!assets.render_loader().contains("(loopwhile t"));
        assert!(!assets.render_loader().contains("(sleep 1.0)"));
        assert!(!assets.render_loader().contains("event-data-rx"));
        assert!(!assets.render_loader().contains("send-data"));
        assert!(
            !assets.render_loader().contains("ext-rust-add"),
            "expected the BLE loopback package loader to only load the native library"
        );

        let refloat_assets = PackageAssets::refloat(
            PackageLayout::new(REFLOAT_PACKAGE_NAME, REFLOAT_PACKAGE_VERSION),
            PackageProvenance::empty(),
        );
        assert_eq!(
            refloat_assets.render_loader(),
            "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n"
        );
        assert!(!refloat_assets.render_loader().contains("vesc-rust-load-v7"));
        assert!(!refloat_assets.render_loader().contains("(print"));
    }
}
