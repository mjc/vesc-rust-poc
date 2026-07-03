use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::manifest::{
    manifest_path, parse_package_manifest, parse_pkgdesc, staging_dir_from_manifest,
};
use crate::package_build::PackageBuildPlan;
use crate::package_format::{
    VescPackageInput, VescPackageWire, build_vesc_package, encode_vesc_package,
};
use crate::package_runner::{RealPackageRunner, package_provenance_from_env};
use crate::package_target::{PackageTargetMode, PackageTargetPlan};
use crate::package_wire::{WireError, parse_vescpkg};

/// Errors returned when reading, decoding, or building a package.
#[derive(Debug)]
pub enum PackageError {
    /// Reading package bytes from disk failed.
    Io(io::Error),
    /// Decoding package wire data failed.
    Wire(WireError),
    /// The decoded package failed structural validation.
    InvalidPackage,
    /// Building package assets from source inputs failed.
    Build(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Wire(error) => write!(f, "{error}"),
            Self::InvalidPackage => f.write_str("invalid VESC package"),
            Self::Build(reason) => write!(f, "package build failed: {reason}"),
        }
    }
}

impl std::error::Error for PackageError {}

impl From<io::Error> for PackageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<WireError> for PackageError {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

impl From<crate::package_target::PackageTargetError> for PackageError {
    fn from(error: crate::package_target::PackageTargetError) -> Self {
        Self::Build(format!("{error:?}"))
    }
}

/// Parsed VESC package metadata and payload data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name shown to the user.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Markdown description used by newer tools.
    pub description_md: String,
    /// Embedded Lisp payload bytes.
    pub lisp_data: Vec<u8>,
    /// QML entrypoint filename.
    pub qml_file: String,
    /// Raw `pkgDescQml` metadata from the package.
    pub pkg_desc_qml: String,
    /// Whether the package should launch fullscreen.
    pub qml_is_fullscreen: bool,
}

impl Package {
    /// Read and decode a package from disk.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let bytes = fs::read(normalize_package_path(path.as_ref()))?;
        Self::from_bytes(&bytes)
    }

    /// Build an in-memory package from a `pkgdesc.qml` and its referenced files.
    pub fn from_manifest(manifest: impl AsRef<Path>) -> Result<Self, PackageError> {
        let manifest = manifest_path(manifest.as_ref());
        let staging_dir = staging_dir_from_manifest(&manifest)?;
        let staging_root = StagingRoot::new(&staging_dir)?;
        let descriptor = parse_package_manifest(&manifest)?;
        let description_md_path = staging_root
            .asset_file("pkgDescriptionMd", descriptor.description_md())?
            .ok_or_else(|| {
                PackageError::Build("pkgDescriptionMd must name a staging file".to_owned())
            })?;
        let lisp_path = staging_root
            .asset_file("pkgLisp", descriptor.lisp())?
            .ok_or_else(|| PackageError::Build("pkgLisp must name a staging file".to_owned()))?;
        let qml_path = staging_root.asset_file("pkgQml", descriptor.qml())?;
        let description_md = fs::read_to_string(description_md_path)?;
        let lisp_source = fs::read_to_string(&lisp_path)?;
        let qml_file = qml_path
            .map(fs::read_to_string)
            .transpose()?
            .unwrap_or_default();
        let pkg_desc_qml = fs::read_to_string(&manifest)?;
        let bytes = build_vesc_package(&VescPackageInput {
            name: descriptor.name(),
            description_md: &description_md,
            lisp_source: &lisp_source,
            lisp_editor_path: &staging_dir,
            lisp_import_path: lisp_path.parent(),
            qml_file: &qml_file,
            pkg_desc_qml: &pkg_desc_qml,
            qml_is_fullscreen: descriptor.qml_is_fullscreen(),
        })?;
        Self::from_bytes(&bytes)
    }

    /// Build and write a package from a `pkgdesc.qml` and its referenced files.
    pub fn write_from_manifest(manifest: impl AsRef<Path>) -> Result<PathBuf, PackageError> {
        let manifest = manifest_path(manifest.as_ref());
        let staging_dir = staging_dir_from_manifest(&manifest)?;
        let staging_root = StagingRoot::new(&staging_dir)?;
        let descriptor = parse_package_manifest(&manifest)?;
        let output = staging_root.output_file(descriptor.output())?;
        Self::from_manifest(&manifest)?.write(&output)?;
        Ok(output)
    }

    /// Encode the package and write it to disk.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, PackageError> {
        let bytes = self.to_bytes()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &bytes)?;
        Ok(bytes)
    }

    /// Decode a package from raw wire bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PackageError> {
        let fields = parse_vescpkg(data)?;
        let mut package = Self {
            name: String::new(),
            description: String::new(),
            description_md: String::new(),
            lisp_data: Vec::new(),
            qml_file: String::new(),
            pkg_desc_qml: String::new(),
            qml_is_fullscreen: false,
        };

        for field in fields {
            match field.key.as_str() {
                "name" => package.name = decode_utf8(field.value)?,
                "description" => package.description = decode_utf8(field.value)?,
                "description_md" => package.description_md = decode_utf8(field.value)?,
                "lispData" => package.lisp_data = field.value,
                "qmlFile" => package.qml_file = decode_utf8(field.value)?,
                "pkgDescQml" => package.pkg_desc_qml = decode_utf8(field.value)?,
                "qmlIsFullscreen" => {
                    package.qml_is_fullscreen = field.value.first().copied().unwrap_or(0) != 0;
                }
                _ => {}
            }
        }

        if package.is_valid() {
            Ok(package)
        } else {
            Err(PackageError::InvalidPackage)
        }
    }

    /// Encode the package into raw wire bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, PackageError> {
        if !self.is_valid() {
            return Err(PackageError::InvalidPackage);
        }

        encode_vesc_package(&VescPackageWire {
            name: &self.name,
            description: &self.description,
            description_md: &self.description_md,
            lisp_data: &self.lisp_data,
            qml_file: &self.qml_file,
            pkg_desc_qml: &self.pkg_desc_qml,
            qml_is_fullscreen: self.qml_is_fullscreen,
        })
        .map_err(PackageError::from)
    }

    /// Return whether the package has at least one populated field.
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            || !self.description.is_empty()
            || !self.description_md.is_empty()
            || !self.lisp_data.is_empty()
            || !self.qml_file.is_empty()
            || !self.pkg_desc_qml.is_empty()
    }
}

struct StagingRoot {
    path: PathBuf,
    canonical_path: PathBuf,
}

impl StagingRoot {
    fn new(path: &Path) -> Result<Self, PackageError> {
        Ok(Self {
            path: path.to_path_buf(),
            canonical_path: path.canonicalize()?,
        })
    }

    fn output_file(&self, output: &str) -> Result<PathBuf, PackageError> {
        let relative = staging_relative_path("pkgOutput", output, RequiredPath::Required)?;
        let Some(relative) = relative else {
            return Err(PackageError::Build(
                "pkgOutput must name a package file inside the staging directory".to_owned(),
            ));
        };
        let output = self.path.join(relative);
        self.reject_symlink_path(&output)?;
        Ok(output)
    }

    fn asset_file(&self, field: &str, asset_path: &str) -> Result<Option<PathBuf>, PackageError> {
        let Some(relative) = staging_relative_path(field, asset_path, RequiredPath::Optional)?
        else {
            return Ok(None);
        };
        let asset = self.path.join(relative);
        self.reject_symlink_path(&asset)?;
        let canonical_asset = asset.canonicalize()?;
        if canonical_asset.starts_with(&self.canonical_path) {
            Ok(Some(asset))
        } else {
            Err(PackageError::Build(format!(
                "{field} must stay inside the staging directory: {}",
                asset.display()
            )))
        }
    }

    fn reject_symlink_path(&self, path: &Path) -> Result<(), PackageError> {
        let relative = path.strip_prefix(&self.path).map_err(|_| {
            PackageError::Build(format!(
                "path must stay inside the staging directory: {}",
                path.display()
            ))
        })?;
        let mut current = self.path.clone();
        for component in relative.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            current.push(name);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(PackageError::Build(format!(
                        "staging paths must not traverse symlinks: {}",
                        current.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(PackageError::Io(error)),
            }
        }
        Ok(())
    }
}

enum RequiredPath {
    Required,
    Optional,
}

fn staging_relative_path(
    field: &str,
    path: &str,
    required: RequiredPath,
) -> Result<Option<PathBuf>, PackageError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return match required {
            RequiredPath::Required => Err(PackageError::Build(format!(
                "{field} must name a package file inside the staging directory"
            ))),
            RequiredPath::Optional => Ok(None),
        };
    }

    if path
        .components()
        .all(|component| component == Component::CurDir)
    {
        return match required {
            RequiredPath::Required => Err(PackageError::Build(format!(
                "{field} must name a package file inside the staging directory"
            ))),
            RequiredPath::Optional => Err(PackageError::Build(format!(
                "{field} must name a staging file"
            ))),
        };
    }

    if path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        Ok(Some(path.to_path_buf()))
    } else {
        Err(PackageError::Build(format!(
            "{field} must be relative to the staging directory: {}",
            path.display()
        )))
    }
}

/// Host-side builder that locates a package manifest and produces output paths.
pub struct Builder {
    source_root: PathBuf,
    plan: PackageBuildPlan,
    mode: PackageTargetMode,
}

impl Builder {
    /// Construct a builder from a package manifest path.
    pub fn from_manifest(manifest: impl AsRef<Path>) -> Result<Self, PackageError> {
        let manifest = manifest_path(manifest.as_ref());
        let staging_dir = staging_dir_from_manifest(&manifest)?;
        let (package_name, version) = parse_pkgdesc(&manifest)?;
        let source_root = find_workspace_root(&staging_dir)?;

        Ok(Self {
            source_root: source_root.clone(),
            plan: PackageBuildPlan::with_provenance(
                &source_root,
                package_name,
                version,
                package_provenance_from_env(),
            ),
            mode: PackageTargetMode::PackageOnly,
        })
    }

    /// Select the package build mode.
    pub fn with_mode(mut self, mode: PackageTargetMode) -> Self {
        self.mode = mode;
        self
    }

    /// Return the underlying build plan.
    pub fn build_plan(&self) -> &PackageBuildPlan {
        &self.plan
    }

    /// Build the package using the default conversion runner.
    pub fn build(&self) -> Result<PathBuf, PackageError> {
        self.build_with(&RealPackageRunner)
    }

    /// Build the package using the supplied conversion runner.
    pub fn build_with<R: crate::package_conversion::PackageBinaryConversionRunner>(
        &self,
        runner: &R,
    ) -> Result<PathBuf, PackageError> {
        let target = PackageTargetPlan::with_provenance(
            &self.source_root,
            self.plan.layout().package_name(),
            self.plan.layout().version(),
            package_provenance_from_env(),
            self.mode,
        );
        target.execute_with(runner).map_err(Into::into)
    }
}

fn decode_utf8(bytes: Vec<u8>) -> Result<String, PackageError> {
    String::from_utf8(bytes).map_err(|_| PackageError::InvalidPackage)
}

fn normalize_package_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    for prefix in ["file://", "file:/"] {
        if let Some(rest) = path_str.strip_prefix(prefix) {
            if rest.starts_with('/') {
                return PathBuf::from(rest);
            }
            return PathBuf::from(format!("/{rest}"));
        }
    }
    path.to_path_buf()
}

fn find_workspace_root(start: &Path) -> Result<PathBuf, PackageError> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("Cargo.toml").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(PackageError::Build(
                "could not locate workspace root from manifest path".to_owned(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Builder, Package};
    use crate::test_support::PackageTestHarness;
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn sample_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        write_string(&mut data, "VESC Packet");
        write_field(&mut data, "name", b"Rust BLE loopback test package");
        write_field(
            &mut data,
            "lispData",
            b"(load-native-lib \"src/package_lib.bin\")\n",
        );
        q_compress(&data)
    }

    fn write_string(buf: &mut Vec<u8>, value: &str) {
        buf.extend_from_slice(value.as_bytes());
        buf.push(0);
    }

    fn write_field(buf: &mut Vec<u8>, name: &str, data: &[u8]) {
        write_string(buf, name);
        buf.extend_from_slice(&(data.len() as i32).to_be_bytes());
        buf.extend_from_slice(data);
    }

    fn q_compress(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut output = Vec::with_capacity(4 + compressed.len());
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(&compressed);
        output
    }

    #[test]
    fn package_round_trips_bytes() {
        let original = Package::from_bytes(&sample_bytes()).expect("decode");
        let round_trip =
            Package::from_bytes(&original.to_bytes().expect("encode")).expect("decode");
        assert_eq!(original.name, round_trip.name);
        assert_eq!(original.lisp_data, round_trip.lisp_data);
    }

    #[test]
    fn builder_from_manifest_reads_staged_pkgdesc() {
        let harness = PackageTestHarness::new()
            .ensure_loopback_staging()
            .write_text("Cargo.toml", "[package]\nname = \"test\"\n");
        std::fs::write(
            harness.loopback_staging_dir().join("pkgdesc.qml"),
            "import QtQuick 2.15\nItem {\n    property string pkgName: \"Rust BLE loopback test package\"\n    property string pkgOutput: \"Rust-BLE-loopback-test-package-0.1.0.vescpkg\"\n}\n",
        )
        .unwrap();

        let builder = Builder::from_manifest(harness.loopback_staging_dir()).expect("builder");
        assert_eq!(
            builder.build_plan().layout().package_name(),
            "Rust BLE loopback test package"
        );
        assert_eq!(builder.build_plan().layout().version(), "0.1.0");
    }

    fn write_refloat_style_staging(harness: &PackageTestHarness) {
        let staging = harness.loopback_staging_dir();
        std::fs::create_dir_all(staging.join("lisp")).unwrap();
        std::fs::create_dir_all(staging.join("src")).unwrap();
        std::fs::write(staging.join("package_README-gen.md"), "Refloat readme").unwrap();
        std::fs::write(
            staging.join("ui.qml"),
            "Item { property string marker: \"refloat\" }\n",
        )
        .unwrap();
        std::fs::write(
            staging.join("lisp/package.lisp"),
            "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n",
        )
        .unwrap();
        std::fs::write(staging.join("src/package_lib.bin"), b"refloat-native").unwrap();
        std::fs::write(
            staging.join("pkgdesc.qml"),
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();
    }

    #[test]
    fn package_from_manifest_uses_descriptor_referenced_assets() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();

        let package = Package::from_manifest(staging.join("pkgdesc.qml")).expect("package");
        assert_eq!(package.name, "Refloat");
        assert_eq!(package.description_md, "Refloat readme");
        assert_eq!(
            package.qml_file,
            "Item { property string marker: \"refloat\" }\n"
        );
        assert_eq!(
            package.pkg_desc_qml,
            std::fs::read_to_string(staging.join("pkgdesc.qml")).unwrap()
        );
        assert!(package.qml_is_fullscreen);
        let (_, imports) =
            crate::package_wire::parse_lisp_imports(&package.lisp_data).expect("lisp imports");
        let [import] = imports.as_slice() else {
            panic!("expected one Lisp import, got {imports:?}");
        };
        assert_eq!(import.payload, b"refloat-native\0");
    }

    #[test]
    fn write_from_manifest_uses_descriptor_output_path() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();

        let output = Package::write_from_manifest(staging.join("pkgdesc.qml")).expect("package");
        assert_eq!(output, staging.join("refloat.vescpkg"));
        let package = Package::read(&output).expect("written package");
        assert_eq!(package.name, "Refloat");
        assert_eq!(package.description_md, "Refloat readme");
    }

    #[test]
    fn write_from_manifest_rejects_output_paths_outside_staging() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();
        std::fs::write(
            staging.join("pkgdesc.qml"),
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"../escaped.vescpkg\"\n}\n",
        )
        .unwrap();

        let error =
            Package::write_from_manifest(staging.join("pkgdesc.qml")).expect_err("bad output");

        assert!(error.to_string().contains("pkgOutput must be relative"));
        assert!(!staging.join("../escaped.vescpkg").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_from_manifest_rejects_symlink_output_paths() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();
        let outside_output = harness.root().join("escaped.vescpkg");
        symlink(&outside_output, staging.join("refloat.vescpkg")).unwrap();

        let error =
            Package::write_from_manifest(staging.join("pkgdesc.qml")).expect_err("bad output");

        assert!(
            error.to_string().contains("must not traverse symlinks"),
            "expected symlink error, got {error}"
        );
        assert!(!outside_output.exists());
    }

    #[test]
    fn from_manifest_rejects_asset_paths_outside_staging() {
        for (field, value) in [
            ("pkgDescriptionMd", "../README.md"),
            ("pkgLisp", "/tmp/package.lisp"),
            ("pkgQml", "../ui.qml"),
        ] {
            let harness = PackageTestHarness::new().ensure_loopback_staging();
            write_refloat_style_staging(&harness);
            let staging = harness.loopback_staging_dir();
            std::fs::write(
                staging.join("pkgdesc.qml"),
                format!(
                    "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"{}\"\n    property string pkgLisp: \"{}\"\n    property string pkgQml: \"{}\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"refloat.vescpkg\"\n}}\n",
                    if field == "pkgDescriptionMd" { value } else { "package_README-gen.md" },
                    if field == "pkgLisp" { value } else { "lisp/package.lisp" },
                    if field == "pkgQml" { value } else { "ui.qml" },
                ),
            )
            .unwrap();

            let error = Package::from_manifest(staging.join("pkgdesc.qml")).expect_err("bad path");

            assert!(
                error.to_string().contains(field),
                "expected {field} error, got {error}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn from_manifest_rejects_symlink_asset_paths() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();
        let outside_readme = harness.root().join("outside.md");
        std::fs::write(&outside_readme, "escaped readme").unwrap();
        std::fs::remove_file(staging.join("package_README-gen.md")).unwrap();
        symlink(&outside_readme, staging.join("package_README-gen.md")).unwrap();

        let error = Package::from_manifest(staging.join("pkgdesc.qml")).expect_err("bad asset");

        assert!(
            error.to_string().contains("must not traverse symlinks"),
            "expected symlink error, got {error}"
        );
    }

    #[test]
    fn write_from_manifest_rejects_empty_output_file_names() {
        for output in ["", ".", "./"] {
            let harness = PackageTestHarness::new().ensure_loopback_staging();
            write_refloat_style_staging(&harness);
            let staging = harness.loopback_staging_dir();
            std::fs::write(
                staging.join("pkgdesc.qml"),
                format!(
                    "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"{output}\"\n}}\n"
                ),
            )
            .unwrap();

            let error =
                Package::write_from_manifest(staging.join("pkgdesc.qml")).expect_err("bad output");

            assert!(
                error
                    .to_string()
                    .contains("pkgOutput must name a package file")
            );
        }
    }

    #[test]
    fn package_from_manifest_resolves_lisp_sibling_imports() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        write_refloat_style_staging(&harness);
        let staging = harness.loopback_staging_dir();
        std::fs::write(
            staging.join("lisp/package.lisp"),
            "(import \"src/package_lib.bin\" 'package-lib)\n(import \"bms.lisp\" 'bms)\n",
        )
        .unwrap();
        std::fs::write(staging.join("lisp/bms.lisp"), "(define bms-enabled true)\n").unwrap();

        let package = Package::from_manifest(staging.join("pkgdesc.qml")).expect("package");

        let (_, imports) =
            crate::package_wire::parse_lisp_imports(&package.lisp_data).expect("lisp imports");
        let payloads = imports
            .iter()
            .map(|import| import.payload.as_slice())
            .collect::<Vec<_>>();
        assert_eq!(
            payloads,
            vec![
                b"refloat-native\0".as_slice(),
                b"(define bms-enabled true)\n\0".as_slice()
            ]
        );
    }
}
