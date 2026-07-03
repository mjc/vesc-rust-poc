use std::path::{Path, PathBuf};

use crate::package::PackageError;

/// Package descriptor fields read from `pkgdesc.qml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    name: String,
    description_md: String,
    lisp: String,
    qml: String,
    qml_is_fullscreen: bool,
    output: String,
}

impl PackageManifest {
    /// Return the VESC package name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the markdown description path from the package descriptor.
    pub fn description_md(&self) -> &str {
        &self.description_md
    }

    /// Return the Lisp loader path from the package descriptor.
    pub fn lisp(&self) -> &str {
        &self.lisp
    }

    /// Return the QML UI path from the package descriptor.
    pub fn qml(&self) -> &str {
        &self.qml
    }

    /// Return whether the QML UI should be fullscreen.
    pub fn qml_is_fullscreen(&self) -> bool {
        self.qml_is_fullscreen
    }

    /// Return the output `.vescpkg` path from the package descriptor.
    pub fn output(&self) -> &str {
        &self.output
    }
}

/// Read package descriptor fields from a `pkgdesc.qml` file.
pub fn parse_package_manifest(path: &Path) -> Result<PackageManifest, PackageError> {
    let text = std::fs::read_to_string(path).map_err(PackageError::Io)?;
    Ok(PackageManifest {
        name: extract_qml_string_property(&text, "pkgName")?,
        description_md: extract_qml_string_property(&text, "pkgDescriptionMd")?,
        lisp: extract_qml_string_property(&text, "pkgLisp")?,
        qml: extract_qml_string_property(&text, "pkgQml")?,
        qml_is_fullscreen: extract_qml_bool_property(&text, "pkgQmlIsFullscreen")?,
        output: extract_qml_string_property(&text, "pkgOutput")?,
    })
}

/// Read the package name and version from a `pkgdesc.qml` file.
pub fn parse_pkgdesc(path: &Path) -> Result<(String, String), PackageError> {
    let text = std::fs::read_to_string(path).map_err(PackageError::Io)?;
    let name = extract_qml_string_property(&text, "pkgName")?;
    let output = extract_qml_string_property(&text, "pkgOutput")?;
    let version = version_from_pkg_output(&output)?;
    Ok((name, version))
}

/// Return the canonical `pkgdesc.qml` path for a manifest or directory.
pub fn manifest_path(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|name| name == "pkgdesc.qml") {
        path.to_path_buf()
    } else {
        path.join("pkgdesc.qml")
    }
}

/// Return the staging directory that contains the manifest.
pub fn staging_dir_from_manifest(path: &Path) -> Result<PathBuf, PackageError> {
    let manifest = manifest_path(path);
    manifest
        .parent()
        .map(Path::to_path_buf)
        .ok_or(PackageError::InvalidPackage)
}

fn extract_qml_string_property(text: &str, key: &str) -> Result<String, PackageError> {
    let needle = format!("property string {key}: \"");
    let bytes = text.as_bytes();
    let start = text.find(&needle).ok_or(PackageError::InvalidPackage)? + needle.len();
    let end = bytes[start..]
        .iter()
        .position(|&byte| byte == b'"')
        .ok_or(PackageError::InvalidPackage)?;
    String::from_utf8(bytes[start..start + end].to_vec()).map_err(|_| PackageError::InvalidPackage)
}

fn extract_qml_bool_property(text: &str, key: &str) -> Result<bool, PackageError> {
    let needle = format!("property bool {key}: ");
    let value = text
        .find(&needle)
        .and_then(|start| text[start + needle.len()..].lines().next())
        .map(str::trim)
        .ok_or(PackageError::InvalidPackage)?;
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(PackageError::InvalidPackage),
    }
}

fn version_from_pkg_output(output: &str) -> Result<String, PackageError> {
    let stem = output.strip_suffix(".vescpkg").unwrap_or(output);
    let version = stem
        .rsplit_once('-')
        .map(|(_, version)| version.to_owned())
        .ok_or(PackageError::InvalidPackage)?;
    if version.is_empty() {
        Err(PackageError::InvalidPackage)
    } else {
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_package_manifest, parse_pkgdesc, version_from_pkg_output};
    use crate::test_support::PackageTestHarness;

    #[test]
    fn parses_pkgdesc_name_and_version() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\nItem {\n    property string pkgName: \"Demo package\"\n    property string pkgOutput: \"Demo-package-1.2.3.vescpkg\"\n}\n",
        )
        .unwrap();

        let (name, version) = parse_pkgdesc(&manifest).expect("manifest");
        assert_eq!(name, "Demo package");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn extracts_version_from_pkg_output_name() {
        assert_eq!(
            version_from_pkg_output("Rust-BLE-loopback-test-package-0.1.0.vescpkg").unwrap(),
            "0.1.0"
        );
    }

    #[test]
    fn parses_refloat_style_package_asset_paths() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();

        let package = parse_package_manifest(&manifest).expect("package manifest");
        assert_eq!(package.name(), "Refloat");
        assert_eq!(package.description_md(), "package_README-gen.md");
        assert_eq!(package.lisp(), "lisp/package.lisp");
        assert_eq!(package.qml(), "ui.qml");
        assert!(!package.qml_is_fullscreen());
        assert_eq!(package.output(), "refloat.vescpkg");
    }
}
