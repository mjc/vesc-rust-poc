//! Host-side VESC package install protocol.
//!
//! Package construction and `.vescpkg` decoding live in this crate.
//! Device install is a CLI concern because it owns transport, firmware state,
//! and operator-facing recovery behavior.

use std::fmt;
use std::io::Write;
use std::path::Path;

use flate2::{Compression, write::ZlibEncoder};

use crate::loopback::LoopbackTarget;
use crate::package_transport::BtlePackageInstallTransport;

const PACKAGE_ERASE_BYTES: usize = 16;

pub use crate::package::Package as VescPackage;

/// Steps emitted while installing or erasing a package over the firmware transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallStep {
    /// Reserve flash space for the package's QML App UI payload.
    EraseQml {
        /// Total QML erase size in bytes.
        bytes: usize,
    },
    /// Write the compressed QML App UI payload.
    UploadQml {
        /// Compressed QML payload size in bytes.
        bytes: usize,
        /// Whether the uploaded QML App UI should run fullscreen.
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

impl PackageInstallStep {
    fn label(&self) -> String {
        match self {
            Self::EraseQml { bytes } => format!("erase QML {bytes} bytes"),
            Self::UploadQml { bytes, .. } => format!("upload QML {bytes} bytes"),
            Self::EraseLisp { bytes } => format!("erase Lisp {bytes} bytes"),
            Self::UploadLisp { bytes } => format!("upload Lisp {bytes} bytes"),
            Self::SetRunning { running } => format!("set Lisp running {running}"),
            Self::ReloadFirmware => "reload firmware".to_owned(),
        }
    }
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
    InvalidPackage(&'static str),
}

impl fmt::Display for PackageInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(reason) => write!(f, "io error: {reason}"),
            Self::Device(reason) => write!(f, "device error: {reason}"),
            Self::InvalidPackage(reason) => write!(f, "invalid VESC package: {reason}"),
        }
    }
}

impl std::error::Error for PackageInstallError {}

impl From<crate::package::PackageError> for PackageInstallError {
    fn from(error: crate::package::PackageError) -> Self {
        match error {
            crate::package::PackageError::Io(error) => Self::Io(error.to_string()),
            crate::package::PackageError::InvalidPackage(reason) => Self::InvalidPackage(reason),
            other => Self::Io(other.to_string()),
        }
    }
}

/// Firmware-side transport operations needed to install or erase a package.
pub trait PackageInstallTransport {
    /// Returns whether a QML App UI is already present on the target.
    fn has_qml_app(&self) -> Result<bool, PackageInstallError>;
    /// Erases enough space for a QML App UI payload of `bytes` bytes.
    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError>;
    /// Uploads a compressed QML App UI payload and its fullscreen setting.
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

#[derive(Debug, Clone, Copy)]
enum PackageInstallAction<'a> {
    EraseQml { bytes: usize },
    UploadQml { qml: &'a [u8], fullscreen: bool },
    EraseLisp { bytes: usize },
    UploadLisp { lisp: &'a [u8] },
    SetLispRunning,
}

impl PackageInstallAction<'_> {
    #[must_use]
    fn step(self) -> PackageInstallStep {
        match self {
            Self::EraseQml { bytes } => PackageInstallStep::EraseQml { bytes },
            Self::UploadQml { qml, fullscreen } => PackageInstallStep::UploadQml {
                bytes: qml.len(),
                fullscreen,
            },
            Self::EraseLisp { bytes } => PackageInstallStep::EraseLisp { bytes },
            Self::UploadLisp { lisp } => PackageInstallStep::UploadLisp { bytes: lisp.len() },
            Self::SetLispRunning => PackageInstallStep::SetRunning { running: true },
        }
    }

    fn execute<T: PackageInstallTransport>(self, transport: &T) -> Result<(), PackageInstallError> {
        match self {
            Self::EraseQml { bytes } => transport.erase_qml(bytes),
            Self::UploadQml { qml, fullscreen } => transport.upload_qml(qml, fullscreen),
            Self::EraseLisp { bytes } => transport.erase_lisp(bytes),
            Self::UploadLisp { lisp } => transport.upload_lisp(lisp),
            // VESC Tool treats a device-side rejection as a successful set-running request.
            Self::SetLispRunning => match transport.set_running(true) {
                Ok(()) | Err(PackageInstallError::Device(_)) => Ok(()),
                Err(error) => Err(error),
            },
        }
    }
}

struct PackageInstallAttempt<'a, T> {
    transport: &'a T,
    steps: Vec<PackageInstallStep>,
    first_error: Option<PackageInstallError>,
}

impl<'a, T: PackageInstallTransport> PackageInstallAttempt<'a, T> {
    #[must_use]
    fn new(transport: &'a T) -> Self {
        Self {
            transport,
            steps: Vec::new(),
            first_error: None,
        }
    }

    fn run(mut self, action: PackageInstallAction<'_>) -> Self {
        if self.first_error.is_none() {
            self.run_action(action);
        }
        self
    }

    fn run_always(mut self, action: PackageInstallAction<'_>) -> Self {
        self.run_action(action);
        self
    }

    fn fail(mut self, error: PackageInstallError) -> Self {
        self.first_error.get_or_insert(error);
        self
    }

    fn reload(mut self) -> Self {
        match self.transport.reload_firmware() {
            Ok(()) => self.steps.push(PackageInstallStep::ReloadFirmware),
            Err(error) => {
                self.first_error
                    .get_or_insert_with(|| step_error("reload firmware", error));
            }
        }
        self
    }

    fn finish(self, package_name: String) -> Result<PackageInstallReport, PackageInstallError> {
        self.first_error.map_or_else(
            || {
                Ok(PackageInstallReport {
                    package_name,
                    steps: self.steps,
                })
            },
            Err,
        )
    }

    fn run_action(&mut self, action: PackageInstallAction<'_>) {
        let step = action.step();
        match action.execute(self.transport) {
            Ok(()) => self.steps.push(step),
            Err(error) => {
                self.first_error
                    .get_or_insert_with(|| step_error(step.label(), error));
            }
        }
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
    let qml = package
        .qml_app_ui
        .as_ref()
        .map(|app_ui| qml_compress(&app_ui.source).map(|qml| (qml, app_ui.mode.is_fullscreen())))
        .transpose()?;
    let attempt = PackageInstallAttempt::new(transport);

    let (attempt, qml_actions) = match qml.as_ref() {
        Some((qml, fullscreen)) => (
            attempt,
            [
                Some(PackageInstallAction::EraseQml {
                    bytes: qml.len() + 100,
                }),
                Some(PackageInstallAction::UploadQml {
                    qml: qml.as_slice(),
                    fullscreen: *fullscreen,
                }),
            ],
        ),
        None => match transport.has_qml_app() {
            Ok(true) => (
                attempt,
                [
                    Some(PackageInstallAction::EraseQml {
                        bytes: PACKAGE_ERASE_BYTES,
                    }),
                    None,
                ],
            ),
            Ok(false) => (attempt, [None, None]),
            Err(error) => (
                attempt.fail(step_error("read QML App UI state", error)),
                [None, None],
            ),
        },
    };

    let lisp = (!package.lisp_data.is_empty()).then_some(package.lisp_data.as_slice());
    let lisp_actions = [
        Some(PackageInstallAction::EraseLisp {
            bytes: lisp.map_or(PACKAGE_ERASE_BYTES, |data| data.len() + 100),
        }),
        lisp.map(|lisp| PackageInstallAction::UploadLisp { lisp }),
        lisp.map(|_| PackageInstallAction::SetLispRunning),
    ];

    qml_actions
        .into_iter()
        .chain(lisp_actions)
        .flatten()
        .fold(attempt, PackageInstallAttempt::run)
        .reload()
        .finish(package.name.clone())
}

/// Erases any installed package payloads from the target and reloads firmware state.
pub fn erase_package<T: PackageInstallTransport>(
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    // Source: third_party/vesc_tool/codeloader.cpp:1072-1090.
    // uninstallVescPackage() erases Lisp, then QML, then reloads firmware.
    PackageInstallAttempt::new(transport)
        .run_always(PackageInstallAction::EraseLisp {
            bytes: PACKAGE_ERASE_BYTES,
        })
        .run_always(PackageInstallAction::EraseQml {
            bytes: PACKAGE_ERASE_BYTES,
        })
        .reload()
        .finish("installed package".to_owned())
}

fn checked_package(package: &VescPackage) -> Result<&VescPackage, PackageInstallError> {
    package.validate_for_install()?;
    Ok(package)
}

fn qml_compress(script: &str) -> Result<Vec<u8>, PackageInstallError> {
    // Source: third_party/vesc_tool/codeloader.cpp:750-754.
    let raw = format!("import Vedder.vesc.vescinterface 1.0;import \"qrc:/mobile\";{script}");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(raw.as_bytes())
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;
    let compressed = encoder
        .finish()
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;
    let len = u32::try_from(raw.len()).map_err(|_| {
        PackageInstallError::Io("QML payload exceeds qCompress length limit".to_owned())
    })?;

    Ok(len.to_be_bytes().into_iter().chain(compressed).collect())
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
    let transport = BtlePackageInstallTransport::new_with_progress()?;
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
        PackageInstallError::InvalidPackage(reason) => PackageInstallError::InvalidPackage(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PackageInstallAction, PackageInstallAttempt, PackageInstallError, PackageInstallStep,
        PackageInstallTransport, decode_package, erase_package, install_package,
    };
    use flate2::{Compression, write::ZlibEncoder};
    use std::cell::{Cell, RefCell};
    use std::io::Write;

    #[derive(Debug, Default)]
    struct FakePackageInstallTransport {
        reject_erase_lisp: Cell<bool>,
        reject_set_running_true: Cell<bool>,
        fail_set_running_true_io: Cell<bool>,
        steps: RefCell<Vec<PackageInstallStep>>,
    }

    impl FakePackageInstallTransport {
        fn reject_erase_lisp(&self) {
            self.reject_erase_lisp.set(true);
        }
        fn reject_set_running_true(&self) {
            self.reject_set_running_true.set(true);
        }
        fn fail_set_running_true_io(&self) {
            self.fail_set_running_true_io.set(true);
        }
    }

    impl PackageInstallTransport for FakePackageInstallTransport {
        fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
            Ok(false)
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
                Err(PackageInstallError::Device(
                    "device rejected the package erase".to_owned(),
                ))
            } else {
                Ok(())
            }
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

    fn build_package_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        write_string(&mut data, "VESC Packet");
        write_field(&mut data, "name", b"A minimal package");
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
    fn attempt_stops_after_error_and_still_reloads() {
        let transport = FakePackageInstallTransport::default();
        transport.reject_erase_lisp();

        let error = PackageInstallAttempt::new(&transport)
            .run(PackageInstallAction::EraseLisp {
                bytes: super::PACKAGE_ERASE_BYTES,
            })
            .run(PackageInstallAction::EraseQml {
                bytes: super::PACKAGE_ERASE_BYTES,
            })
            .reload()
            .finish("Float Out Boy".to_owned())
            .expect_err("install should fail");

        assert!(error.to_string().contains("erase Lisp 16 bytes"));
        assert_eq!(
            &*transport.steps.borrow(),
            &[
                PackageInstallStep::EraseLisp {
                    bytes: super::PACKAGE_ERASE_BYTES,
                },
                PackageInstallStep::ReloadFirmware,
            ]
        );
    }

    #[test]
    fn decodes_a_compressed_vesc_package() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let qml = package.qml_app_ui.as_ref().expect("QML App UI");

        assert_eq!(package.name, "A minimal package");
        assert_eq!(qml.source, "import QtQuick 2.15\nItem {}\n");
        assert_eq!(qml.mode, crate::package::QmlAppUiMode::Fullscreen);
        assert!(package.validate_for_install().is_ok());
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
        let qml = package.qml_app_ui.as_ref().expect("QML App UI");
        let qml = super::qml_compress(&qml.source).expect("qml");

        let report = install_package(&package, &transport).expect("report");

        // Source: third_party/vesc_tool/codeloader.cpp:982-1024
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
    fn install_ignores_set_running_rejection_like_vesc_tool() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        transport.reject_set_running_true();

        let report = install_package(&package, &transport).expect("report");

        // Source: third_party/vesc_tool/codeloader.cpp:1014-1016 and
        // third_party/vesc_tool/commands.cpp:2234-2240. VESC Tool sends
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
        let qml = package.qml_app_ui.as_ref().expect("QML App UI");
        let qml = super::qml_compress(&qml.source).expect("qml");

        let error = install_package(&package, &transport).expect_err("install should fail");

        assert!(error.to_string().contains(&format!(
            "erase Lisp {} bytes",
            package.lisp_data.len() + 100
        )));
        // Source: third_party/vesc_tool/codeloader.cpp:982-1024
        // QML writes complete before the later Lisp erase fails; reload still runs.
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
    fn erases_package_in_vesc_tool_order() {
        let transport = FakePackageInstallTransport::default();
        let report = erase_package(&transport).expect("report");
        assert_eq!(report.package_name, "installed package");
        // Source: third_party/vesc_tool/codeloader.cpp:1083-1089
        // uninstallVescPackage() erases Lisp, QML, then reloads.
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
    fn erase_package_reloads_after_lisp_erase_failure() {
        let transport = FakePackageInstallTransport::default();
        transport.reject_erase_lisp();

        let error = erase_package(&transport).expect_err("erase should fail");

        assert!(error.to_string().contains("erase Lisp 16 bytes"));
        // Source: third_party/vesc_tool/codeloader.cpp:1083-1089
        // QML erase and reload still run after the Lisp erase reports failure.
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
