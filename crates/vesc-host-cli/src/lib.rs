#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Layout,
    Status,
    Loopback,
    PackageInstall(String),
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
        Some("loopback") => Ok(Command::Loopback),
        Some("package-install") => iter
            .next()
            .map(Command::PackageInstall)
            .ok_or_else(|| ParseError::UnknownCommand("package-install".to_owned())),
        Some(other) => Err(ParseError::UnknownCommand(other.to_owned())),
    }
}

pub mod btle;
pub mod loopback;
pub mod package_install;

#[cfg(test)]
mod tests {
    use super::{parse_args, Command, ParseError};
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
    fn parses_package_install_command() {
        assert_eq!(
            parse_args(["vesc-host-cli", "package-install", "foo.vescpkg"]),
            Ok(Command::PackageInstall("foo.vescpkg".to_owned()))
        );
    }

    #[test]
    fn shares_the_protocol_crate_version_and_command_codes() {
        assert_eq!(WireVersion::CURRENT.raw(), 1);
        assert_eq!(WireCommand::Status.code(), 3);
    }
}
