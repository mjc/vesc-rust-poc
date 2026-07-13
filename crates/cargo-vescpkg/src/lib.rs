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
    Loopback(DeviceArgs),
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
    use super::{Command, DeviceArgs, PackageEraseArgs, PackageInstallArgs, parse_args};

    #[test]
    fn parse_args_builds_typed_package_options() {
        let command = parse_args([
            "cargo-vescpkg",
            "build",
            "--package",
            "minimal-package",
            "--target",
            "thumbv7em-none-eabihf",
        ])
        .expect("parse build command");

        let Command::Build(args) = command else {
            panic!("expected build command");
        };
        assert_eq!(args.package, "minimal-package");
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
}
