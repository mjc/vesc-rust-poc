use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::package_format::{VescPackageWire, encode_vesc_package};
use crate::package_wire::{WireError, parse_vescpkg};

/// A decoded VESC package that can be installed or written back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package display name.
    pub name: String,
    /// Rendered HTML description.
    pub description: String,
    /// Original Markdown description.
    pub description_md: String,
    /// Packed Lisp source and imported native payloads.
    pub lisp_data: Vec<u8>,
    /// Embedded QML source.
    pub qml_file: String,
    /// Package descriptor QML.
    pub pkg_desc_qml: String,
    /// Whether the QML app runs fullscreen.
    pub qml_is_fullscreen: bool,
}

#[derive(Debug)]
pub enum PackageError {
    Io(io::Error),
    Wire(WireError),
    InvalidPackage,
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Wire(error) => error.fmt(f),
            Self::InvalidPackage => f.write_str("invalid VESC package"),
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

impl Package {
    /// Read and decode a package file.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        Self::from_bytes(&fs::read(path)?)
    }

    /// Decode package bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PackageError> {
        let mut package = Self {
            name: String::new(),
            description: String::new(),
            description_md: String::new(),
            lisp_data: Vec::new(),
            qml_file: String::new(),
            pkg_desc_qml: String::new(),
            qml_is_fullscreen: false,
        };

        for field in parse_vescpkg(data)? {
            match field.key.as_str() {
                "name" => package.name = decode_text(field.value)?,
                "description" => package.description = decode_text(field.value)?,
                "description_md" => package.description_md = decode_text(field.value)?,
                "lispData" => package.lisp_data = field.value,
                "qmlFile" => package.qml_file = decode_text(field.value)?,
                "pkgDescQml" => package.pkg_desc_qml = decode_text(field.value)?,
                "qmlIsFullscreen" => {
                    package.qml_is_fullscreen =
                        field.value.first().copied().unwrap_or_default() != 0;
                }
                _ => {}
            }
        }

        package
            .is_valid()
            .then_some(package)
            .ok_or(PackageError::InvalidPackage)
    }

    /// Encode this package into wire bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, PackageError> {
        self.is_valid()
            .then(|| {
                encode_vesc_package(&VescPackageWire {
                    name: &self.name,
                    description: &self.description,
                    description_md: &self.description_md,
                    lisp_data: &self.lisp_data,
                    qml_file: &self.qml_file,
                    pkg_desc_qml: &self.pkg_desc_qml,
                    qml_is_fullscreen: self.qml_is_fullscreen,
                })
            })
            .ok_or(PackageError::InvalidPackage)?
            .map_err(PackageError::Io)
    }

    /// Encode and write this package to a file.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), PackageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_bytes()?)?;
        Ok(())
    }

    /// Return whether the package contains at least one meaningful field.
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            || !self.description.is_empty()
            || !self.description_md.is_empty()
            || !self.lisp_data.is_empty()
            || !self.qml_file.is_empty()
            || !self.pkg_desc_qml.is_empty()
    }
}

fn decode_text(bytes: Vec<u8>) -> Result<String, PackageError> {
    String::from_utf8(bytes).map_err(|_| PackageError::InvalidPackage)
}
