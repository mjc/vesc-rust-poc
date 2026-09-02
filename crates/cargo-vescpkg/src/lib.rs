//! Command-line tool for building, installing, and debugging Rust VESC packages.

use std::num::NonZeroU16;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

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

#[derive(Debug, Clone, PartialEq, Subcommand)]
enum Command {
    Build(BuildArgs),
    AudioBeep(DeviceArgs),
    #[command(name = "loopback")]
    Probe(DeviceArgs),
    CustomAppData(CustomAppDataArgs),
    CustomConfig(DeviceArgs),
    FirmwareValues(DeviceArgs),
    FirmwareImu(FirmwareImuArgs),
    FobLog(FobLogArgs),
    LispStats(DeviceArgs),
    #[command(name = "control-loop")]
    ControlLoopProbe(DeviceArgs),
    #[command(name = "control-loop-deploy")]
    ControlLoopDeploy(DeployArgs),
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

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct CustomAppDataArgs {
    #[arg(value_delimiter = ',', num_args = 1..)]
    payload: Vec<u8>,
    #[command(flatten)]
    device: DeviceArgs,
}

#[derive(Debug, Clone, PartialEq, Args)]
struct FirmwareImuArgs {
    #[arg(long, num_args = 3, value_names = ["KP", "KI", "DECAY"])]
    set_live: Option<Vec<f32>>,
    #[arg(long, requires = "set_live")]
    store: bool,
    #[command(flatten)]
    device: DeviceArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
struct FobLogArgs {
    output: PathBuf,
    #[arg(long, default_value = "100")]
    samples: NonZeroU16,
    #[arg(long, default_value = "100")]
    interval_ms: NonZeroU16,
    #[command(flatten)]
    device: DeviceArgs,
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
        Ok(Command::AudioBeep(command)) => run_audio_beep(command),
        Ok(Command::Probe(command)) => run_probe(command),
        Ok(Command::CustomAppData(command)) => run_custom_app_data(command),
        Ok(Command::CustomConfig(command)) => run_custom_config(command),
        Ok(Command::FirmwareValues(command)) => run_firmware_values(command),
        Ok(Command::FirmwareImu(command)) => run_firmware_imu(command),
        Ok(Command::FobLog(command)) => run_fob_log(command),
        Ok(Command::LispStats(command)) => run_lisp_stats(command),
        Ok(Command::ControlLoopProbe(command)) => run_control_loop_probe(command),
        Ok(Command::ControlLoopDeploy(command)) => run_control_loop_deploy(command),
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

fn run_custom_config(command: DeviceArgs) -> ExitCode {
    match deploy::run_custom_config_probe(command.into_target()) {
        Ok(config) => {
            println!("custom config: {config:02x?}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("custom config failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_lisp_stats(command: DeviceArgs) -> ExitCode {
    match deploy::run_lisp_stats_probe(command.into_target()) {
        Ok(stats) => {
            println!(
                "lisp stats: cpu={:.2}% heap={:.2}% memory={:.2}% stack={:.2}% result={}",
                stats.cpu_usage(),
                stats.heap_usage(),
                stats.memory_usage(),
                stats.stack_usage(),
                stats.result(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("lisp stats failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_firmware_values(command: DeviceArgs) -> ExitCode {
    match deploy::run_firmware_values_probe(command.into_target()) {
        Ok(report) => {
            let version = report.firmware_version();
            let values = report.values();
            println!(
                "firmware {}.{} values: odometer={}m uptime={}ms",
                version.major(),
                version.minor(),
                values.odometer_meters(),
                values.uptime_ms(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("firmware values failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_fob_log(command: FobLogArgs) -> ExitCode {
    let report = match deploy::run_fob_log_probe(
        command.device.into_target(),
        usize::from(command.samples.get()),
        Duration::from_millis(u64::from(command.interval_ms.get())),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("FOB log failed: {error}");
            return ExitCode::from(1);
        }
    };

    let file = match std::fs::File::create(&command.output) {
        Ok(file) => file,
        Err(error) => {
            eprintln!(
                "FOB log failed to create {}: {error}",
                command.output.display()
            );
            return ExitCode::from(1);
        }
    };
    let mut output = std::io::BufWriter::new(file);
    if let Err(error) = write_fob_log_csv(&mut output, &report) {
        eprintln!(
            "FOB log failed to write {}: {error}",
            command.output.display()
        );
        return ExitCode::from(1);
    }

    let version = report.firmware_version();
    println!(
        "FOB log: firmware={}.{} samples={} output={}",
        version.major(),
        version.minor(),
        report.samples().len(),
        command.output.display(),
    );
    ExitCode::SUCCESS
}

fn write_fob_log_csv(
    output: &mut impl std::io::Write,
    report: &deploy::FobLogReport,
) -> std::io::Result<()> {
    use std::fmt::Write as _;

    writeln!(output, "host_elapsed_ms,response_len,response_hex")?;
    for sample in report.samples() {
        let mut hex = String::with_capacity(sample.response().len() * 2);
        for byte in sample.response() {
            write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
        }
        writeln!(
            output,
            "{},{},{}",
            sample.elapsed().as_millis(),
            sample.response().len(),
            hex,
        )?;
    }
    Ok(())
}

fn run_firmware_imu(command: FirmwareImuArgs) -> ExitCode {
    let settings = command
        .set_live
        .map(|values| package_transport::FirmwareImuSettings::new(values[0], values[1], values[2]));
    match deploy::run_firmware_imu_probe(command.device.into_target(), settings, command.store) {
        Ok(settings) => {
            println!(
                "firmware IMU: mahony-kp={} mahony-ki={} acceleration-confidence-decay={}",
                settings.mahony_kp(),
                settings.mahony_ki(),
                settings.acceleration_confidence_decay(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("firmware IMU read failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_custom_app_data(command: CustomAppDataArgs) -> ExitCode {
    let target = command.device.into_target();
    match deploy::run_custom_app_data_probe(target, &command.payload) {
        Ok(report) => {
            let version = report.firmware_version();
            let response = report.response();
            println!(
                "custom app-data firmware={}.{} response: {response:02x?}",
                version.major(),
                version.minor()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("custom app-data failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_audio_beep(command: DeviceArgs) -> ExitCode {
    use vesc_protocol::audio_smoke::{BeepResponse, BeepStatus, encode_beep_command};

    let target = command.into_target();
    match deploy::run_custom_app_data_probe(target, &encode_beep_command()) {
        Ok(report) => match BeepResponse::decode(report.response()).map(BeepResponse::status) {
            Ok(BeepStatus::Played) => {
                let version = report.firmware_version();
                println!(
                    "audio beep accepted on firmware={}.{}",
                    version.major(),
                    version.minor()
                );
                ExitCode::SUCCESS
            }
            Ok(status) => {
                eprintln!("audio beep was not played: {status:?}");
                ExitCode::from(1)
            }
            Err(error) => {
                eprintln!("audio beep returned an invalid response: {error:?}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("audio beep failed: {error}");
            ExitCode::from(1)
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

fn run_control_loop_deploy(command: DeployArgs) -> ExitCode {
    let target = command.device.into_target();
    let package_path = match build_package(command.build) {
        Ok(package_path) => package_path,
        Err(error) => {
            eprintln!("control-loop-deploy build failed: {error}");
            return ExitCode::from(1);
        }
    };
    match package_install::install_over_ble(&package_path, target.clone()) {
        Ok(report) => println!("Installed {}", report.package_name),
        Err(error) => {
            eprintln!("control-loop-deploy install failed: {error}");
            return ExitCode::from(1);
        }
    }
    match deploy::run_control_loop_probe(target, |event| println!("control-loop: {event}")) {
        Ok(report) => {
            println!(
                "control-loop-deploy ok: samples={} elapsed={:?} jitter={:?}",
                report.statuses().len(),
                report.elapsed(),
                report.timing().jitter(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("control-loop-deploy probe failed: {error}");
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
    use std::path::PathBuf;

    #[test]
    fn command_reference_inventories_every_clap_subcommand() {
        let reference = include_str!("../../../docs/cargo-vescpkg-command.md");
        for command in [
            "build",
            "audio-beep",
            "loopback",
            "custom-app-data",
            "custom-config",
            "firmware-values",
            "firmware-imu",
            "fob-log",
            "lisp-stats",
            "control-loop",
            "control-loop-deploy",
            "package-install",
            "erase-package",
            "deploy",
        ] {
            let row = format!("| `{command}` |");
            assert!(
                reference.contains(&row),
                "command reference is missing `{row}`"
            );
        }
    }

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
    fn parse_args_builds_a_typed_custom_app_data_probe() {
        let command = parse_args([
            "cargo-vescpkg",
            "custom-app-data",
            "101,0,2",
            "--device",
            "VESC BLE UART",
        ])
        .expect("parse custom app-data probe");

        let Command::CustomAppData(args) = command else {
            panic!("expected custom app-data command");
        };
        assert_eq!(args.payload, [101, 0, 2]);
        assert_eq!(args.device.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_read_only_custom_config_probe() {
        let command = parse_args([
            "cargo-vescpkg",
            "custom-config",
            "--device",
            "VESC BLE UART",
        ])
        .expect("parse custom-config probe");

        let Command::CustomConfig(args) = command else {
            panic!("expected custom-config command");
        };
        assert_eq!(args.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_fixed_audio_beep_probe() {
        let command = parse_args(["cargo-vescpkg", "audio-beep", "--device", "VESC BLE UART"])
            .expect("parse audio-beep probe");

        let Command::AudioBeep(args) = command else {
            panic!("expected audio-beep command");
        };
        assert_eq!(args.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_read_only_lisp_stats_probe() {
        let command = parse_args(["cargo-vescpkg", "lisp-stats", "--device", "VESC BLE UART"])
            .expect("parse Lisp stats probe");

        let Command::LispStats(args) = command else {
            panic!("expected Lisp stats command");
        };
        assert_eq!(args.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_read_only_firmware_values_probe() {
        let command = parse_args([
            "cargo-vescpkg",
            "firmware-values",
            "--device",
            "VESC BLE UART",
        ])
        .expect("parse firmware values probe");

        let Command::FirmwareValues(args) = command else {
            panic!("expected firmware values command");
        };
        assert_eq!(args.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_bounded_fob_log() {
        let command = parse_args([
            "cargo-vescpkg",
            "fob-log",
            "ride.csv",
            "--samples",
            "12",
            "--interval-ms",
            "50",
        ])
        .expect("parse FOB log");

        let Command::FobLog(args) = command else {
            panic!("expected FOB log command");
        };
        assert_eq!(args.output, PathBuf::from("ride.csv"));
        assert_eq!(args.samples.get(), 12);
        assert_eq!(args.interval_ms.get(), 50);
    }

    #[test]
    fn parse_args_builds_a_read_only_firmware_imu_probe() {
        let command = parse_args(["cargo-vescpkg", "firmware-imu", "--device", "VESC BLE UART"])
            .expect("parse firmware IMU probe");

        let Command::FirmwareImu(args) = command else {
            panic!("expected firmware IMU command");
        };
        assert_eq!(args.set_live, None);
        assert!(!args.store);
        assert_eq!(args.device.device_name.as_deref(), Some("VESC BLE UART"));
    }

    #[test]
    fn parse_args_builds_a_live_only_firmware_imu_update() {
        let command = parse_args([
            "cargo-vescpkg",
            "firmware-imu",
            "--set-live",
            "0.4",
            "0.0",
            "0.1",
        ])
        .expect("parse live firmware IMU update");

        let Command::FirmwareImu(args) = command else {
            panic!("expected firmware IMU command");
        };
        assert_eq!(args.set_live, Some(vec![0.4, 0.0, 0.1]));
        assert!(!args.store);
    }

    #[test]
    fn parse_args_requires_values_for_a_stored_firmware_imu_update() {
        assert!(parse_args(["cargo-vescpkg", "firmware-imu", "--store"]).is_err());
        let command = parse_args([
            "cargo-vescpkg",
            "firmware-imu",
            "--set-live",
            "2.0",
            "0.2",
            "0.3",
            "--store",
        ])
        .expect("parse stored firmware IMU update");
        let Command::FirmwareImu(args) = command else {
            panic!("expected firmware IMU command");
        };
        assert!(args.store);
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
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "control-loop-deploy",
                "-p",
                "vesc-example-control-loop-smoke",
                "--device",
                "Floatwheel PintV",
            ])
            .expect("control-loop-deploy"),
            Command::ControlLoopDeploy(DeployArgs {
                build: BuildArgs {
                    package: "vesc-example-control-loop-smoke".to_owned(),
                    manifest_path: None,
                    target: "thumbv7em-none-eabihf".to_owned(),
                    profile: "release".to_owned(),
                    features: None,
                },
                device: DeviceArgs {
                    device_name: Some("Floatwheel PintV".to_owned()),
                    address: None,
                },
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
