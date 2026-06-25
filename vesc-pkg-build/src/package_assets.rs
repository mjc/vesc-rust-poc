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

    pub fn asset_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            self.readme_path(),
            self.descriptor_path(),
            self.loader_path(),
        ]
        .into_iter()
    }

    pub fn render_readme(&self) -> String {
        let mut output = format!("# {}\n\nVersion: {}\n", self.package_name(), self.version());

        if let Some(commit) = self.provenance.git_commit() {
            output.push_str(&format!("Git commit: {}\n", commit));
        }
        if let Some(build_date) = self.provenance.build_date() {
            output.push_str(&format!("Build date: {}\n", build_date));
        }

        output.push_str(
            "\nThis package contains the Rust-backed VESC native library proof and its loader assets.\n",
        );
        output
    }

    pub fn render_descriptor(&self) -> String {
        format!(
            "import QtQuick 2.15\n\nItem {{\n    property string packageName: \"{}\"\n    property string packageVersion: \"{}\"\n    property string nativeLibraryPath: \"src/package_lib.bin\"\n}}\n",
            self.package_name(),
            self.version()
        )
    }

    pub fn render_loader(&self) -> String {
        format!(
            "; Auto-generated loader for {}\n(load-native-lib \"src/package_lib.bin\")\n",
            self.package_name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageAssets, PackageProvenance};
    use crate::PackageLayout;

    #[test]
    fn renders_the_expected_package_assets() {
        let assets = PackageAssets::new(
            PackageLayout::new("Rust VESC package", "0.1.0"),
            PackageProvenance::new(Some("abc123"), Some("2026-06-25")),
        );

        assert_eq!(
            assets.asset_paths().collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/README.md"),
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml"),
                std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/code.lisp"),
            ]
        );
        assert!(assets.render_readme().contains("Version: 0.1.0"));
        assert!(assets.render_readme().contains("Git commit: abc123"));
        assert!(assets.render_readme().contains("Build date: 2026-06-25"));
        assert!(assets
            .render_descriptor()
            .contains("packageName: \"Rust VESC package\""));
        assert!(assets
            .render_descriptor()
            .contains("nativeLibraryPath: \"src/package_lib.bin\""));
        assert!(assets
            .render_loader()
            .contains("(load-native-lib \"src/package_lib.bin\")"));
    }
}
