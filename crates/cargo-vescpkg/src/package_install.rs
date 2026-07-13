//! Host-side VESC package install protocol.
//!
//! Package construction and `.vescpkg` decoding live in this crate.
//! Device install is a CLI concern because it owns transport, firmware state,
//! and operator-facing recovery behavior.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::path::Path;

use crate::loopback::LoopbackTarget;
use crate::package_transport::BtlePackageInstallTransport;

const PACKAGE_ERASE_BYTES: usize = 16;

pub use crate::package::Package as VescPackage;

/// Steps emitted while installing or erasing a package over the firmware transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallStep {
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

impl From<crate::package::PackageError> for PackageInstallError {
    fn from(error: crate::package::PackageError) -> Self {
        match error {
            crate::package::PackageError::Io(error) => Self::Io(error.to_string()),
            crate::package::PackageError::InvalidPackage => Self::InvalidPackage,
            other => Self::Io(other.to_string()),
        }
    }
}

/// Firmware-side transport operations needed to install or erase a package.
pub trait PackageInstallTransport {
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
    reject_erase_lisp: Cell<bool>,
    reject_set_running_true: Cell<bool>,
    fail_set_running_true_io: Cell<bool>,
    /// Recorded transport steps for assertions in tests and golden checks.
    pub steps: RefCell<Vec<PackageInstallStep>>,
}

impl FakePackageInstallTransport {
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

    let mut steps = Vec::new();
    let mut first_error = None;
    let lisp = (!package.lisp_data.is_empty()).then_some(package.lisp_data.as_slice());
    let erase_bytes = lisp.map_or(PACKAGE_ERASE_BYTES, |data| data.len() + 100);

    try_step(
        &mut steps,
        &mut first_error,
        PackageInstallStep::EraseLisp { bytes: erase_bytes },
        || transport.erase_lisp(erase_bytes),
    );
    if first_error.is_none()
        && let Some(lisp) = lisp
    {
        try_step(
            &mut steps,
            &mut first_error,
            PackageInstallStep::UploadLisp { bytes: lisp.len() },
            || transport.upload_lisp(lisp),
        );
        if first_error.is_none() {
            match transport.set_running(true) {
                Ok(()) => steps.push(PackageInstallStep::SetRunning { running: true }),
                Err(PackageInstallError::Device(_)) => {
                    steps.push(PackageInstallStep::SetRunning { running: true });
                }
                Err(error) => first_error = Some(step_error("set Lisp running true", error)),
            }
        }
    }
    let reload = PackageInstallStep::ReloadFirmware;
    match transport.reload_firmware() {
        Ok(()) => steps.push(reload),
        Err(error) => {
            first_error.get_or_insert(step_error("reload firmware", error));
        }
    }

    first_error.map_or_else(
        || {
            Ok(PackageInstallReport {
                package_name: package.name.clone(),
                steps,
            })
        },
        Err,
    )
}

/// Erases any installed package payloads from the target and reloads firmware state.
pub fn erase_package<T: PackageInstallTransport>(
    transport: &T,
) -> Result<PackageInstallReport, PackageInstallError> {
    // Source: third_party/vesc_tool/codeloader.cpp:1072-1090
    // uninstallVescPackage() erases Lisp, reloads firmware, and returns the
    // erase result.
    let mut steps = Vec::new();
    let mut first_error = None;
    try_step(
        &mut steps,
        &mut first_error,
        PackageInstallStep::EraseLisp {
            bytes: PACKAGE_ERASE_BYTES,
        },
        || transport.erase_lisp(PACKAGE_ERASE_BYTES),
    );
    let reload = PackageInstallStep::ReloadFirmware;
    match transport.reload_firmware() {
        Ok(()) => steps.push(reload),
        Err(error) => {
            first_error.get_or_insert(step_error("reload firmware", error));
        }
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

fn try_step(
    steps: &mut Vec<PackageInstallStep>,
    first_error: &mut Option<PackageInstallError>,
    step: PackageInstallStep,
    run: impl FnOnce() -> Result<(), PackageInstallError>,
) {
    if first_error.is_some() {
        return;
    }
    let label = match &step {
        PackageInstallStep::EraseLisp { bytes } => format!("erase Lisp {bytes} bytes"),
        PackageInstallStep::UploadLisp { bytes } => format!("upload Lisp {bytes} bytes"),
        PackageInstallStep::SetRunning { running } => format!("set Lisp running {running}"),
        PackageInstallStep::ReloadFirmware => "reload firmware".to_owned(),
    };
    match run() {
        Ok(()) => steps.push(step),
        Err(error) => *first_error = Some(step_error(label, error)),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        FakePackageInstallTransport, PackageInstallError, PackageInstallStep, decode_package,
        erase_package, install_package,
    };
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;

    fn build_package_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        write_string(&mut data, "VESC Packet");
        write_field(&mut data, "name", b"A minimal package");
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
        assert_eq!(package.name, "A minimal package");
        assert!(package.is_valid());
    }

    #[test]
    fn installer_runs_install_and_erase() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();

        install_package(&package, &transport).expect("install");
        erase_package(&transport).expect("erase");
        assert_eq!(transport.steps.borrow().len(), 6);
    }

    #[test]
    fn installs_package_in_vesc_tool_order() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakePackageInstallTransport::default();
        let report = install_package(&package, &transport).expect("report");

        // Source: third_party/vesc_tool/codeloader.cpp:994-1024
        // installVescPackage() runs Lisp erase/upload, lispSetRunning(1),
        // then sleep/reload.
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
        let error = install_package(&package, &transport).expect_err("install should fail");

        assert!(error.to_string().contains(&format!(
            "erase Lisp {} bytes",
            package.lisp_data.len() + 100
        )));
        // Source: third_party/vesc_tool/codeloader.cpp:1007-1024
        // installVescPackage() stops later Lisp work when res goes false, but
        // still sleeps and reloads firmware before returning res.
        assert_eq!(
            &*transport.steps.borrow(),
            &[
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
        // uninstallVescPackage() erases Lisp, reloads, then returns the result.
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
    fn erase_package_reloads_after_lisp_erase_failure() {
        let transport = FakePackageInstallTransport::default();
        transport.reject_erase_lisp();

        let error = erase_package(&transport).expect_err("erase should fail");

        assert!(error.to_string().contains("erase Lisp 16 bytes"));
        // Reload still runs after the Lisp erase reports failure.
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
    fn install_step_errors_keep_the_failed_step_name() {
        let error = super::step_error(
            "erase Lisp 407 bytes",
            PackageInstallError::Device("timed out waiting for a BLE reply".to_owned()),
        );
        assert!(error.to_string().contains("erase Lisp 407 bytes"));
    }
}
