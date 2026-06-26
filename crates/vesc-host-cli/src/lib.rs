#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Layout,
    Status,
    Scan,
    Loopback,
    LispProbe(LispProbeCommand),
    PackageInstall(PackageInstallCommand),
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
        Some("loopback") => Ok(Command::Loopback),
        Some("lisp-probe") => parse_lisp_probe(iter).map(Command::LispProbe),
        Some("package-install") => parse_package_install(iter).map(Command::PackageInstall),
        Some(other) => Err(ParseError::UnknownCommand(other.to_owned())),
    }
}

fn parse_lisp_probe(
    mut iter: impl Iterator<Item = String>,
) -> Result<LispProbeCommand, ParseError> {
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

mod ble_discovery;
pub mod btle;
pub mod loopback;
pub mod package_install;
pub mod package_transport;
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{parse_args, Command, LispProbeCommand, PackageInstallCommand, ParseError};
    use vesc_protocol::{WireCommand, WireVersion};

    #[test]
    fn parses_layout_command() {
        assert_eq!(parse_args(["vesc-host-cli", "layout"]), Ok(Command::Layout));
    }

    #[test]
    fn parses_status_command() {
        assert_eq!(parse_args(["vesc-host-cli", "status"]), Ok(Command::Status));
    }

    #[test]
    fn parses_scan_command() {
        assert_eq!(parse_args(["vesc-host-cli", "scan"]), Ok(Command::Scan));
    }

    #[test]
    fn defaults_to_help() {
        assert_eq!(parse_args(["vesc-host-cli"]), Ok(Command::Help));
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            parse_args(["vesc-host-cli", "spoon"]),
            Err(ParseError::UnknownCommand("spoon".to_owned()))
        );
    }

    #[test]
    fn parses_loopback_command() {
        assert_eq!(
            parse_args(["vesc-host-cli", "loopback"]),
            Ok(Command::Loopback)
        );
    }

    #[test]
    fn parses_lisp_probe_command() {
        assert_eq!(
            parse_args(["vesc-host-cli", "lisp-probe"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: None,
            }))
        );
    }

    #[test]
    fn parses_lisp_probe_device_selector() {
        assert_eq!(
            parse_args(["vesc-host-cli", "lisp-probe", "--device", "VESC BLE UART"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            }))
        );
    }

    #[test]
    fn parses_lisp_probe_address_selector() {
        assert_eq!(
            parse_args([
                "vesc-host-cli",
                "lisp-probe",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
    }

    #[test]
    fn parses_package_install_command() {
        assert_eq!(
            parse_args(["vesc-host-cli", "package-install", "foo.vescpkg"]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: None,
            }))
        );
    }

    #[test]
    fn parses_package_install_device_selector() {
        assert_eq!(
            parse_args([
                "vesc-host-cli",
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
    }

    #[test]
    fn parses_package_install_address_selector() {
        assert_eq!(
            parse_args([
                "vesc-host-cli",
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
    }

    #[test]
    fn shares_the_protocol_crate_version_and_command_codes() {
        assert_eq!(WireVersion::CURRENT.raw(), 1);
        assert_eq!(WireCommand::Status.code(), 3);
    }
}
