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
    /// Install a package on a target device.
    PackageInstall(PackageInstallCommand),
    /// Erase an installed package from a target device.
    ErasePackage(PackageEraseCommand),
    /// Build, install, and smoke-test a package on a target device.
    Deploy(PackageInstallCommand),
    /// Run the terminal Snake example surface.
    Snake(SnakeCommand),
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
}

/// Arguments for the `snake` example command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
    /// Board width in cells.
    pub board_width: snake::SnakeBoardWidth,
    /// Board height in cells.
    pub board_height: snake::SnakeBoardHeight,
    /// Deterministic seed used by scripted and fake sessions.
    pub seed: snake::SnakeSeed,
    /// Scripted WASD-style direction input for CLI-only sessions.
    pub moves: Vec<snake::SnakeDirection>,
    /// Scripted local-play input actions for CLI-only sessions.
    pub actions: Vec<snake::SnakeLocalAction>,
    /// Number of ticks to render after the initial frame.
    pub tick_limit: snake::SnakeTickLimit,
}

/// Errors returned while parsing CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first non-program argument did not match a supported command.
    UnknownCommand(String),
}

/// Run a parsed CLI invocation and return the process exit code.
pub fn run_args<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match parse_args(args) {
        Ok(Command::Help) => {
            println!(
                "cargo vescpkg: use `build`, `layout`, `status`, `scan`, `loopback`, `lisp-probe`, `deploy`, `package-install`, `erase-package`, or `snake`"
            );
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
        Ok(Command::Snake(command)) => run_snake(command),
        Err(ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}

fn run_snake(command: SnakeCommand) -> ExitCode {
    let Some(board) =
        snake::SnakeBoardSize::new(command.board_width.get(), command.board_height.get())
    else {
        eprintln!("snake failed: invalid board size");
        return ExitCode::from(2);
    };
    let model = snake::SnakeModel::new(board, command.seed.get());
    let mut model = model;
    let rendered = if command.actions.is_empty() {
        snake::render_scripted_terminal_session(&mut model, command.moves, command.tick_limit)
    } else {
        snake::render_local_terminal_session(&mut model, command.actions, command.tick_limit)
            .map(|report| report.output().to_owned())
    };

    match rendered {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("snake failed: {error:?}");
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

fn run_package_install(command: PackageInstallCommand) -> ExitCode {
    let package = match package_install::read_package_from_path(&command.package_path) {
        Ok(package) => package,
        Err(error) => {
            eprintln!("failed to read package {}: {error}", command.package_path);
            return ExitCode::from(1);
        }
    };

    let transport = match package_transport::BtlePackageInstallTransport::new() {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("failed to initialize package transport: {error}");
            return ExitCode::from(1);
        }
    };

    if let Err(error) = transport.open(loopback_target(command.address, command.device_name)) {
        eprintln!("failed to open package transport: {error}");
        return ExitCode::from(1);
    }

    match package_install::install_package(&package, &transport) {
        Ok(report) => {
            transport.close();
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
    let transport = match package_transport::BtlePackageInstallTransport::new() {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("failed to initialize package transport: {error}");
            return ExitCode::from(1);
        }
    };

    if let Err(error) = transport.open(loopback_target(command.address, command.device_name)) {
        eprintln!("failed to open package transport: {error}");
        return ExitCode::from(1);
    }

    match package_install::erase_package(&transport) {
        Ok(report) => {
            transport.close();
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
        Some("package-install") => parse_package_install(iter).map(Command::PackageInstall),
        Some("deploy") => parse_package_install(iter).map(Command::Deploy),
        Some("erase-package") => parse_erase_package(iter).map(Command::ErasePackage),
        Some("snake") => parse_snake(iter).map(Command::Snake),
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

fn parse_package_install(
    mut iter: impl Iterator<Item = String>,
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
        .ok_or_else(|| ParseError::UnknownCommand("package-install".to_owned()))
}

fn parse_erase_package(
    mut iter: impl Iterator<Item = String>,
) -> Result<PackageEraseCommand, ParseError> {
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

    Ok(PackageEraseCommand {
        device_name,
        address,
    })
}

fn parse_snake(mut iter: impl Iterator<Item = String>) -> Result<SnakeCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut board_width = snake::SnakeBoardWidth::new(16).expect("default width");
    let mut board_height = snake::SnakeBoardHeight::new(12).expect("default height");
    let mut seed = snake::SnakeSeed::new(1);
    let mut moves = Vec::new();
    let mut actions = Vec::new();
    let mut tick_limit = snake::SnakeTickLimit::new(1).expect("default tick limit");

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
            "--board" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--board".to_owned()))?;
                let (width, height) = parse_snake_board(&value)?;
                board_width = width;
                board_height = height;
            }
            "--seed" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--seed".to_owned()))?;
                seed = parse_snake_seed(&value)?;
            }
            "--moves" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--moves".to_owned()))?;
                moves = parse_snake_moves(&value)?;
            }
            "--actions" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--actions".to_owned()))?;
                actions = parse_snake_actions(&value)?;
            }
            "--ticks" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--ticks".to_owned()))?;
                tick_limit = parse_snake_tick_limit(&value)?;
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok(SnakeCommand {
        device_name,
        address,
        board_width,
        board_height,
        seed,
        moves,
        actions,
        tick_limit,
    })
}

fn parse_snake_board(
    value: &str,
) -> Result<(snake::SnakeBoardWidth, snake::SnakeBoardHeight), ParseError> {
    let Some((width, height)) = value.split_once('x') else {
        return Err(ParseError::UnknownCommand(value.to_owned()));
    };
    let width = width
        .parse::<u8>()
        .ok()
        .and_then(snake::SnakeBoardWidth::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))?;
    let height = height
        .parse::<u8>()
        .ok()
        .and_then(snake::SnakeBoardHeight::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))?;

    Ok((width, height))
}

fn parse_snake_seed(value: &str) -> Result<snake::SnakeSeed, ParseError> {
    value
        .parse::<u32>()
        .map(snake::SnakeSeed::new)
        .map_err(|_| ParseError::UnknownCommand(value.to_owned()))
}

fn parse_snake_moves(value: &str) -> Result<Vec<snake::SnakeDirection>, ParseError> {
    value
        .chars()
        .map(|ch| match ch {
            'w' | 'W' => Ok(snake::SnakeDirection::Up),
            'a' | 'A' => Ok(snake::SnakeDirection::Left),
            's' | 'S' => Ok(snake::SnakeDirection::Down),
            'd' | 'D' => Ok(snake::SnakeDirection::Right),
            _ => Err(ParseError::UnknownCommand(value.to_owned())),
        })
        .collect()
}

fn parse_snake_actions(value: &str) -> Result<Vec<snake::SnakeLocalAction>, ParseError> {
    value
        .chars()
        .map(|ch| match ch {
            'w' | 'W' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Up)),
            'a' | 'A' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Left)),
            's' | 'S' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Down)),
            'd' | 'D' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Right)),
            '.' => Ok(snake::SnakeLocalAction::Tick),
            'p' | 'P' => Ok(snake::SnakeLocalAction::Pause),
            'u' | 'U' => Ok(snake::SnakeLocalAction::Resume),
            'r' | 'R' => Ok(snake::SnakeLocalAction::Reset),
            'q' | 'Q' => Ok(snake::SnakeLocalAction::Quit),
            _ => Err(ParseError::UnknownCommand(value.to_owned())),
        })
        .collect()
}

fn parse_snake_tick_limit(value: &str) -> Result<snake::SnakeTickLimit, ParseError> {
    value
        .parse::<u16>()
        .ok()
        .and_then(snake::SnakeTickLimit::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))
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
/// Deterministic snake game model for host-side tests and future UI wiring.
pub mod snake;
/// VESC UART packet encoding, decoding, and checksum helpers.
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{
        Command, LispProbeCommand, LoopbackCommand, PackageEraseCommand, PackageInstallCommand,
        ParseError, SnakeCommand, parse_args,
    };
    use crate::snake::{
        SnakeBoardHeight, SnakeBoardWidth, SnakeDirection, SnakeLocalAction, SnakeSeed,
        SnakeTickLimit,
    };
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
            parse_args(["cargo-vescpkg", "loopback"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "loopback", "--device", "Floatwheel PintV"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "lisp-probe"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "lisp-probe", "--device", "VESC BLE UART"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "lisp-probe",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
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
            parse_args([
                "cargo-vescpkg",
                "package-install",
                "--device",
                "Floatwheel PintV",
                "foo.vescpkg"
            ]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "package-install",
                "--address",
                "AA:BB:CC:DD:EE:FF",
                "foo.vescpkg"
            ]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package"]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--device",
                "Floatwheel PintV"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase"]),
            Err(ParseError::UnknownCommand("erase".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--force"]),
            Err(ParseError::UnknownCommand("--force".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--device"]),
            Err(ParseError::UnknownCommand("--device".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--address"]),
            Err(ParseError::UnknownCommand("--address".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "extra.vescpkg"]),
            Err(ParseError::UnknownCommand("extra.vescpkg".to_owned()))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "snake",
                "--device",
                "VESC BLE UART",
                "--board",
                "12x8",
                "--seed",
                "99",
                "--moves",
                "sa",
                "--ticks",
                "2"
            ]),
            Ok(Command::Snake(SnakeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
                board_width: SnakeBoardWidth::new(12).expect("width"),
                board_height: SnakeBoardHeight::new(8).expect("height"),
                seed: SnakeSeed::new(99),
                moves: vec![SnakeDirection::Down, SnakeDirection::Left],
                actions: Vec::new(),
                tick_limit: SnakeTickLimit::new(2).expect("tick limit"),
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "snake",
                "--board",
                "12x8",
                "--actions",
                "s.pu.rq",
                "--ticks",
                "12"
            ]),
            Ok(Command::Snake(SnakeCommand {
                device_name: None,
                address: None,
                board_width: SnakeBoardWidth::new(12).expect("width"),
                board_height: SnakeBoardHeight::new(8).expect("height"),
                seed: SnakeSeed::new(1),
                moves: Vec::new(),
                actions: vec![
                    SnakeLocalAction::Turn(SnakeDirection::Down),
                    SnakeLocalAction::Tick,
                    SnakeLocalAction::Pause,
                    SnakeLocalAction::Resume,
                    SnakeLocalAction::Tick,
                    SnakeLocalAction::Reset,
                    SnakeLocalAction::Quit,
                ],
                tick_limit: SnakeTickLimit::new(12).expect("tick limit"),
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--device",
                "Floatwheel PintV",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );

        assert_eq!(WireVersion::CURRENT.get(), 1);
        assert_eq!(u8::from(WireCommand::Status), 3);
    }
}
