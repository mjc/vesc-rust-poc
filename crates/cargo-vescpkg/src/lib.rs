//! Command-line tool for building, installing, and debugging Rust VESC packages.

use std::process::ExitCode;

/// Parsed top-level command requested by the operator-facing CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print CLI usage information.
    Help,
    /// Print package layout information.
    Layout,
    /// Print host and build status information.
    Status,
    /// Scan for nearby VESC BLE UART devices.
    Scan,
    /// Run the BLE loopback protocol against a target device.
    Loopback(LoopbackCommand),
    /// Run the Lisp probe diagnostic against a target device.
    LispProbe(LispProbeCommand),
    /// Run one Lisp REPL form against a target device.
    LispEval(LispEvalCommand),
    /// Stop the running Lisp/native package on a target device.
    LispStop(LispStopCommand),
    /// Read back the installed Lisp/code payload from a target device.
    LispReadCode(LispReadCodeCommand),
    /// Read back the installed QML app payload from a target device.
    QmlAppRead(QmlAppReadCommand),
    /// Send a Refloat app-data handshake probe to a target device.
    RefloatProbe(LoopbackCommand),
    /// Install a package on a target device.
    PackageInstall(PackageInstallCommand),
    /// Erase an installed package from a target device.
    ErasePackage(PackageEraseCommand),
    /// Build, install, and smoke-test a package on a target device.
    Deploy(PackageInstallCommand),
}

/// Arguments for the `loopback` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `lisp-probe` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProbeCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `lisp-eval` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispEvalCommand {
    /// Lisp REPL form to evaluate.
    pub expression: String,
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `lisp-stop` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispStopCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `lisp-read-code` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispReadCodeCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `qml-app-read` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QmlAppReadCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for package install and deploy commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInstallCommand {
    /// Path to the `.vescpkg` file to install.
    pub package_path: String,
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `erase-package` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEraseCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
    /// Skip firmware-version preflight before erasing.
    pub no_preflight: bool,
}

/// Errors returned while parsing CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first non-program argument did not match a supported command.
    UnknownCommand(String),
}

const HELP_TEXT: &str = "cargo vescpkg: use `build`, `layout`, `status`, `scan`, `loopback`, `lisp-probe`, `lisp-eval`, `lisp-stop`, `lisp-read-code`, `qml-app-read`, `refloat-probe`, `deploy`, `package-install`, or `erase-package`";

/// Run a parsed CLI invocation and return the process exit code.
pub fn run_args<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match parse_args(args) {
        Ok(Command::Help) => {
            println!("{HELP_TEXT}");
            ExitCode::SUCCESS
        }
        Ok(Command::Layout) => {
            println!("workspace layout is documented in docs/workspace-layout.md");
            ExitCode::SUCCESS
        }
        Ok(Command::Status) => {
            println!("status: placeholder host surface");
            ExitCode::SUCCESS
        }
        Ok(Command::Scan) => match btle::scan_devices() {
            Ok(devices) => {
                devices.into_iter().for_each(|device| {
                    println!(
                        "{} {:?} {:?}",
                        device.identifier, device.local_name, device.services
                    );
                });
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("scan failed: {error}");
                ExitCode::from(1)
            }
        },
        Ok(Command::Loopback(command)) => {
            let target = loopback_target(command.address, command.device_name);

            match loopback_debug::run_loopback_with_diagnostics(target, |event| {
                if event.should_print_to_cli() {
                    println!("loopback: {}", event.describe());
                }
            }) {
                Ok(report) => {
                    println!(
                        "loopback ok on device={} service={}: {:?}",
                        report.target().device_name_hint(),
                        report.target().service_name_hint(),
                        report.commands()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("loopback failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::LispProbe(command)) => {
            let target = loopback_target(command.address, command.device_name);

            match btle::run_lisp_probe_with_progress(target, |event| {
                if event.should_print_to_cli() {
                    println!("lisp probe: {}", event.describe());
                }
            }) {
                Ok(report) => {
                    let ok = report
                        .prints()
                        .iter()
                        .any(|line| line.contains("vesc-rust-probe-ok-42"));
                    if ok {
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("lisp probe: missing expected vesc-rust-probe-ok-42 print");
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("lisp probe failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::LispEval(command)) => {
            let target = loopback_target(command.address, command.device_name);

            match btle::run_lisp_eval_with_progress(target, &command.expression, |event| {
                if event.should_print_to_cli() {
                    println!("lisp eval: {}", event.describe());
                }
            }) {
                Ok(report) => {
                    report.prints().iter().for_each(|line| println!("{line}"));
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("lisp eval failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::LispStop(command)) => run_lisp_stop(command),
        Ok(Command::LispReadCode(command)) => {
            let target = loopback_target(command.address, command.device_name);
            match read_lisp_code(target) {
                Ok(read) => {
                    println!(
                        "lisp code: total={} offset={} bytes={} preview={}",
                        read.total_len,
                        read.offset,
                        read.data.len(),
                        loopback_debug::hex_snippet(&read.data, 64)
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("lisp read-code failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::QmlAppRead(command)) => {
            let target = loopback_target(command.address, command.device_name);
            match read_qml_app(target) {
                Ok(read) => {
                    println!(
                        "qml app: has_qml_app={} compressed={} decompressed={} preview={}",
                        read.has_qml_app,
                        read.compressed.len(),
                        read.decompressed.as_ref().map_or(0, String::len),
                        read.decompressed
                            .as_deref()
                            .map_or_else(|| "<none>".to_owned(), qml_preview)
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("qml app read failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::RefloatProbe(command)) => run_refloat_probe(command),
        Ok(Command::Deploy(command)) => {
            let package_path = command.package_path;
            let target = loopback_target(command.address, command.device_name);

            match deploy::run_deploy(&package_path, target, |event| {
                if event.should_print_to_cli() {
                    println!("loopback: {}", event.describe());
                }
            }) {
                Ok((install, report)) => {
                    println!(
                        "package install ok for {}: {:?}",
                        install.package_name, install.steps
                    );
                    println!(
                        "loopback ok on device={} service={}: {:?}",
                        report.target().device_name_hint(),
                        report.target().service_name_hint(),
                        report.commands()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("deploy failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::PackageInstall(command)) => run_package_install(command),
        Ok(Command::ErasePackage(command)) => run_package_erase(command),
        Err(ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}

fn loopback_target(
    address: Option<String>,
    device_name: Option<String>,
) -> loopback::LoopbackTarget {
    match (address, device_name) {
        (Some(address), _) => loopback::LoopbackTarget::addressed(address),
        (None, Some(device_name)) => loopback::LoopbackTarget::named(device_name),
        (None, None) => loopback::LoopbackTarget::default(),
    }
}

fn run_refloat_probe(command: LoopbackCommand) -> ExitCode {
    let target = loopback_target(command.address, command.device_name);

    loopback_debug::run_refloat_probe_with_diagnostics(target, |event| {
        std::iter::once(event)
            .filter(loopback_debug::LoopbackProgress::should_print_to_cli)
            .for_each(|event| println!("refloat probe: {}", event.describe()));
    })
    .map_or_else(
        |error| {
            eprintln!("refloat probe failed: {error}");
            ExitCode::from(1)
        },
        |response| {
            println!(
                "refloat probe ok: response_len={} response={}",
                response.len(),
                loopback_debug::hex_snippet(&response, 96)
            );
            ExitCode::SUCCESS
        },
    )
}

fn read_lisp_code(
    target: loopback::LoopbackTarget,
) -> Result<package_transport::LispCodeRead, package_install::PackageInstallError> {
    const READBACK_LEN: u32 = 384;

    let transport = package_transport::BtlePackageInstallTransport::new()?;
    transport.open(target)?;
    let read = transport.read_lisp_code(0, READBACK_LEN);
    transport.close();
    read
}

fn run_lisp_stop(command: LispStopCommand) -> ExitCode {
    let target = loopback_target(command.address, command.device_name);
    match stop_lisp_running(target) {
        Ok(()) => {
            println!("lisp stop ok");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("lisp stop failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn stop_lisp_running(
    target: loopback::LoopbackTarget,
) -> Result<(), package_install::PackageInstallError> {
    let transport = package_transport::BtlePackageInstallTransport::new()?;
    transport.open_without_preflight(target)?;
    let stopped = transport.stop_running_recovery();
    transport.close();
    stopped
}

fn read_qml_app(
    target: loopback::LoopbackTarget,
) -> Result<package_transport::QmlAppRead, package_install::PackageInstallError> {
    let transport = package_transport::BtlePackageInstallTransport::new()?;
    transport.open(target)?;
    let read = transport.read_qml_app();
    transport.close();
    read
}

fn qml_preview(script: &str) -> String {
    const PREVIEW_CHARS: usize = 80;

    script.chars().take(PREVIEW_CHARS).collect()
}

fn run_package_install(command: PackageInstallCommand) -> ExitCode {
    let target = loopback_target(command.address, command.device_name);
    match package_install::install_over_ble(command.package_path, target) {
        Ok(report) => {
            println!(
                "package install ok for {}: {:?}",
                report.package_name, report.steps
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package install failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_package_erase(command: PackageEraseCommand) -> ExitCode {
    let target = loopback_target(command.address, command.device_name);
    match package_install::erase_over_ble(target, command.no_preflight) {
        Ok(report) => {
            println!("package erase ok: {:?}", report.steps);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package erase failed: {error}");
            ExitCode::from(1)
        }
    }
}

/// Parses command-line arguments into a top-level CLI command.
pub fn parse_args<I, S>(args: I) -> Result<Command, ParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());
    let _program = iter.next();

    match iter.next().as_deref() {
        None | Some("-h") | Some("--help") => Ok(Command::Help),
        Some("layout") => Ok(Command::Layout),
        Some("status") => Ok(Command::Status),
        Some("scan") => Ok(Command::Scan),
        Some("loopback") => parse_loopback(iter).map(Command::Loopback),
        Some("lisp-probe") => parse_lisp_probe(iter).map(Command::LispProbe),
        Some("lisp-eval") => parse_lisp_eval(iter).map(Command::LispEval),
        Some("lisp-stop") => parse_lisp_stop(iter).map(Command::LispStop),
        Some("lisp-read-code") => parse_lisp_read_code(iter).map(Command::LispReadCode),
        Some("qml-app-read") => parse_qml_app_read(iter).map(Command::QmlAppRead),
        Some("refloat-probe") => parse_loopback(iter).map(Command::RefloatProbe),
        Some("package-install") => {
            parse_package_install(iter, "package-install").map(Command::PackageInstall)
        }
        Some("deploy") => parse_package_install(iter, "deploy").map(Command::Deploy),
        Some("erase-package") => parse_erase_package(iter).map(Command::ErasePackage),
        Some(other) => Err(ParseError::UnknownCommand(other.to_owned())),
    }
}

fn parse_device_flags(
    mut iter: impl Iterator<Item = String>,
) -> Result<(Option<String>, Option<String>), ParseError> {
    let mut device_name = None;
    let mut address = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok((device_name, address))
}

fn parse_loopback(iter: impl Iterator<Item = String>) -> Result<LoopbackCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LoopbackCommand {
        device_name,
        address,
    })
}

fn parse_lisp_probe(iter: impl Iterator<Item = String>) -> Result<LispProbeCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LispProbeCommand {
        device_name,
        address,
    })
}

fn parse_lisp_eval(mut iter: impl Iterator<Item = String>) -> Result<LispEvalCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut expression = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            value if expression.is_none() => expression = Some(value.to_owned()),
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok(LispEvalCommand {
        expression: expression.ok_or_else(|| ParseError::UnknownCommand("lisp-eval".to_owned()))?,
        device_name,
        address,
    })
}

fn parse_lisp_stop(iter: impl Iterator<Item = String>) -> Result<LispStopCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LispStopCommand {
        device_name,
        address,
    })
}

fn parse_lisp_read_code(
    iter: impl Iterator<Item = String>,
) -> Result<LispReadCodeCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LispReadCodeCommand {
        device_name,
        address,
    })
}

fn parse_qml_app_read(iter: impl Iterator<Item = String>) -> Result<QmlAppReadCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(QmlAppReadCommand {
        device_name,
        address,
    })
}

fn parse_package_install(
    mut iter: impl Iterator<Item = String>,
    command_name: &'static str,
) -> Result<PackageInstallCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut package_path = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            _ if package_path.is_none() => package_path = Some(arg),
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    package_path
        .map(|package_path| PackageInstallCommand {
            package_path,
            device_name,
            address,
        })
        .ok_or_else(|| ParseError::UnknownCommand(command_name.to_owned()))
}

fn parse_erase_package(
    mut iter: impl Iterator<Item = String>,
) -> Result<PackageEraseCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut no_preflight = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--no-preflight" => no_preflight = true,
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok(PackageEraseCommand {
        device_name,
        address,
        no_preflight,
    })
}

mod ble_discovery;

/// BLE UART transport, discovery, and Lisp probe helpers.
pub mod btle;
pub mod deploy;
/// Loopback protocol runner and transport abstractions.
pub mod loopback;
pub mod loopback_debug;
pub mod package_install;
/// Package install transport implementations and BLE command helpers.
pub mod package_transport;
/// VESC UART packet encoding, decoding, and checksum helpers.
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{Command, PackageEraseCommand, PackageInstallCommand, ParseError, parse_args};
    use vesc_protocol::{WireCommand, WireVersion};

    #[test]
    fn parse_args_covers_cli_commands() {
        assert_eq!(parse_args(["cargo-vescpkg", "layout"]), Ok(Command::Layout));
        assert_eq!(parse_args(["cargo-vescpkg", "status"]), Ok(Command::Status));
        assert_eq!(parse_args(["cargo-vescpkg", "scan"]), Ok(Command::Scan));
        assert_eq!(parse_args(["cargo-vescpkg"]), Ok(Command::Help));
        assert_eq!(
            parse_args(["cargo-vescpkg", "spoon"]),
            Err(ParseError::UnknownCommand("spoon".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "package-install", "foo.vescpkg"]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--no-preflight"]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: None,
                no_preflight: true,
            }))
        );
        assert_eq!(WireVersion::CURRENT.get(), 1);
        assert_eq!(u8::from(WireCommand::Status), 3);
    }
}
