use std::cell::Cell;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;

use flate2::{Compression, write::ZlibEncoder};

const PACKAGE_ERASE_BYTES: usize = 16;

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
    fn has_qml_app(&self) -> Result<bool, PackageInstallError>;
    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError>;
    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError>;
    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError>;
    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError>;
    fn set_running(&self, running: bool) -> Result<(), PackageInstallError>;
    fn reload_firmware(&self) -> Result<(), PackageInstallError>;
}

#[derive(Debug, Default)]
pub struct FakePackageInstallTransport {
    has_qml_app: Cell<bool>,
    pub steps: std::cell::RefCell<Vec<PackageInstallStep>>,
}

impl FakePackageInstallTransport {
    pub fn set_has_qml_app(&self, has_qml_app: bool) {
        self.has_qml_app.set(has_qml_app);
    }
}

impl PackageInstallTransport for FakePackageInstallTransport {
    fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
        Ok(self.has_qml_app.get())
    }

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
    let path = normalize_package_path(path.as_ref());
    let data = fs::read(path).map_err(|error| PackageInstallError::Io(error.to_string()))?;
    decode_package(&data)
}

pub fn decode_package(data: &[u8]) -> Result<VescPackage, PackageInstallError> {
    let fields = vesc_pkg_build::parse_vescpkg(data).map_err(map_wire_error)?;

    let mut package = VescPackage {
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
            "name" => package.name = String::from_utf8(field.value).map_err(invalid_utf8)?,
            "description" => {
                package.description = String::from_utf8(field.value).map_err(invalid_utf8)?
            }
            "description_md" => {
                package.description_md = String::from_utf8(field.value).map_err(invalid_utf8)?
            }
            "lispData" => package.lisp_data = field.value,
            "qmlFile" => package.qml_file = String::from_utf8(field.value).map_err(invalid_utf8)?,
            "pkgDescQml" => {
                package.pkg_desc_qml = String::from_utf8(field.value).map_err(invalid_utf8)?
            }
            "qmlIsFullscreen" => {
                package.qml_is_fullscreen = field.value.first().copied().unwrap_or(0) != 0;
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

fn map_wire_error(error: vesc_pkg_build::WireError) -> PackageInstallError {
    match error {
        vesc_pkg_build::WireError::DecompressionFailed(reason) => PackageInstallError::Io(reason),
        _ => PackageInstallError::InvalidPackage,
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
        transport
            .erase_qml(bytes + 100)
            .map_err(|error| step_error(format!("erase QML {} bytes", bytes + 100), error))?;
        steps.push(PackageInstallStep::EraseQml { bytes: bytes + 100 });
        transport
            .upload_qml(&qml, package.qml_is_fullscreen)
            .map_err(|error| step_error(format!("upload QML {bytes} bytes"), error))?;
        steps.push(PackageInstallStep::UploadQml {
            bytes,
            fullscreen: package.qml_is_fullscreen,
        });
    } else if transport.has_qml_app()? {
        transport
            .erase_qml(PACKAGE_ERASE_BYTES)
            .map_err(|error| step_error("erase QML 16 bytes", error))?;
        steps.push(PackageInstallStep::EraseQml {
            bytes: PACKAGE_ERASE_BYTES,
        });
    }

    if !package.lisp_data.is_empty() {
        let bytes = package.lisp_data.len();
        transport
            .erase_lisp(bytes + 100)
            .map_err(|error| step_error(format!("erase Lisp {} bytes", bytes + 100), error))?;
        steps.push(PackageInstallStep::EraseLisp { bytes: bytes + 100 });
        transport
            .upload_lisp(&package.lisp_data)
            .map_err(|error| step_error(format!("upload Lisp {bytes} bytes"), error))?;
        steps.push(PackageInstallStep::UploadLisp { bytes });
        transport
            .set_running(true)
            .map_err(|error| step_error("set Lisp running true", error))?;
        steps.push(PackageInstallStep::SetRunning { running: true });
    } else {
        transport
            .erase_lisp(PACKAGE_ERASE_BYTES)
            .map_err(|error| step_error("erase Lisp 16 bytes", error))?;
        steps.push(PackageInstallStep::EraseLisp {
            bytes: PACKAGE_ERASE_BYTES,
        });
    }

    transport
        .reload_firmware()
        .map_err(|error| step_error("reload firmware", error))?;
    steps.push(PackageInstallStep::ReloadFirmware);

    Ok(PackageInstallReport {
        package_name: package.name.clone(),
        steps,
    })
}

pub fn erase_package<T: PackageInstallTransport>(
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    let mut steps = Vec::new();

    transport
        .erase_qml(PACKAGE_ERASE_BYTES)
        .map_err(|error| step_error("erase QML 16 bytes", error))?;
    steps.push(PackageInstallStep::EraseQml {
        bytes: PACKAGE_ERASE_BYTES,
    });

    transport
        .erase_lisp(PACKAGE_ERASE_BYTES)
        .map_err(|error| step_error("erase Lisp 16 bytes", error))?;
    steps.push(PackageInstallStep::EraseLisp {
        bytes: PACKAGE_ERASE_BYTES,
    });

    transport
        .reload_firmware()
        .map_err(|error| step_error("reload firmware", error))?;
    steps.push(PackageInstallStep::ReloadFirmware);

    Ok(PackageInstallReport {
        package_name: "installed package".to_owned(),
        steps,
    })
}

fn step_error(step: impl AsRef<str>, error: PackageInstallError) -> PackageInstallError {
    match error {
        PackageInstallError::Device(reason) => {
            PackageInstallError::Device(format!("{}: {reason}", step.as_ref()))
        }
        PackageInstallError::Io(reason) => {
            PackageInstallError::Io(format!("{}: {reason}", step.as_ref()))
        }
        PackageInstallError::InvalidPackage => PackageInstallError::InvalidPackage,
    }
}

fn invalid_utf8(_: std::string::FromUtf8Error) -> PackageInstallError {
    PackageInstallError::InvalidPackage
}

fn normalize_package_path(path: &Path) -> std::path::PathBuf {
    let path_str = path.to_string_lossy();
    for prefix in ["file://", "file:/"] {
        if let Some(rest) = path_str.strip_prefix(prefix) {
            if rest.starts_with('/') {
                return std::path::PathBuf::from(rest);
            }

            return std::path::PathBuf::from(format!("/{rest}"));
        }
    }

    path.to_path_buf()
}

fn qml_compress(script: &str) -> Result<Vec<u8>, PackageInstallError> {
    let raw = format!("import \"qrc:/mobile\";import Vedder.vesc.vescinterface 1.0;{script}")
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
        FakePackageInstallTransport, PackageInstallError, PackageInstallStep,
        PackageInstallTransport, decode_package, erase_package, install_package, qml_compress,
        read_package_from_path, step_error,
    };
    use flate2::{Compression, write::ZlibEncoder};
    use std::cell::Cell;
    use std::io::Write;
    use std::path::Path;

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

        q_compress(&data)
    }

    fn build_lisp_only_package_bytes() -> Vec<u8> {
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
    fn decodes_vesc_packages() {
        let package = decode_package(&build_package_bytes()).expect("package");
        assert_eq!(package.name, "Rust BLE loopback test package");
        assert!(package.qml_is_fullscreen);
        assert!(package.load_ok());

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(b"VESC Packet\0").unwrap();
        let invalid = encoder.finish().unwrap();
        assert!(decode_package(&invalid).is_err());
    }

    #[test]
    fn decodes_refloat_vesc_tool_fixture_when_present() {
        let path = Path::new("/home/mjc/projects/refloat/refloat.vescpkg");
        if !path.exists() {
            return;
        }

        let package = read_package_from_path(path).expect("refloat package");

        assert_eq!(package.name, "Refloat");
        assert!(!package.qml_file.is_empty());
        assert!(!package.lisp_data.is_empty());
        assert!(
            package.lisp_data.len() < 128 * 1024,
            "fixture should stay below the VESC Lisp data limit"
        );
    }

    #[test]
    fn strips_file_uri_prefixes_from_package_paths() {
        let path = Path::new("file:/home/mjc/projects/refloat/refloat.vescpkg");
        if !Path::new("/home/mjc/projects/refloat/refloat.vescpkg").exists() {
            return;
        }

        let package = read_package_from_path(path).expect("package");
        assert_eq!(package.name, "Refloat");
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

    #[test]
    fn skips_qml_erasure_when_the_device_has_no_qml_app() {
        let package = decode_package(&build_lisp_only_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        transport.set_has_qml_app(false);

        let report = install_package(&package, &transport).expect("report");

        assert_eq!(
            report.steps,
            vec![
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

    #[test]
    fn install_step_errors_keep_the_failed_step_name() {
        let error = step_error(
            "erase Lisp 407 bytes",
            PackageInstallError::Device("timed out waiting for a BLE reply".to_owned()),
        );

        assert_eq!(
            error.to_string(),
            "device error: erase Lisp 407 bytes: timed out waiting for a BLE reply"
        );
    }

    #[test]
    fn erases_package_in_vesc_tool_order() {
        let transport = FakePackageInstallTransport::default();

        let report = erase_package(&transport).expect("report");

        assert_eq!(report.package_name, "installed package");
        assert_eq!(
            report.steps,
            vec![
                PackageInstallStep::EraseQml { bytes: 16 },
                PackageInstallStep::EraseLisp { bytes: 16 },
                PackageInstallStep::ReloadFirmware,
            ]
        );
        assert_eq!(transport.steps.borrow().len(), 3);
    }

    #[derive(Debug, Default)]
    struct ScriptedPackageInstallTransport {
        recorded: std::cell::RefCell<Vec<PackageInstallStep>>,
        fail_on: std::cell::Cell<Option<TransportFailPoint>>,
        queried_qml_app: Cell<bool>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TransportFailPoint {
        EraseQml,
        EraseLisp,
        ReloadFirmware,
    }

    impl ScriptedPackageInstallTransport {
        fn fail_on(point: TransportFailPoint) -> Self {
            Self {
                fail_on: std::cell::Cell::new(Some(point)),
                ..Self::default()
            }
        }
    }

    impl PackageInstallTransport for ScriptedPackageInstallTransport {
        fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
            self.queried_qml_app.set(true);
            Ok(true)
        }

        fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::EraseQml { bytes });
            if self.fail_on.get() == Some(TransportFailPoint::EraseQml) {
                return Err(PackageInstallError::Device("qml erase failed".to_owned()));
            }
            Ok(())
        }

        fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::UploadQml {
                    bytes: qml.len(),
                    fullscreen,
                });
            Ok(())
        }

        fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::EraseLisp { bytes });
            if self.fail_on.get() == Some(TransportFailPoint::EraseLisp) {
                return Err(PackageInstallError::Device("lisp erase failed".to_owned()));
            }
            Ok(())
        }

        fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::UploadLisp { bytes: lisp.len() });
            Ok(())
        }

        fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::SetRunning { running });
            Ok(())
        }

        fn reload_firmware(&self) -> Result<(), PackageInstallError> {
            self.recorded
                .borrow_mut()
                .push(PackageInstallStep::ReloadFirmware);
            if self.fail_on.get() == Some(TransportFailPoint::ReloadFirmware) {
                return Err(PackageInstallError::Device("reload failed".to_owned()));
            }
            Ok(())
        }
    }

    #[test]
    fn erase_package_does_not_query_qml_presence() {
        let transport = ScriptedPackageInstallTransport::default();

        erase_package(&transport).expect("report");

        assert!(!transport.queried_qml_app.get());
    }

    #[test]
    fn erase_package_aborts_before_lisp_when_qml_erase_fails() {
        let transport = ScriptedPackageInstallTransport::fail_on(TransportFailPoint::EraseQml);

        let error = erase_package(&transport).expect_err("expected qml erase failure");

        assert_eq!(
            error.to_string(),
            "device error: erase QML 16 bytes: qml erase failed"
        );
        assert_eq!(
            transport.recorded.borrow().as_slice(),
            &[PackageInstallStep::EraseQml { bytes: 16 }]
        );
    }

    #[test]
    fn erase_package_aborts_before_reload_when_lisp_erase_fails() {
        let transport = ScriptedPackageInstallTransport::fail_on(TransportFailPoint::EraseLisp);

        let error = erase_package(&transport).expect_err("expected lisp erase failure");

        assert_eq!(
            error.to_string(),
            "device error: erase Lisp 16 bytes: lisp erase failed"
        );
        assert_eq!(
            transport.recorded.borrow().as_slice(),
            &[
                PackageInstallStep::EraseQml { bytes: 16 },
                PackageInstallStep::EraseLisp { bytes: 16 },
            ]
        );
    }

    #[test]
    fn erase_package_reports_reload_failures() {
        let transport =
            ScriptedPackageInstallTransport::fail_on(TransportFailPoint::ReloadFirmware);

        let error = erase_package(&transport).expect_err("expected reload failure");

        assert_eq!(
            error.to_string(),
            "device error: reload firmware: reload failed"
        );
        assert_eq!(
            transport.recorded.borrow().as_slice(),
            &[
                PackageInstallStep::EraseQml { bytes: 16 },
                PackageInstallStep::EraseLisp { bytes: 16 },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn erase_package_uses_vesc_tool_fixed_erase_sizes() {
        let transport = FakePackageInstallTransport::default();

        erase_package(&transport).expect("report");

        assert_eq!(
            transport.steps.borrow().as_slice(),
            &[
                PackageInstallStep::EraseQml { bytes: 16 },
                PackageInstallStep::EraseLisp { bytes: 16 },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }
}
