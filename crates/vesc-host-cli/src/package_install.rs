use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use flate2::read::ZlibDecoder;
use flate2::{write::ZlibEncoder, Compression};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VescPackage {
    pub name: String,
    pub description: String,
    pub description_md: String,
    pub lisp_data: Vec<u8>,
    pub qml_file: String,
    pub pkg_desc_qml: String,
    pub qml_is_fullscreen: bool,
}

impl VescPackage {
    pub fn load_ok(&self) -> bool {
        !self.name.is_empty()
            || !self.description.is_empty()
            || !self.description_md.is_empty()
            || !self.lisp_data.is_empty()
            || !self.qml_file.is_empty()
            || !self.pkg_desc_qml.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallStep {
    EraseQml { bytes: usize },
    UploadQml { bytes: usize, fullscreen: bool },
    EraseLisp { bytes: usize },
    UploadLisp { bytes: usize },
    SetRunning { running: bool },
    ReloadFirmware,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInstallReport {
    pub package_name: String,
    pub steps: Vec<PackageInstallStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallError {
    Io(String),
    Device(String),
    InvalidPackage,
}

impl fmt::Display for PackageInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(reason) => write!(f, "io error: {reason}"),
            Self::Device(reason) => write!(f, "device error: {reason}"),
            Self::InvalidPackage => f.write_str("invalid VESC package"),
        }
    }
}

impl std::error::Error for PackageInstallError {}

pub trait PackageInstallTransport {
    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError>;
    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError>;
    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError>;
    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError>;
    fn set_running(&self, running: bool) -> Result<(), PackageInstallError>;
    fn reload_firmware(&self) -> Result<(), PackageInstallError>;
}

#[derive(Debug, Default)]
pub struct FakePackageInstallTransport {
    pub steps: std::cell::RefCell<Vec<PackageInstallStep>>,
}

impl PackageInstallTransport for FakePackageInstallTransport {
    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::EraseQml { bytes });
        Ok(())
    }

    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError> {
        self.steps.borrow_mut().push(PackageInstallStep::UploadQml {
            bytes: qml.len(),
            fullscreen,
        });
        Ok(())
    }

    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::EraseLisp { bytes });
        Ok(())
    }

    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::UploadLisp { bytes: lisp.len() });
        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::SetRunning { running });
        Ok(())
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::ReloadFirmware);
        Ok(())
    }
}

pub fn read_package_from_path(path: impl AsRef<Path>) -> Result<VescPackage, PackageInstallError> {
    let data = fs::read(path).map_err(|error| PackageInstallError::Io(error.to_string()))?;
    decode_package(&data)
}

pub fn decode_package(data: &[u8]) -> Result<VescPackage, PackageInstallError> {
    let mut decoder = ZlibDecoder::new(data);
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;

    let mut cursor = &bytes[..];
    if read_string(&mut cursor)? != "VESC Packet" {
        return Err(PackageInstallError::InvalidPackage);
    }

    let mut package = VescPackage {
        name: String::new(),
        description: String::new(),
        description_md: String::new(),
        lisp_data: Vec::new(),
        qml_file: String::new(),
        pkg_desc_qml: String::new(),
        qml_is_fullscreen: false,
    };

    while !cursor.is_empty() {
        let field = read_string(&mut cursor)?;
        let len = read_u32(&mut cursor)? as usize;
        let field_bytes = take(&mut cursor, len)?;

        match field.as_str() {
            "name" => package.name = String::from_utf8(field_bytes).map_err(invalid_utf8)?,
            "description" => {
                package.description = String::from_utf8(field_bytes).map_err(invalid_utf8)?
            }
            "description_md" => {
                package.description_md = String::from_utf8(field_bytes).map_err(invalid_utf8)?
            }
            "lispData" => package.lisp_data = field_bytes,
            "qmlFile" => package.qml_file = String::from_utf8(field_bytes).map_err(invalid_utf8)?,
            "pkgDescQml" => {
                package.pkg_desc_qml = String::from_utf8(field_bytes).map_err(invalid_utf8)?
            }
            "qmlIsFullscreen" => {
                package.qml_is_fullscreen = field_bytes.first().copied().unwrap_or(0) != 0;
            }
            _ => {}
        }
    }

    if package.load_ok() {
        Ok(package)
    } else {
        Err(PackageInstallError::InvalidPackage)
    }
}

pub fn install_package<T: PackageInstallTransport>(
    package: &VescPackage,
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    if !package.load_ok() {
        return Err(PackageInstallError::InvalidPackage);
    }

    let mut steps = Vec::new();

    if !package.qml_file.is_empty() {
        let qml = qml_compress(&package.qml_file)?;
        let bytes = qml.len();
        transport.erase_qml(bytes + 100)?;
        steps.push(PackageInstallStep::EraseQml { bytes: bytes + 100 });
        transport.upload_qml(&qml, package.qml_is_fullscreen)?;
        steps.push(PackageInstallStep::UploadQml {
            bytes,
            fullscreen: package.qml_is_fullscreen,
        });
    } else {
        transport.erase_qml(16)?;
        steps.push(PackageInstallStep::EraseQml { bytes: 16 });
    }

    if !package.lisp_data.is_empty() {
        let bytes = package.lisp_data.len();
        transport.erase_lisp(bytes + 100)?;
        steps.push(PackageInstallStep::EraseLisp { bytes: bytes + 100 });
        transport.upload_lisp(&package.lisp_data)?;
        steps.push(PackageInstallStep::UploadLisp { bytes });
        transport.set_running(true)?;
        steps.push(PackageInstallStep::SetRunning { running: true });
    } else {
        transport.erase_lisp(16)?;
        steps.push(PackageInstallStep::EraseLisp { bytes: 16 });
    }

    transport.reload_firmware()?;
    steps.push(PackageInstallStep::ReloadFirmware);

    Ok(PackageInstallReport {
        package_name: package.name.clone(),
        steps,
    })
}

fn read_string(cursor: &mut &[u8]) -> Result<String, PackageInstallError> {
    let len = read_u32(cursor)? as usize;
    let bytes = take(cursor, len)?;
    String::from_utf8(bytes).map_err(invalid_utf8)
}

fn read_u32(cursor: &mut &[u8]) -> Result<u32, PackageInstallError> {
    let bytes = take(cursor, 4)?;
    Ok(u32::from_le_bytes(bytes.try_into().expect("slice length")))
}

fn take(cursor: &mut &[u8], len: usize) -> Result<Vec<u8>, PackageInstallError> {
    if cursor.len() < len {
        return Err(PackageInstallError::InvalidPackage);
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head.to_vec())
}

fn invalid_utf8(_: std::string::FromUtf8Error) -> PackageInstallError {
    PackageInstallError::InvalidPackage
}

fn qml_compress(script: &str) -> Result<Vec<u8>, PackageInstallError> {
    let raw = format!("import Vedder.vesc.vescinterface 1.0;import \"qrc:/mobile\";{script}")
        .into_bytes();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&raw)
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;
    let compressed = encoder
        .finish()
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;

    let mut out = Vec::with_capacity(4 + compressed.len());
    out.extend_from_slice(&(raw.len() as u32).to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_package, install_package, qml_compress, FakePackageInstallTransport,
        PackageInstallStep,
    };
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn build_package_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        write_string(&mut data, "VESC Packet");
        write_field(&mut data, "name", b"Rust BLE loopback test package");
        write_field(&mut data, "qmlFile", b"import QtQuick 2.15\nItem {}\n");
        write_field(
            &mut data,
            "lispData",
            b"(load-native-lib \"src/package_lib.bin\")\n",
        );
        write_field(&mut data, "qmlIsFullscreen", &[1]);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&data).unwrap();
        encoder.finish().unwrap()
    }

    fn write_string(buf: &mut Vec<u8>, value: &str) {
        buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buf.extend_from_slice(value.as_bytes());
    }

    fn write_field(buf: &mut Vec<u8>, name: &str, data: &[u8]) {
        write_string(buf, name);
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(data);
    }

    #[test]
    fn decodes_a_compressed_vesc_package() {
        let package = decode_package(&build_package_bytes()).expect("package");
        assert_eq!(package.name, "Rust BLE loopback test package");
        assert!(package.qml_is_fullscreen);
        assert!(package.load_ok());
    }

    #[test]
    fn installs_package_in_vesc_tool_order() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        let qml = qml_compress("import QtQuick 2.15\nItem {}\n").expect("qml");

        let report = install_package(&package, &transport).expect("report");

        assert_eq!(
            report.steps,
            vec![
                PackageInstallStep::EraseQml {
                    bytes: qml.len() + 100
                },
                PackageInstallStep::UploadQml {
                    bytes: qml.len(),
                    fullscreen: true
                },
                PackageInstallStep::EraseLisp {
                    bytes: package.lisp_data.len() + 100
                },
                PackageInstallStep::UploadLisp {
                    bytes: package.lisp_data.len()
                },
                PackageInstallStep::SetRunning { running: true },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }
}
