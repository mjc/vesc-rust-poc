//! Command-line tool for building, installing, and debugging Rust VESC packages.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Layout,
    Status,
    Scan,
    Loopback(LoopbackCommand),
    LispProbe(LispProbeCommand),
    PackageInstall(PackageInstallCommand),
    ErasePackage(PackageEraseCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackCommand {
    pub device_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProbeCommand {
    pub device_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInstallCommand {
    pub package_path: String,
    pub device_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEraseCommand {
    pub device_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnknownCommand(String),
}

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

mod ble_discovery;

pub mod btle;
pub mod loopback;
pub mod package_install;
pub mod package_transport;
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{
        Command, LispProbeCommand, LoopbackCommand, PackageEraseCommand, PackageInstallCommand,
        ParseError, parse_args,
    };
    use vesc_protocol::{WireCommand, WireVersion};

    #[test]
    fn parse_args_covers_cli_commands() {
        assert_eq!(parse_args(["vesc-cli", "layout"]), Ok(Command::Layout));
        assert_eq!(parse_args(["vesc-cli", "status"]), Ok(Command::Status));
        assert_eq!(parse_args(["vesc-cli", "scan"]), Ok(Command::Scan));
        assert_eq!(parse_args(["vesc-cli"]), Ok(Command::Help));
        assert_eq!(
            parse_args(["vesc-cli", "spoon"]),
            Err(ParseError::UnknownCommand("spoon".to_owned()))
        );
        assert_eq!(
            parse_args(["vesc-cli", "loopback"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "loopback", "--device", "Floatwheel PintV"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "lisp-probe"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "lisp-probe", "--device", "VESC BLE UART"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "lisp-probe", "--address", "AA:BB:CC:DD:EE:FF"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "package-install", "foo.vescpkg"]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "vesc-cli",
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
                "vesc-cli",
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
            parse_args(["vesc-cli", "erase-package"]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["vesc-cli", "erase-package", "--device", "Floatwheel PintV"]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "vesc-cli",
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
            parse_args(["vesc-cli", "erase"]),
            Err(ParseError::UnknownCommand("erase".to_owned()))
        );
        assert_eq!(
            parse_args(["vesc-cli", "erase-package", "--force"]),
            Err(ParseError::UnknownCommand("--force".to_owned()))
        );
        assert_eq!(
            parse_args(["vesc-cli", "erase-package", "--device"]),
            Err(ParseError::UnknownCommand("--device".to_owned()))
        );
        assert_eq!(
            parse_args(["vesc-cli", "erase-package", "--address"]),
            Err(ParseError::UnknownCommand("--address".to_owned()))
        );
        assert_eq!(
            parse_args(["vesc-cli", "erase-package", "extra.vescpkg"]),
            Err(ParseError::UnknownCommand("extra.vescpkg".to_owned()))
        );
        assert_eq!(
            parse_args([
                "vesc-cli",
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

        assert_eq!(WireVersion::CURRENT.raw(), 1);
        assert_eq!(WireCommand::Status.code(), 3);
    }
}
