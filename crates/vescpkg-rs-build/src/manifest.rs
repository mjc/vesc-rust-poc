use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::package::PackageError;
use tree_sitter::{Node, Parser};

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
    let properties = qml_root_properties(&text)?;
    Ok(PackageManifest {
        name: required_string_property(&properties, "pkgName")?,
        description_md: required_string_property(&properties, "pkgDescriptionMd")?,
        lisp: required_string_property(&properties, "pkgLisp")?,
        qml: required_string_property(&properties, "pkgQml")?,
        qml_is_fullscreen: required_bool_property(&properties, "pkgQmlIsFullscreen")?,
        output: required_string_property(&properties, "pkgOutput")?,
    })
}

/// Read the package name and version from a `pkgdesc.qml` file.
pub fn parse_pkgdesc(path: &Path) -> Result<(String, String), PackageError> {
    let text = std::fs::read_to_string(path).map_err(PackageError::Io)?;
    let properties = qml_root_properties(&text)?;
    let name = required_string_property(&properties, "pkgName")?;
    let output = required_string_property(&properties, "pkgOutput")?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum QmlPropertyValue {
    String(String),
    Bool(bool),
}

fn qml_root_properties(text: &str) -> Result<BTreeMap<String, QmlPropertyValue>, PackageError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_qmljs::LANGUAGE.into())
        .map_err(|_| PackageError::InvalidPackage)?;
    let tree = parser
        .parse(text, None)
        .ok_or(PackageError::InvalidPackage)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(PackageError::InvalidPackage);
    }

    let root_object =
        find_descendant(root, "ui_object_definition").ok_or(PackageError::InvalidPackage)?;
    let initializer = root_object
        .child_by_field_name("initializer")
        .ok_or(PackageError::InvalidPackage)?;
    let mut properties = BTreeMap::new();
    collect_root_properties(initializer, text.as_bytes(), &mut properties)?;
    Ok(properties)
}

fn collect_root_properties(
    node: Node<'_>,
    source: &[u8],
    properties: &mut BTreeMap<String, QmlPropertyValue>,
) -> Result<(), PackageError> {
    if node.kind() == "ui_property" {
        let Some((name, value)) = parse_qml_property(node, source)? else {
            return Ok(());
        };
        properties.insert(name, value);
        return Ok(());
    }
    if node.kind() == "ui_object_definition" || node.kind() == "ui_object_definition_binding" {
        return Ok(());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_root_properties(child, source, properties)?;
    }
    Ok(())
}

fn parse_qml_property(
    node: Node<'_>,
    source: &[u8],
) -> Result<Option<(String, QmlPropertyValue)>, PackageError> {
    let type_node = node
        .child_by_field_name("type")
        .ok_or(PackageError::InvalidPackage)?;
    let name_node = node
        .child_by_field_name("name")
        .ok_or(PackageError::InvalidPackage)?;
    let Some(value_node) = node.child_by_field_name("value") else {
        return Ok(None);
    };

    let property_type = type_node
        .utf8_text(source)
        .map_err(|_| PackageError::InvalidPackage)?;
    let name = name_node
        .utf8_text(source)
        .map_err(|_| PackageError::InvalidPackage)?
        .to_owned();
    match property_type {
        "string" => {
            let string_node =
                find_descendant(value_node, "string").ok_or(PackageError::InvalidPackage)?;
            Ok(Some((
                name,
                QmlPropertyValue::String(qml_static_string_value(string_node, source)?),
            )))
        }
        "bool" => {
            let value = find_descendant(value_node, "true")
                .map(|_| true)
                .or_else(|| find_descendant(value_node, "false").map(|_| false))
                .ok_or(PackageError::InvalidPackage)?;
            Ok(Some((name, QmlPropertyValue::Bool(value))))
        }
        _ => Ok(None),
    }
}

fn qml_static_string_value(node: Node<'_>, source: &[u8]) -> Result<String, PackageError> {
    let raw = node
        .utf8_text(source)
        .map_err(|_| PackageError::InvalidPackage)?;
    let Some(quote) = raw.chars().next() else {
        return Err(PackageError::InvalidPackage);
    };
    if !matches!(quote, '"' | '\'') || !raw.ends_with(quote) {
        return Err(PackageError::InvalidPackage);
    }

    let value = &raw[quote.len_utf8()..raw.len() - quote.len_utf8()];
    if value.contains('\\') {
        return Err(PackageError::InvalidPackage);
    }
    Ok(value.to_owned())
}

fn find_descendant<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    if node.kind() == kind {
        return Some(node);
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| find_descendant(child, kind))
}

fn required_string_property(
    properties: &BTreeMap<String, QmlPropertyValue>,
    key: &str,
) -> Result<String, PackageError> {
    match properties.get(key) {
        Some(QmlPropertyValue::String(value)) => Ok(value.clone()),
        _ => Err(PackageError::InvalidPackage),
    }
}

fn required_bool_property(
    properties: &BTreeMap<String, QmlPropertyValue>,
    key: &str,
) -> Result<bool, PackageError> {
    match properties.get(key) {
        Some(QmlPropertyValue::Bool(value)) => Ok(*value),
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

    #[test]
    fn parses_qml_bool_properties_with_semicolon_and_inline_comment() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false; // same shape VESC Tool accepts\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();

        let package = parse_package_manifest(&manifest).expect("package manifest");

        assert!(!package.qml_is_fullscreen());
    }

    #[test]
    fn parses_qml_descriptor_with_single_quoted_strings() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: 'Refloat package'\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();

        let package = parse_package_manifest(&manifest).expect("package manifest");

        assert_eq!(package.name(), "Refloat package");
        assert!(package.qml_is_fullscreen());
    }

    #[test]
    fn rejects_qml_descriptor_strings_that_need_escape_evaluation() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: 'Refloat\\'s package'\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();

        assert!(parse_package_manifest(&manifest).is_err());
    }

    #[test]
    fn ignores_nested_qml_properties_when_reading_package_metadata() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let manifest = harness.loopback_staging_dir().join("pkgdesc.qml");
        std::fs::write(
            &manifest,
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Root package\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"root.vescpkg\"\n    Item {\n        property string pkgName: \"Nested package\"\n        property string pkgOutput: \"nested.vescpkg\"\n    }\n}\n",
        )
        .unwrap();

        let package = parse_package_manifest(&manifest).expect("package manifest");

        assert_eq!(package.name(), "Root package");
        assert_eq!(package.output(), "root.vescpkg");
    }
}
