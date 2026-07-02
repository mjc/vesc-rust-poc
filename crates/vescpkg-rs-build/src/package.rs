use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
        let descriptor = parse_package_manifest(&manifest)?;
        let description_md = fs::read_to_string(staging_dir.join(descriptor.description_md()))?;
        let lisp_source = fs::read_to_string(staging_dir.join(descriptor.lisp()))?;
        let qml_file = if descriptor.qml().is_empty() {
            String::new()
        } else {
            fs::read_to_string(staging_dir.join(descriptor.qml()))?
        };
        let pkg_desc_qml = fs::read_to_string(&manifest)?;
        let bytes = build_vesc_package(&VescPackageInput {
            name: descriptor.name(),
            description_md: &description_md,
            lisp_source: &lisp_source,
            lisp_editor_path: &staging_dir,
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
        let descriptor = parse_package_manifest(&manifest)?;
        let output = staging_dir.join(descriptor.output());
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
}
