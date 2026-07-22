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
    about = "Build, install, and probe Rust VESC packages",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
enum Command {
    Build(BuildArgs),
    #[command(name = "loopback")]
    Probe(DeviceArgs),
    #[command(name = "control-loop")]
    ControlLoopProbe(DeviceArgs),
    PackageInstall(PackageInstallArgs),
    ErasePackage(PackageEraseArgs),
    Deploy(DeployArgs),
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

impl DeviceArgs {
    fn into_target(self) -> loopback::LoopbackTarget {
        loopback_target(self.address, self.device_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct PackageInstallArgs {
    package_path: PathBuf,
    #[command(flatten)]
    device: DeviceArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct DeployArgs {
    #[command(flatten)]
    build: BuildArgs,
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
        Ok(Command::Probe(command)) => run_probe(command),
        Ok(Command::ControlLoopProbe(command)) => run_control_loop_probe(command),
        Ok(Command::Deploy(command)) => run_deploy(command),
        Ok(Command::PackageInstall(command)) => run_package_install(command),
        Ok(Command::ErasePackage(command)) => run_package_erase(command),
        Err(error) => {
            let exit_code = u8::try_from(error.exit_code()).unwrap_or(2);
            let _ = error.print();
            ExitCode::from(exit_code)
        }
    }
}

fn print_loopback_report(report: &loopback::LoopbackReport) {
    println!(
        "loopback ok on device={} service={}: {:?}",
        report.target().device_name_hint(),
        report.target().service_name_hint(),
        report.commands()
    );
}

fn run_probe(command: DeviceArgs) -> ExitCode {
    let target = command.into_target();
    match deploy::run_loopback_probe(target, |event| println!("loopback: {event}")) {
        Ok(report) => {
            print_loopback_report(&report);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("loopback failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_control_loop_probe(command: DeviceArgs) -> ExitCode {
    let target = command.into_target();
    match deploy::run_control_loop_probe(target, |event| println!("control-loop: {event}")) {
        Ok(report) => {
            let first = report
                .statuses()
                .first()
                .map_or(0, |status| status.tick_count());
            let last = report
                .statuses()
                .last()
                .map_or(0, |status| status.tick_count());
            println!(
                "control-loop ok on device={} service={}: ticks={first}->{last} elapsed={:?} tick-period={:?}..{:?} jitter={:?}",
                report.target().device_name_hint(),
                report.target().service_name_hint(),
                report.elapsed(),
                report.timing().min_tick_period(),
                report.timing().max_tick_period(),
                report.timing().jitter(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("control-loop failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_deploy(command: DeployArgs) -> ExitCode {
    match build_package(command.build) {
        Ok(package_path) => run_package_install(PackageInstallArgs {
            package_path,
            device: command.device,
        }),
        Err(error) => {
            eprintln!("deploy failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_build(args: BuildArgs) -> ExitCode {
    match build_package(args) {
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

fn build_package(args: BuildArgs) -> Result<PathBuf, String> {
    let root = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;

    let options = build::BuildOptions {
        package: args.package,
        manifest_path: args.manifest_path,
        target: args.target,
        profile: args.profile,
        features: args.features,
    };
    build::build_package(&root, &options).map_err(|error| error.to_string())
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
    let target = command.device.into_target();
    match package_install::install_over_ble(command.package_path, target) {
        Ok(report) => {
            println!("Installed {}", report.package_name);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package install failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_package_erase(command: PackageEraseArgs) -> ExitCode {
    let target = command.device.into_target();
    match package_install::erase_over_ble(target, command.no_preflight) {
        Ok(_) => {
            println!("Package erased");
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
    let mut args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "vescpkg") {
        args.remove(1);
    }
    Cli::try_parse_from(args).map(|cli| cli.command)
}

mod ble_discovery;

pub mod deploy;
/// Loopback target and report types.
pub mod loopback;
pub mod package_install;
/// Package install transport implementations and BLE command helpers.
pub mod package_transport;
/// VESC UART packet encoding, decoding, and checksum helpers.
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{
        BuildArgs, Command, DeployArgs, DeviceArgs, PackageEraseArgs, PackageInstallArgs,
        parse_args,
    };

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
    fn parse_args_accepts_the_cargo_subcommand_shim() {
        let command = parse_args([
            "cargo-vescpkg",
            "vescpkg",
            "build",
            "--package",
            "minimal-package",
        ])
        .expect("parse Cargo subcommand invocation");

        assert!(matches!(command, Command::Build(_)));
    }

    #[test]
    fn parse_args_maps_commands_and_shared_device_flags() {
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
            Command::Probe(DeviceArgs {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            })
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "control-loop",
                "--device",
                "Floatwheel PintV",
            ])
            .expect("control-loop"),
            Command::ControlLoopProbe(DeviceArgs {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
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
                package_path: "foo.vescpkg".into(),
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
    fn parse_args_maps_deploy_package_and_device_flags() {
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "deploy",
                "-p",
                "vesc-example-loopback",
                "--device",
                "VESC BLE UART",
            ])
            .expect("deploy package"),
            Command::Deploy(DeployArgs {
                build: BuildArgs {
                    package: "vesc-example-loopback".to_owned(),
                    manifest_path: None,
                    target: "thumbv7em-none-eabihf".to_owned(),
                    profile: "release".to_owned(),
                    features: None,
                },
                device: DeviceArgs {
                    device_name: Some("VESC BLE UART".to_owned()),
                    address: None,
                },
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
