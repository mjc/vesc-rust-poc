use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::package_wire::{PackageField, WireError, parse_vescpkg};

/// Rendering mode for an embedded QML App UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QmlAppUiMode {
    Embedded,
    Fullscreen,
}

impl QmlAppUiMode {
    const fn from_package_value(value: u8) -> Self {
        if value == 0 {
            Self::Embedded
        } else {
            Self::Fullscreen
        }
    }

    pub(crate) const fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen)
    }
}

/// Embedded QML App UI source and rendering mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QmlAppUi {
    pub(crate) source: String,
    pub(crate) mode: QmlAppUiMode,
}

/// A decoded VESC package ready for installation.
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
    /// Embedded QML App UI, if the package provides one.
    pub qml_app_ui: Option<QmlAppUi>,
}

#[derive(Debug)]
struct PackageDecoder {
    package: Package,
    qml_source: Option<String>,
    qml_mode: QmlAppUiMode,
}

impl PackageDecoder {
    #[must_use]
    fn new() -> Self {
        Self {
            package: Package {
                name: String::new(),
                description: String::new(),
                description_md: String::new(),
                lisp_data: Vec::new(),
                qml_app_ui: None,
            },
            qml_source: None,
            qml_mode: QmlAppUiMode::Embedded,
        }
    }

    fn apply(mut self, field: PackageField) -> Result<Self, PackageError> {
        let PackageField { key, value } = field;

        match key.as_str() {
            "name" => self.package.name = decode_text(value)?,
            "description" => self.package.description = decode_text(value)?,
            "description_md" => self.package.description_md = decode_text(value)?,
            "lispData" => self.package.lisp_data = value,
            "qmlFile" => self.qml_source = Some(decode_text(value)?),
            "qmlIsFullscreen" => {
                self.qml_mode =
                    QmlAppUiMode::from_package_value(value.first().copied().unwrap_or_default());
            }
            _ => {}
        }

        Ok(self)
    }

    fn into_package(self) -> Result<Package, PackageError> {
        let Self {
            package,
            qml_source,
            qml_mode,
        } = self;
        let package = Package {
            qml_app_ui: qml_source
                .filter(|source| !source.is_empty())
                .map(|source| QmlAppUi {
                    source,
                    mode: qml_mode,
                }),
            ..package
        };

        package.validate_for_install()?;
        Ok(package)
    }
}

#[derive(Debug)]
pub enum PackageError {
    Io(io::Error),
    Wire(WireError),
    InvalidPackage(&'static str),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Wire(error) => error.fmt(f),
            Self::InvalidPackage(reason) => write!(f, "invalid VESC package: {reason}"),
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
        parse_vescpkg(data)?
            .into_iter()
            .try_fold(PackageDecoder::new(), PackageDecoder::apply)?
            .into_package()
    }

    pub(crate) fn validate_for_install(&self) -> Result<(), PackageError> {
        (!self.lisp_data.is_empty() || self.qml_app_ui.is_some())
            .then_some(())
            .ok_or(PackageError::InvalidPackage(
                "package has no Lisp or QML payload",
            ))
    }
}

fn decode_text(bytes: Vec<u8>) -> Result<String, PackageError> {
    String::from_utf8(bytes)
        .map_err(|_| PackageError::InvalidPackage("text field is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_wire::PackageField;
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;

    #[test]
    fn decoder_keeps_qml_mode_when_seen_before_source() {
        let package = [
            PackageField {
                key: "name".to_owned(),
                value: b"Float Out Boy".to_vec(),
            },
            PackageField {
                key: "qmlIsFullscreen".to_owned(),
                value: vec![1],
            },
            PackageField {
                key: "qmlFile".to_owned(),
                value: b"Item {}".to_vec(),
            },
        ]
        .into_iter()
        .try_fold(PackageDecoder::new(), PackageDecoder::apply)
        .and_then(PackageDecoder::into_package)
        .expect("valid package");

        assert_eq!(
            package.qml_app_ui,
            Some(QmlAppUi {
                source: "Item {}".to_owned(),
                mode: QmlAppUiMode::Fullscreen,
            })
        );
    }

    #[test]
    fn rejects_a_package_with_no_meaningful_fields() {
        let raw = b"VESC Packet\0";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw).expect("compress package");
        let mut bytes = (raw.len() as u32).to_be_bytes().to_vec();
        bytes.extend(encoder.finish().expect("finish package"));

        assert!(matches!(
            Package::from_bytes(&bytes),
            Err(PackageError::InvalidPackage(
                "package has no Lisp or QML payload"
            ))
        ));
    }

    #[test]
    fn rejects_metadata_without_an_installable_payload() {
        let package = Package {
            name: "metadata only".to_owned(),
            description: "no code".to_owned(),
            description_md: String::new(),
            lisp_data: Vec::new(),
            qml_app_ui: None,
        };

        assert!(matches!(
            package.validate_for_install(),
            Err(PackageError::InvalidPackage(
                "package has no Lisp or QML payload"
            ))
        ));
    }
}
