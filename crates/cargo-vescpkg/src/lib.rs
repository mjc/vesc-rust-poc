//! Command-line tool for building, installing, and debugging Rust VESC packages.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

mod build;
mod package;
mod package_format;
mod package_wire;

#[derive(Debug, Parser)]
#[command(
    name = "cargo-vescpkg",
    bin_name = "cargo vescpkg",
    about = "Build, install, and debug Rust VESC packages",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
enum Command {
    Build(BuildArgs),
    Layout,
    Status,
    Scan,
    Loopback(DeviceArgs),
    LispProbe(DeviceArgs),
    LispEval(LispEvalArgs),
    LispStop(DeviceArgs),
    LispReadCode(DeviceArgs),
    QmlAppRead(DeviceArgs),
    AllocSmokeProbe(DeviceArgs),
    RefloatProbe(DeviceArgs),
    PackageInstall(PackageInstallArgs),
    ErasePackage(PackageEraseArgs),
    Deploy(PackageInstallArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[group(skip)]
struct BuildArgs {
    #[arg(short = 'p', long)]
    package: String,
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    #[arg(long, default_value = "thumbv7em-none-eabihf")]
    target: String,
    #[arg(long, default_value = "release")]
    profile: String,
    #[arg(long)]
    features: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct DeviceArgs {
    #[arg(long = "device")]
    device_name: Option<String>,
    #[arg(long)]
    address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct LispEvalArgs {
    expression: String,
    #[command(flatten)]
    device: DeviceArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct PackageInstallArgs {
    package_path: String,
    #[command(flatten)]
    device: DeviceArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct PackageEraseArgs {
    #[command(flatten)]
    device: DeviceArgs,
    #[arg(long)]
    no_preflight: bool,
}

/// Run a parsed CLI invocation and return the process exit code.
pub fn run_args<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match parse_args(args) {
        Ok(Command::Build(args)) => run_build(args),
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
            let target = loopback_target(command.device.address, command.device.device_name);

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
        Ok(Command::AllocSmokeProbe(command)) => run_alloc_smoke_probe(command),
        Ok(Command::RefloatProbe(command)) => run_refloat_probe(command),
        Ok(Command::Deploy(command)) => {
            let package_path = command.package_path;
            let target = loopback_target(command.device.address, command.device.device_name);

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
        Err(error) => {
            let exit_code = u8::try_from(error.exit_code()).unwrap_or(2);
            let _ = error.print();
            ExitCode::from(exit_code)
        }
    }
}

fn run_build(args: BuildArgs) -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };

    let options = build::BuildOptions::new(
        args.package,
        args.manifest_path,
        args.target,
        args.profile,
        args.features,
    );
    match build::build_package(&root, &options) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cargo vescpkg package failed: {error}");
            ExitCode::from(1)
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

fn run_alloc_smoke_probe(command: DeviceArgs) -> ExitCode {
    let target = loopback_target(command.address, command.device_name);

    match alloc_smoke_probe::run_alloc_smoke_probe(target) {
        Ok(report) => {
            println!(
                "alloc smoke probe ok on device={} service={}: response={}",
                report.target().device_name_hint(),
                report.target().service_name_hint(),
                String::from_utf8_lossy(report.response())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("alloc smoke probe failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_refloat_probe(command: DeviceArgs) -> ExitCode {
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

fn run_lisp_stop(command: DeviceArgs) -> ExitCode {
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

    let preview = script.chars().take(PREVIEW_CHARS).collect::<String>();
    if script.chars().count() > PREVIEW_CHARS {
        format!("{preview}...")
    } else {
        preview
    }
}

fn run_package_install(command: PackageInstallArgs) -> ExitCode {
    let target = loopback_target(command.device.address, command.device.device_name);
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

fn run_package_erase(command: PackageEraseArgs) -> ExitCode {
    let target = loopback_target(command.device.address, command.device.device_name);
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

fn parse_args<I, S>(args: I) -> Result<Command, clap::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    Cli::try_parse_from(args).map(|cli| cli.command)
}

mod ble_discovery;

/// Alloc-smoke package app-data probe runner.
pub mod alloc_smoke_probe;
/// BLE UART transport, discovery, and Lisp probe helpers.
pub mod btle;
pub mod deploy;
/// Loopback protocol runner and transport abstractions.
pub mod loopback;
pub mod loopback_debug;
pub mod package_install;
/// In-process package runtime used by host-side loopback tests.
pub mod package_runtime;
/// Package install transport implementations and BLE command helpers.
pub mod package_transport;
/// VESC UART packet encoding, decoding, and checksum helpers.
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{
        Command, DeviceArgs, PackageEraseArgs, PackageInstallArgs, parse_args, qml_preview,
    };

    #[test]
    fn parse_args_builds_typed_package_options() {
        let command = parse_args([
            "cargo-vescpkg",
            "build",
            "--package",
            "vesc-example-alloc-smoke",
            "--target",
            "thumbv7em-none-eabihf",
        ])
        .expect("parse build command");

        let Command::Build(args) = command else {
            panic!("expected build command");
        };
        assert_eq!(args.package, "vesc-example-alloc-smoke");
        assert_eq!(args.target, "thumbv7em-none-eabihf");
        assert_eq!(args.profile, "release");
    }

    #[test]
    fn parse_args_maps_commands_and_shared_device_flags() {
        assert_eq!(
            parse_args(["cargo-vescpkg", "layout"]).expect("layout"),
            Command::Layout
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "loopback",
                "--device",
                "Floatwheel PintV",
                "--address",
                "AA:BB:CC:DD:EE:FF",
            ])
            .expect("loopback"),
            Command::Loopback(DeviceArgs {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            })
        );
    }

    #[test]
    fn parse_args_uses_clap_errors_for_invalid_commands() {
        assert_eq!(
            parse_args(["cargo-vescpkg"])
                .expect_err("missing subcommand")
                .kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "spoon"])
                .expect_err("unknown subcommand")
                .kind(),
            clap::error::ErrorKind::InvalidSubcommand
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "lisp-eval"])
                .expect_err("missing expression")
                .kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--force"])
                .expect_err("unknown argument")
                .kind(),
            clap::error::ErrorKind::UnknownArgument
        );
    }

    #[test]
    fn parse_args_maps_probe_and_package_commands() {
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "refloat-probe",
                "--device",
                "VESC BLE UART",
            ])
            .expect("refloat probe"),
            Command::RefloatProbe(DeviceArgs {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            })
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "package-install",
                "foo.vescpkg",
                "--address",
                "AA:BB:CC:DD:EE:FF",
            ])
            .expect("package install"),
            Command::PackageInstall(PackageInstallArgs {
                package_path: "foo.vescpkg".to_owned(),
                device: DeviceArgs {
                    device_name: None,
                    address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
                },
            })
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--device",
                "VESC BLE UART",
                "--no-preflight",
            ])
            .expect("erase package"),
            Command::ErasePackage(PackageEraseArgs {
                device: DeviceArgs {
                    device_name: Some("VESC BLE UART".to_owned()),
                    address: None,
                },
                no_preflight: true,
            })
        );
    }

    #[test]
    fn parse_args_requires_a_package() {
        assert_eq!(
            parse_args(["cargo-vescpkg", "build"])
                .expect_err("missing package")
                .kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn qml_preview_marks_truncated_scripts() {
        assert_eq!(qml_preview("short"), "short");
        assert!(qml_preview(&"x".repeat(81)).ends_with("..."));
    }
}
