use std::path::PathBuf;

use crate::PackageLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProvenance {
    git_commit: Option<String>,
    build_date: Option<String>,
}

impl PackageProvenance {
    pub fn empty() -> Self {
        Self {
            git_commit: None,
            build_date: None,
        }
    }

    pub fn new(
        git_commit: Option<impl Into<String>>,
        build_date: Option<impl Into<String>>,
    ) -> Self {
        Self {
            git_commit: git_commit.map(Into::into),
            build_date: build_date.map(Into::into),
        }
    }

    pub fn git_commit(&self) -> Option<&str> {
        self.git_commit.as_deref()
    }

    pub fn build_date(&self) -> Option<&str> {
        self.build_date.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageAssets {
    layout: PackageLayout,
    provenance: PackageProvenance,
}

impl PackageAssets {
    pub fn new(layout: PackageLayout, provenance: PackageProvenance) -> Self {
        Self { layout, provenance }
    }

    pub fn package_name(&self) -> &str {
        self.layout.package_name()
    }

    pub fn version(&self) -> &str {
        self.layout.version()
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.layout.staging_dir()
    }

    pub fn readme_path(&self) -> PathBuf {
        self.staging_dir().join("README.md")
    }

    pub fn descriptor_path(&self) -> PathBuf {
        self.staging_dir().join("pkgdesc.qml")
    }

    pub fn loader_path(&self) -> PathBuf {
        self.staging_dir().join("code.lisp")
    }

    pub fn native_payload_path(&self) -> PathBuf {
        self.staging_dir().join("src/package_lib.bin")
    }

    pub fn asset_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            self.readme_path(),
            self.descriptor_path(),
            self.loader_path(),
            self.native_payload_path(),
        ]
        .into_iter()
    }

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

    pub fn render_descriptor(&self) -> String {
        format!(
            "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"{}\"\n}}\n",
            self.package_name(),
            self.layout.artifact_name()
        )
    }

    pub fn render_loader(&self) -> String {
        "; Auto-generated loader for the Rust BLE loopback test package.\n(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageAssets, PackageProvenance};
    use crate::{PackageLayout, BLE_LOOPBACK_PACKAGE_NAME};

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
            "; Auto-generated loader for the Rust BLE loopback test package.\n(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n"
        );
        let loader = assets.render_loader();
        assert_eq!(loader.matches("load-native-lib").count(), 1);
        assert!(!loader.contains("vesc-rust-load-v7"));
        assert!(!loader.contains("(loopwhile t"));
        assert!(!loader.contains("(sleep 1.0)"));
        assert!(
            !loader.contains("ext-rust-add"),
            "expected the BLE loopback package loader to only load the native library"
        );
    }
}
