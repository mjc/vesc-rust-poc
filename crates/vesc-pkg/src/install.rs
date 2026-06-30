use std::cell::Cell;
use std::fmt;
use std::io::Write;

use flate2::{Compression, write::ZlibEncoder};

use crate::package::Package;

const PACKAGE_ERASE_BYTES: usize = 16;

/// Steps emitted while installing or erasing a package over the firmware transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStep {
    /// Reserve flash space for the package's QML payload.
    EraseQml {
        /// Total QML erase size in bytes.
        bytes: usize,
    },
    /// Write the compressed QML payload and fullscreen flag.
    UploadQml {
        /// Compressed QML payload size in bytes.
        bytes: usize,
        /// Whether the uploaded QML app should run fullscreen.
        fullscreen: bool,
    },
    /// Reserve flash space for the package's Lisp payload.
    EraseLisp {
        /// Total Lisp erase size in bytes.
        bytes: usize,
    },
    /// Write the package's Lisp payload.
    UploadLisp {
        /// Lisp payload size in bytes.
        bytes: usize,
    },
    /// Toggle the installed Lisp package's running state.
    SetRunning {
        /// Desired running state after install.
        running: bool,
    },
    /// Ask the firmware to reload package state after the transfer completes.
    ReloadFirmware,
}

/// Summary of the package operations issued to the target.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    /// Package name reported by the install or erase flow.
    pub package_name: String,
    /// Ordered transport operations that were performed.
    pub steps: Vec<InstallStep>,
}

/// Errors produced while translating a package into firmware install steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// Host-side I/O failed while preparing package payloads.
    Io(String),
    /// The device rejected or failed a transport operation.
    Device(String),
    /// The package bytes failed structural validation before install.
    InvalidPackage,
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(reason) => write!(f, "io error: {reason}"),
            Self::Device(reason) => write!(f, "device error: {reason}"),
            Self::InvalidPackage => f.write_str("invalid VESC package"),
        }
    }
}

impl std::error::Error for InstallError {}

impl From<crate::package::PackageError> for InstallError {
    fn from(error: crate::package::PackageError) -> Self {
        match error {
            crate::package::PackageError::Io(error) => Self::Io(error.to_string()),
            crate::package::PackageError::InvalidPackage => Self::InvalidPackage,
            other => Self::Io(other.to_string()),
        }
    }
}

/// Firmware-side transport operations needed to install or erase a package.

pub trait InstallTransport {
    /// Returns whether a QML app is already present on the target.
    fn has_qml_app(&self) -> Result<bool, InstallError>;
    /// Erases enough space for a QML payload of `bytes` bytes.
    fn erase_qml(&self, bytes: usize) -> Result<(), InstallError>;
    /// Uploads a compressed QML payload and its fullscreen setting.
    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), InstallError>;
    /// Erases enough space for a Lisp payload of `bytes` bytes.
    fn erase_lisp(&self, bytes: usize) -> Result<(), InstallError>;
    /// Uploads the Lisp payload bytes.
    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), InstallError>;
    /// Enables or disables the installed Lisp package.
    fn set_running(&self, running: bool) -> Result<(), InstallError>;
    /// Requests a firmware reload after package changes complete.
    fn reload_firmware(&self) -> Result<(), InstallError>;
}

/// Small helper that drives package install and erase flows through an `InstallTransport`.

pub struct Installer<'a, T: InstallTransport> {
    transport: &'a T,
}

impl<'a, T: InstallTransport> Installer<'a, T> {
    /// Binds an installer to a transport implementation.

    pub fn new(transport: &'a T) -> Self {
        Self { transport }
    }

    /// Installs `package` through the configured transport.

    pub fn install(&self, package: &Package) -> Result<InstallReport, InstallError> {
        install_package(package, self.transport)
    }

    /// Erases the currently installed package through the configured transport.

    pub fn erase(&self) -> Result<InstallReport, InstallError> {
        erase_package(self.transport)
    }
}

/// In-memory transport used by tests to capture install sequencing.

#[derive(Debug, Default)]
pub struct FakeInstallTransport {
    has_qml_app: Cell<bool>,
    /// Recorded transport steps for assertions in tests and golden checks.
    pub steps: std::cell::RefCell<Vec<InstallStep>>,
}

impl FakeInstallTransport {
    /// Controls whether the fake transport reports an existing QML app.

    pub fn set_has_qml_app(&self, has_qml_app: bool) {
        self.has_qml_app.set(has_qml_app);
    }
}

impl InstallTransport for FakeInstallTransport {
    fn has_qml_app(&self) -> Result<bool, InstallError> {
        Ok(self.has_qml_app.get())
    }

    fn erase_qml(&self, bytes: usize) -> Result<(), InstallError> {
        self.steps
            .borrow_mut()
            .push(InstallStep::EraseQml { bytes });
        Ok(())
    }

    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), InstallError> {
        self.steps.borrow_mut().push(InstallStep::UploadQml {
            bytes: qml.len(),
            fullscreen,
        });
        Ok(())
    }

    fn erase_lisp(&self, bytes: usize) -> Result<(), InstallError> {
        self.steps
            .borrow_mut()
            .push(InstallStep::EraseLisp { bytes });
        Ok(())
    }

    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), InstallError> {
        self.steps
            .borrow_mut()
            .push(InstallStep::UploadLisp { bytes: lisp.len() });
        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), InstallError> {
        self.steps
            .borrow_mut()
            .push(InstallStep::SetRunning { running });
        Ok(())
    }

    fn reload_firmware(&self) -> Result<(), InstallError> {
        self.steps.borrow_mut().push(InstallStep::ReloadFirmware);
        Ok(())
    }
}

/// Installs a validated package using the same operation order as VESC Tool.

pub fn install_package<T: InstallTransport>(
    package: &Package,
    transport: &T,
) -> Result<InstallReport, InstallError> {
    if !package.is_valid() {
        return Err(InstallError::InvalidPackage);
    }

    let mut steps = Vec::new();

    if !package.qml_file.is_empty() {
        let qml = qml_compress(&package.qml_file)?;
        let bytes = qml.len();
        transport
            .erase_qml(bytes + 100)
            .map_err(|error| step_error(format!("erase QML {} bytes", bytes + 100), error))?;
        steps.push(InstallStep::EraseQml { bytes: bytes + 100 });
        transport
            .upload_qml(&qml, package.qml_is_fullscreen)
            .map_err(|error| step_error(format!("upload QML {bytes} bytes"), error))?;
        steps.push(InstallStep::UploadQml {
            bytes,
            fullscreen: package.qml_is_fullscreen,
        });
    } else if transport.has_qml_app()? {
        transport
            .erase_qml(PACKAGE_ERASE_BYTES)
            .map_err(|error| step_error("erase QML 16 bytes", error))?;
        steps.push(InstallStep::EraseQml {
            bytes: PACKAGE_ERASE_BYTES,
        });
    }

    if !package.lisp_data.is_empty() {
        let bytes = package.lisp_data.len();
        transport
            .erase_lisp(bytes + 100)
            .map_err(|error| step_error(format!("erase Lisp {} bytes", bytes + 100), error))?;
        steps.push(InstallStep::EraseLisp { bytes: bytes + 100 });
        transport
            .upload_lisp(&package.lisp_data)
            .map_err(|error| step_error(format!("upload Lisp {bytes} bytes"), error))?;
        steps.push(InstallStep::UploadLisp { bytes });
        transport
            .set_running(true)
            .map_err(|error| step_error("set Lisp running true", error))?;
        steps.push(InstallStep::SetRunning { running: true });
    } else {
        transport
            .erase_lisp(PACKAGE_ERASE_BYTES)
            .map_err(|error| step_error("erase Lisp 16 bytes", error))?;
        steps.push(InstallStep::EraseLisp {
            bytes: PACKAGE_ERASE_BYTES,
        });
    }

    transport
        .reload_firmware()
        .map_err(|error| step_error("reload firmware", error))?;
    steps.push(InstallStep::ReloadFirmware);

    Ok(InstallReport {
        package_name: package.name.clone(),
        steps,
    })
}

/// Erases any installed package payloads from the target and reloads firmware state.

pub fn erase_package<T: InstallTransport>(transport: &T) -> Result<InstallReport, InstallError> {
    let mut steps = Vec::new();

    transport
        .erase_qml(PACKAGE_ERASE_BYTES)
        .map_err(|error| step_error("erase QML 16 bytes", error))?;
    steps.push(InstallStep::EraseQml {
        bytes: PACKAGE_ERASE_BYTES,
    });

    transport
        .erase_lisp(PACKAGE_ERASE_BYTES)
        .map_err(|error| step_error("erase Lisp 16 bytes", error))?;
    steps.push(InstallStep::EraseLisp {
        bytes: PACKAGE_ERASE_BYTES,
    });

    transport
        .reload_firmware()
        .map_err(|error| step_error("reload firmware", error))?;
    steps.push(InstallStep::ReloadFirmware);

    Ok(InstallReport {
        package_name: "installed package".to_owned(),
        steps,
    })
}

fn step_error(step: impl AsRef<str>, error: InstallError) -> InstallError {
    match error {
        InstallError::Device(reason) => {
            InstallError::Device(format!("{}: {reason}", step.as_ref()))
        }
        InstallError::Io(reason) => InstallError::Io(format!("{}: {reason}", step.as_ref())),
        InstallError::InvalidPackage => InstallError::InvalidPackage,
    }
}

fn qml_compress(script: &str) -> Result<Vec<u8>, InstallError> {
    let raw = format!("import \"qrc:/mobile\";import Vedder.vesc.vescinterface 1.0;{script}")
        .into_bytes();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&raw)
        .map_err(|error| InstallError::Io(error.to_string()))?;
    let compressed = encoder
        .finish()
        .map_err(|error| InstallError::Io(error.to_string()))?;

    let mut out = Vec::with_capacity(4 + compressed.len());
    out.extend_from_slice(&(raw.len() as u32).to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        FakeInstallTransport, InstallError, InstallStep, Installer, erase_package, install_package,
        qml_compress,
    };
    use crate::package::Package;
    use flate2::{Compression, write::ZlibEncoder};
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
    fn installer_runs_install_and_erase() {
        let package = Package::from_bytes(&build_package_bytes()).expect("package");
        let transport = FakeInstallTransport::default();
        let installer = Installer::new(&transport);

        installer.install(&package).expect("install");
        installer.erase().expect("erase");
        assert_eq!(transport.steps.borrow().len(), 9);
    }

    #[test]
    fn installs_package_in_vesc_tool_order() {
        let package = Package::from_bytes(&build_package_bytes()).expect("package");
        let transport = FakeInstallTransport::default();
        let qml = qml_compress("import QtQuick 2.15\nItem {}\n").expect("qml");

        let report = install_package(&package, &transport).expect("report");

        assert_eq!(
            report.steps,
            vec![
                InstallStep::EraseQml {
                    bytes: qml.len() + 100
                },
                InstallStep::UploadQml {
                    bytes: qml.len(),
                    fullscreen: true
                },
                InstallStep::EraseLisp {
                    bytes: package.lisp_data.len() + 100
                },
                InstallStep::UploadLisp {
                    bytes: package.lisp_data.len()
                },
                InstallStep::SetRunning { running: true },
                InstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn erases_package_in_vesc_tool_order() {
        let transport = FakeInstallTransport::default();
        let report = erase_package(&transport).expect("report");
        assert_eq!(report.package_name, "installed package");
        assert_eq!(transport.steps.borrow().len(), 3);
    }

    #[test]
    fn install_step_errors_keep_the_failed_step_name() {
        let error = super::step_error(
            "erase Lisp 407 bytes",
            InstallError::Device("timed out waiting for a BLE reply".to_owned()),
        );
        assert!(error.to_string().contains("erase Lisp 407 bytes"));
    }
}
