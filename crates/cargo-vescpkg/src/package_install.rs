//! Host-side VESC package install protocol.
//!
//! Package construction and `.vescpkg` decoding live in `vescpkg_rs_build`.
//! Device install is a CLI concern because it owns transport, firmware state,
//! and operator-facing recovery behavior.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::Write;
use std::path::Path;

use flate2::{Compression, write::ZlibEncoder};

use crate::loopback::LoopbackTarget;
use crate::package_transport::BtlePackageInstallTransport;

const PACKAGE_ERASE_BYTES: usize = 16;

pub use vescpkg_rs_build::Package as VescPackage;

/// Steps emitted while installing or erasing a package over the firmware transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallStep {
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
pub struct PackageInstallReport {
    /// Package name reported by the install or erase flow.
    pub package_name: String,
    /// Ordered transport operations that were performed.
    pub steps: Vec<PackageInstallStep>,
}

/// Errors produced while translating a package into firmware install steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallError {
    /// Host-side I/O failed while preparing package payloads.
    Io(String),
    /// The device rejected or failed a transport operation.
    Device(String),
    /// The package bytes failed structural validation before install.
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

impl From<vescpkg_rs_build::PackageError> for PackageInstallError {
    fn from(error: vescpkg_rs_build::PackageError) -> Self {
        match error {
            vescpkg_rs_build::PackageError::Io(error) => Self::Io(error.to_string()),
            vescpkg_rs_build::PackageError::InvalidPackage => Self::InvalidPackage,
            other => Self::Io(other.to_string()),
        }
    }
}

/// Firmware-side transport operations needed to install or erase a package.
pub trait PackageInstallTransport {
    /// Returns whether a QML app is already present on the target.
    fn has_qml_app(&self) -> Result<bool, PackageInstallError>;
    /// Erases enough space for a QML payload of `bytes` bytes.
    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError>;
    /// Uploads a compressed QML payload and its fullscreen setting.
    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError>;
    /// Erases enough space for a Lisp payload of `bytes` bytes.
    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError>;
    /// Uploads the Lisp payload bytes.
    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError>;
    /// Enables or disables the installed Lisp package.
    fn set_running(&self, running: bool) -> Result<(), PackageInstallError>;
    /// Requests a firmware reload after package changes complete.
    fn reload_firmware(&self) -> Result<(), PackageInstallError>;
}

/// In-memory transport used by tests to capture install sequencing.
#[derive(Debug, Default)]
pub struct FakePackageInstallTransport {
    has_qml_app: Cell<bool>,
    reject_erase_lisp: Cell<bool>,
    reject_set_running_true: Cell<bool>,
    fail_set_running_true_io: Cell<bool>,
    /// Recorded transport steps for assertions in tests and golden checks.
    pub steps: RefCell<Vec<PackageInstallStep>>,
}

impl FakePackageInstallTransport {
    /// Controls whether the fake transport reports an existing QML app.
    pub fn set_has_qml_app(&self, has_qml_app: bool) {
        self.has_qml_app.set(has_qml_app);
    }

    /// Controls whether erasing Lisp reports device rejection.
    pub fn reject_erase_lisp(&self) {
        self.reject_erase_lisp.set(true);
    }

    /// Controls whether starting Lisp reports device rejection.
    pub fn reject_set_running_true(&self) {
        self.reject_set_running_true.set(true);
    }

    /// Controls whether starting Lisp reports a host transport failure.
    pub fn fail_set_running_true_io(&self) {
        self.fail_set_running_true_io.set(true);
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
        if self.reject_erase_lisp.get() {
            return Err(PackageInstallError::Device(
                "device rejected the package erase".to_owned(),
            ));
        }
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
        if running && self.reject_set_running_true.get() {
            return Err(PackageInstallError::Device(
                "device rejected the package write".to_owned(),
            ));
        }
        if running && self.fail_set_running_true_io.get() {
            return Err(PackageInstallError::Io(
                "failed to write set-running command".to_owned(),
            ));
        }
        Ok(())
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        self.steps
            .borrow_mut()
            .push(PackageInstallStep::ReloadFirmware);
        Ok(())
    }
}

enum InstallOperation<'a> {
    EraseQml { bytes: usize },
    UploadQml { qml: Vec<u8>, fullscreen: bool },
    EraseLisp { bytes: usize },
    UploadLisp { lisp: &'a [u8] },
    SetRunning { running: bool },
    ReloadFirmware,
}

impl InstallOperation<'_> {
    fn step(&self) -> PackageInstallStep {
        match self {
            Self::EraseQml { bytes } => PackageInstallStep::EraseQml { bytes: *bytes },
            Self::UploadQml { qml, fullscreen } => PackageInstallStep::UploadQml {
                bytes: qml.len(),
                fullscreen: *fullscreen,
            },
            Self::EraseLisp { bytes } => PackageInstallStep::EraseLisp { bytes: *bytes },
            Self::UploadLisp { lisp } => PackageInstallStep::UploadLisp { bytes: lisp.len() },
            Self::SetRunning { running } => PackageInstallStep::SetRunning { running: *running },
            Self::ReloadFirmware => PackageInstallStep::ReloadFirmware,
        }
    }

    fn label(&self) -> Cow<'_, str> {
        match self {
            Self::EraseQml { bytes } => Cow::Owned(format!("erase QML {bytes} bytes")),
            Self::UploadQml { qml, .. } => Cow::Owned(format!("upload QML {} bytes", qml.len())),
            Self::EraseLisp { bytes } => Cow::Owned(format!("erase Lisp {bytes} bytes")),
            Self::UploadLisp { lisp } => Cow::Owned(format!("upload Lisp {} bytes", lisp.len())),
            Self::SetRunning { running } => Cow::Owned(format!("set Lisp running {running}")),
            Self::ReloadFirmware => Cow::Borrowed("reload firmware"),
        }
    }

    fn run<T: PackageInstallTransport>(&self, transport: &T) -> Result<(), PackageInstallError> {
        match self {
            Self::EraseQml { bytes } => transport.erase_qml(*bytes),
            Self::UploadQml { qml, fullscreen } => transport.upload_qml(qml, *fullscreen),
            Self::EraseLisp { bytes } => transport.erase_lisp(*bytes),
            Self::UploadLisp { lisp } => transport.upload_lisp(lisp),
            Self::SetRunning { running } => transport.set_running(*running),
            Self::ReloadFirmware => transport.reload_firmware(),
        }
        .map_err(|error| step_error(self.label(), error))
    }

    fn run_for_install<T: PackageInstallTransport>(
        &self,
        transport: &T,
    ) -> Result<(), PackageInstallError> {
        match self.run(transport) {
            Err(PackageInstallError::Device(_))
                if matches!(self, Self::SetRunning { running: true }) =>
            {
                Ok(())
            }
            result => result,
        }
    }
}

enum PackageQml<'a> {
    Upload { script: &'a str, fullscreen: bool },
    EraseExisting,
    LeaveEmpty,
}

impl<'a> PackageQml<'a> {
    fn from_package<T: PackageInstallTransport>(
        package: &'a VescPackage,
        transport: &T,
    ) -> Result<Self, PackageInstallError> {
        match package.qml_file.as_str() {
            "" if transport.has_qml_app()? => Ok(Self::EraseExisting),
            "" => Ok(Self::LeaveEmpty),
            script => Ok(Self::Upload {
                script,
                fullscreen: package.qml_is_fullscreen,
            }),
        }
    }

    fn into_operations(
        self,
    ) -> Result<impl Iterator<Item = InstallOperation<'a>>, PackageInstallError> {
        let operations = match self {
            Self::Upload { script, fullscreen } => {
                let qml = qml_compress(script)?;
                [
                    Some(InstallOperation::EraseQml {
                        bytes: qml.len() + 100,
                    }),
                    Some(InstallOperation::UploadQml { qml, fullscreen }),
                ]
            }
            Self::EraseExisting => [
                Some(InstallOperation::EraseQml {
                    bytes: PACKAGE_ERASE_BYTES,
                }),
                None,
            ],
            Self::LeaveEmpty => [None, None],
        };

        Ok(operations.into_iter().flatten())
    }
}

enum PackageLisp<'a> {
    Upload(&'a [u8]),
    EraseEmpty,
}

impl<'a> PackageLisp<'a> {
    fn from_package(package: &'a VescPackage) -> Self {
        match package.lisp_data.as_slice() {
            [] => Self::EraseEmpty,
            lisp => Self::Upload(lisp),
        }
    }

    fn into_operations(self) -> impl Iterator<Item = InstallOperation<'a>> {
        let operations = match self {
            Self::Upload(lisp) => [
                Some(InstallOperation::EraseLisp {
                    bytes: lisp.len() + 100,
                }),
                Some(InstallOperation::UploadLisp { lisp }),
                Some(InstallOperation::SetRunning { running: true }),
            ],
            Self::EraseEmpty => [
                Some(InstallOperation::EraseLisp {
                    bytes: PACKAGE_ERASE_BYTES,
                }),
                None,
                None,
            ],
        };

        operations.into_iter().flatten()
    }
}

/// Reads and decodes a package from a filesystem path.
pub fn read_package_from_path(path: impl AsRef<Path>) -> Result<VescPackage, PackageInstallError> {
    VescPackage::read(path).map_err(Into::into)
}

/// Decodes raw package bytes into an installable VESC package model.
pub fn decode_package(data: &[u8]) -> Result<VescPackage, PackageInstallError> {
    VescPackage::from_bytes(data).map_err(Into::into)
}

/// Opens BLE, installs a package from disk, and closes the transport.
pub fn install_over_ble(
    package_path: impl AsRef<Path>,
    target: LoopbackTarget,
) -> Result<PackageInstallReport, PackageInstallError> {
    let package = read_package_from_path(package_path)?;
    with_open_transport(target, OpenMode::Preflight, |transport| {
        install_package(&package, transport)
    })
}

/// Opens BLE, erases the installed package, and closes the transport.
pub fn erase_over_ble(
    target: LoopbackTarget,
    no_preflight: bool,
) -> Result<PackageInstallReport, PackageInstallError> {
    let open_mode = match no_preflight {
        true => OpenMode::NoPreflightRecovery,
        false => OpenMode::Preflight,
    };

    with_open_transport(target, open_mode, |transport| {
        if no_preflight {
            eprintln!("package erase recovery: best-effort stop before erase");
            if let Err(error) = transport.stop_running_recovery() {
                eprintln!("package erase recovery: stop did not ack: {error}");
            }
        }

        erase_package(transport)
    })
}

/// Installs a validated package using the same operation order as VESC Tool.
pub fn install_package<T: PackageInstallTransport>(
    package: &VescPackage,
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    let package = checked_package(package)?;

    execute_install_plan(transport, install_operations(package, transport)?).map(|steps| {
        PackageInstallReport {
            package_name: package.name.clone(),
            steps,
        }
    })
}

/// Erases any installed package payloads from the target and reloads firmware state.
pub fn erase_package<T: PackageInstallTransport>(
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    // Source: ~/projects/vesc_tool/codeloader.cpp:1072-1090
    // uninstallVescPackage() erases Lisp first, then QML, then reloads firmware
    // and returns resLisp && resQml.
    let mut steps = Vec::new();
    let mut first_error = None;
    for op in [
        InstallOperation::EraseLisp {
            bytes: PACKAGE_ERASE_BYTES,
        },
        InstallOperation::EraseQml {
            bytes: PACKAGE_ERASE_BYTES,
        },
    ] {
        let step = op.step();
        if let Err(error) = op.run(transport) {
            first_error.get_or_insert(error);
        } else {
            steps.push(step);
        }
    }

    let reload = InstallOperation::ReloadFirmware;
    let reload_step = reload.step();
    if let Err(error) = reload.run(transport) {
        first_error.get_or_insert(error);
    } else {
        steps.push(reload_step);
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    Ok(PackageInstallReport {
        package_name: "installed package".to_owned(),
        steps,
    })
}

fn checked_package(package: &VescPackage) -> Result<&VescPackage, PackageInstallError> {
    package
        .is_valid()
        .then_some(package)
        .ok_or(PackageInstallError::InvalidPackage)
}

fn install_operations<'a, T: PackageInstallTransport>(
    package: &'a VescPackage,
    transport: &T,
) -> Result<impl Iterator<Item = InstallOperation<'a>>, PackageInstallError> {
    Ok(PackageQml::from_package(package, transport)?
        .into_operations()?
        .chain(PackageLisp::from_package(package).into_operations())
        .chain([InstallOperation::ReloadFirmware]))
}

fn execute_install_plan<'a, T, I>(
    transport: &T,
    operations: I,
) -> Result<Vec<PackageInstallStep>, PackageInstallError>
where
    T: PackageInstallTransport,
    I: IntoIterator<Item = InstallOperation<'a>>,
{
    // Source: ~/projects/vesc_tool/codeloader.cpp:1007-1024 installVescPackage()
    // gates later QML/Lisp steps on res, but always sleeps and reloads firmware
    // before returning res.
    let mut steps = Vec::new();
    let mut first_error = None;
    let mut reload = None;

    for op in operations {
        if matches!(op, InstallOperation::ReloadFirmware) {
            reload = Some(op);
            continue;
        }

        if first_error.is_some() {
            continue;
        }

        let step = op.step();
        if let Err(error) = op.run_for_install(transport) {
            first_error.get_or_insert(error);
        } else {
            steps.push(step);
        }
    }

    if let Some(reload) = reload {
        let step = reload.step();
        if let Err(error) = reload.run_for_install(transport) {
            first_error.get_or_insert(error);
        } else {
            steps.push(step);
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    Ok(steps)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenMode {
    Preflight,
    NoPreflightRecovery,
}

fn with_open_transport<R>(
    target: LoopbackTarget,
    open_mode: OpenMode,
    run: impl FnOnce(&BtlePackageInstallTransport) -> Result<R, PackageInstallError>,
) -> Result<R, PackageInstallError> {
    let transport = BtlePackageInstallTransport::new()?;
    match open_mode {
        OpenMode::Preflight => transport.open(target)?,
        OpenMode::NoPreflightRecovery => transport.open_without_preflight(target)?,
    }

    let result = run(&transport);
    transport.close();
    result
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

    Ok((raw.len() as u32)
        .to_be_bytes()
        .into_iter()
        .chain(compressed)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        FakePackageInstallTransport, PackageInstallError, PackageInstallStep, decode_package,
        erase_package, install_package,
    };
    use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
    use std::io::{Read, Write};

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
        encoder.write_all(data).expect("write compressed package");
        let compressed = encoder.finish().expect("finish compressed package");

        (data.len() as u32)
            .to_be_bytes()
            .into_iter()
            .chain(compressed)
            .collect()
    }

    #[test]
    fn decodes_a_compressed_vesc_package() {
        let package = decode_package(&build_package_bytes()).expect("package");
        assert_eq!(package.name, "Rust BLE loopback test package");
        assert!(package.qml_is_fullscreen);
        assert!(package.is_valid());
    }

    #[test]
    fn installer_runs_install_and_erase() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();

        install_package(&package, &transport).expect("install");
        erase_package(&transport).expect("erase");
        assert_eq!(transport.steps.borrow().len(), 9);
    }

    #[test]
    fn installs_package_in_vesc_tool_order() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        let qml = super::qml_compress("import QtQuick 2.15\nItem {}\n").expect("qml");

        let report = install_package(&package, &transport).expect("report");

        // Source: ~/projects/vesc_tool/codeloader.cpp:994-1024
        // installVescPackage() runs QML erase/upload, Lisp erase/upload,
        // lispSetRunning(1), then sleep/reload.
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
    fn qml_compress_prepends_vesc_tool_mobile_imports() {
        let qml = super::qml_compress("import QtQuick 2.15\nItem {}\n").expect("qml");
        let raw_len = u32::from_be_bytes(qml[0..4].try_into().expect("qCompress length"));
        let mut decoder = ZlibDecoder::new(&qml[4..]);
        let mut raw = String::new();
        decoder
            .read_to_string(&mut raw)
            .expect("decompress generated qml");

        // Source: ~/projects/vesc_tool/codeloader.cpp:750-754
        // qmlCompress() prepends the same imports before qCompress(..., 9).
        assert_eq!(raw.len(), raw_len as usize);
        assert!(raw.starts_with(
            "import Vedder.vesc.vescinterface 1.0;import \"qrc:/mobile\";import QtQuick 2.15\n"
        ));
    }

    #[test]
    fn install_ignores_set_running_rejection_like_vesc_tool() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        transport.reject_set_running_true();

        let report = install_package(&package, &transport).expect("report");

        // Source: ~/projects/vesc_tool/codeloader.cpp:1014-1016 and
        // ~/projects/vesc_tool/commands.cpp:2234-2240. VESC Tool sends
        // lispSetRunning(1) and does not wait for lispRunningResRx.
        assert!(
            report
                .steps
                .contains(&PackageInstallStep::SetRunning { running: true })
        );
        assert_eq!(
            report.steps.last(),
            Some(&PackageInstallStep::ReloadFirmware)
        );
    }

    #[test]
    fn install_reports_set_running_host_io_failures() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        transport.fail_set_running_true_io();

        let error = install_package(&package, &transport).expect_err("install should fail");

        assert!(matches!(error, PackageInstallError::Io(_)));
        assert!(error.to_string().contains("set Lisp running true"));
        assert!(error.to_string().contains("failed to write set-running"));
    }

    #[test]
    fn install_reloads_after_lisp_erase_failure_like_vesc_tool() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        transport.reject_erase_lisp();
        let qml = super::qml_compress("import QtQuick 2.15\nItem {}\n").expect("qml");

        let error = install_package(&package, &transport).expect_err("install should fail");

        assert!(error.to_string().contains(&format!(
            "erase Lisp {} bytes",
            package.lisp_data.len() + 100
        )));
        // Source: ~/projects/vesc_tool/codeloader.cpp:1007-1024
        // installVescPackage() stops later Lisp work when res goes false, but
        // still sleeps and reloads firmware before returning res.
        assert_eq!(
            &*transport.steps.borrow(),
            &[
                PackageInstallStep::EraseQml {
                    bytes: qml.len() + 100,
                },
                PackageInstallStep::UploadQml {
                    bytes: qml.len(),
                    fullscreen: true,
                },
                PackageInstallStep::EraseLisp {
                    bytes: package.lisp_data.len() + 100,
                },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn erases_existing_qml_when_new_package_has_none() {
        let mut package = decode_package(&build_package_bytes()).expect("package");
        package.qml_file.clear();
        let transport = FakePackageInstallTransport::default();
        transport.set_has_qml_app(true);

        let report = install_package(&package, &transport).expect("report");

        // Source: ~/projects/vesc_tool/codeloader.cpp:1001-1004
        // installVescPackage() erases old QML only when no new QML exists and
        // hasQmlApp is true.
        assert_eq!(
            report.steps.first(),
            Some(&PackageInstallStep::EraseQml {
                bytes: super::PACKAGE_ERASE_BYTES,
            })
        );
    }

    #[test]
    fn erases_package_in_vesc_tool_order() {
        let transport = FakePackageInstallTransport::default();
        let report = erase_package(&transport).expect("report");
        assert_eq!(report.package_name, "installed package");
        // Source: ~/projects/vesc_tool/codeloader.cpp:1083-1089
        // uninstallVescPackage() erases Lisp, erases QML, reloads, then returns
        // resLisp && resQml.
        assert_eq!(
            &*transport.steps.borrow(),
            &[
                PackageInstallStep::EraseLisp {
                    bytes: super::PACKAGE_ERASE_BYTES,
                },
                PackageInstallStep::EraseQml {
                    bytes: super::PACKAGE_ERASE_BYTES,
                },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn erase_package_attempts_qml_and_reload_after_lisp_erase_failure() {
        let transport = FakePackageInstallTransport::default();
        transport.reject_erase_lisp();

        let error = erase_package(&transport).expect_err("erase should fail");

        assert!(error.to_string().contains("erase Lisp 16 bytes"));
        // Source: ~/projects/vesc_tool/codeloader.cpp:1083-1089
        // uninstallVescPackage() stores resLisp before qmlErase(16), so QML
        // erase and reload still run.
        assert_eq!(
            &*transport.steps.borrow(),
            &[
                PackageInstallStep::EraseLisp {
                    bytes: super::PACKAGE_ERASE_BYTES,
                },
                PackageInstallStep::EraseQml {
                    bytes: super::PACKAGE_ERASE_BYTES,
                },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn install_step_errors_keep_the_failed_step_name() {
        let error = super::step_error(
            "erase Lisp 407 bytes",
            PackageInstallError::Device("timed out waiting for a BLE reply".to_owned()),
        );
        assert!(error.to_string().contains("erase Lisp 407 bytes"));
    }
}
