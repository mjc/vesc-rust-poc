use std::fs;
use std::path::PathBuf;

use crate::{
    BLE_LOOPBACK_PACKAGE_NAME, PackageBinaryConversionRunner, PackageTargetError,
    PackageTargetMode, PackageTargetPlan,
};

pub const DEFAULT_PACKAGE_VERSION: &str = "0.1.0";
pub const DEFAULT_TARGET_TRIPLE: &str = "thumbv7em-none-eabihf";
pub const BUILD_SUBCOMMAND: &str = "build";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoVescPkgMode {
    Build,
    BuildPackageOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoVescPkgInvocation {
    mode: CargoVescPkgMode,
    package_version: String,
    target_triple: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoVescPkgParseError {
    MissingSubcommand,
    UnexpectedSubcommand(String),
    MissingTargetValue,
    UnexpectedArgument(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoVescPkgError {
    Parse(CargoVescPkgParseError),
    Package(PackageTargetError),
}

impl std::fmt::Display for CargoVescPkgParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSubcommand => f.write_str("missing cargo vescpkg subcommand"),
            Self::UnexpectedSubcommand(subcommand) => {
                write!(f, "unexpected cargo vescpkg subcommand: {subcommand}")
            }
            Self::MissingTargetValue => f.write_str("missing value for --target"),
            Self::UnexpectedArgument(argument) => {
                write!(f, "unexpected cargo vescpkg argument: {argument}")
            }
        }
    }
}

impl std::error::Error for CargoVescPkgParseError {}

impl std::fmt::Display for CargoVescPkgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Package(error) => write!(f, "{error:?}"),
        }
    }
}

impl std::error::Error for CargoVescPkgError {}

impl CargoVescPkgInvocation {
    pub fn new(mode: CargoVescPkgMode) -> Self {
        Self {
            mode,
            package_version: DEFAULT_PACKAGE_VERSION.to_owned(),
            target_triple: DEFAULT_TARGET_TRIPLE.to_owned(),
        }
    }

    pub fn with_package_version(mut self, package_version: impl Into<String>) -> Self {
        self.package_version = package_version.into();
        self
    }

    pub fn with_target_triple(mut self, target_triple: impl Into<String>) -> Self {
        self.target_triple = target_triple.into();
        self
    }

    pub fn mode(&self) -> CargoVescPkgMode {
        self.mode
    }

    pub fn package_version(&self) -> &str {
        &self.package_version
    }

    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    pub fn subcommand_args(&self) -> Vec<String> {
        let mut args = vec![BUILD_SUBCOMMAND.to_owned()];
        if matches!(self.mode, CargoVescPkgMode::BuildPackageOnly) {
            args.push("--package-only".to_owned());
        }
        args.push("--target".to_owned());
        args.push(self.target_triple.clone());
        args
    }

    pub fn package_target_mode(&self) -> PackageTargetMode {
        match self.mode {
            CargoVescPkgMode::Build => PackageTargetMode::Package,
            CargoVescPkgMode::BuildPackageOnly => PackageTargetMode::PackageOnly,
        }
    }

    pub fn package_target_plan(&self, repo_root: impl Into<PathBuf>) -> PackageTargetPlan {
        PackageTargetPlan::new(
            repo_root,
            BLE_LOOPBACK_PACKAGE_NAME,
            self.package_version.clone(),
            self.package_target_mode(),
        )
    }

    pub fn execute_with<C>(
        &self,
        repo_root: impl Into<PathBuf>,
        conversion_runner: &C,
    ) -> Result<PathBuf, CargoVescPkgError>
    where
        C: PackageBinaryConversionRunner,
    {
        self.package_target_plan(repo_root)
            .execute_with(conversion_runner)
            .map_err(CargoVescPkgError::Package)
    }
}

pub fn parse_args<I, S>(args: I) -> Result<CargoVescPkgInvocation, CargoVescPkgParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let Some(subcommand) = args.next() else {
        return Err(CargoVescPkgParseError::MissingSubcommand);
    };

    if subcommand.as_ref() != BUILD_SUBCOMMAND {
        return Err(CargoVescPkgParseError::UnexpectedSubcommand(
            subcommand.as_ref().to_owned(),
        ));
    }

    let mut mode = CargoVescPkgMode::Build;
    let mut target_triple = DEFAULT_TARGET_TRIPLE.to_owned();
    while let Some(argument) = args.next() {
        match argument.as_ref() {
            "--package-only" => mode = CargoVescPkgMode::BuildPackageOnly,
            "--target" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingTargetValue);
                };
                target_triple = value.as_ref().to_owned();
            }
            other if other.starts_with('-') => {
                return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned()));
            }
            other => return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned())),
        }
    }

    Ok(CargoVescPkgInvocation::new(mode).with_target_triple(target_triple))
}

pub fn run_with<I, S, C>(
    repo_root: impl Into<PathBuf>,
    args: I,
    conversion_runner: &C,
) -> Result<PathBuf, CargoVescPkgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    C: PackageBinaryConversionRunner,
{
    let invocation = parse_args(args).map_err(CargoVescPkgError::Parse)?;
    invocation.execute_with(repo_root, conversion_runner)
}

pub fn command_design_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/cargo-vescpkg-command.md")
}

pub fn command_design_text() -> String {
    fs::read_to_string(command_design_path()).expect("cargo vescpkg command design")
}

#[cfg(test)]
mod tests {
    use crate::rust_package_api_roadmap::markdown_section_body;

    use super::{
        CargoVescPkgError, CargoVescPkgInvocation, CargoVescPkgMode, DEFAULT_PACKAGE_VERSION,
        DEFAULT_TARGET_TRIPLE, command_design_text, parse_args, run_with,
    };
    use crate::PackageTargetMode;
    use crate::package_conversion::PackageBinaryConversionCommand;
    use crate::test_support::{FakeConversionRunner, PackageTestHarness};
    use std::path::PathBuf;

    #[test]
    fn command_design_mentions_the_expected_contract() {
        let text = command_design_text();

        for section in [
            "## Contract",
            "## Intended Shape",
            "## Responsibilities",
            "## Non-Goals",
            "## Notes",
        ] {
            assert!(
                text.contains(section),
                "command design document is missing required section: {section}"
            );
        }

        let contract = markdown_section_body(&text, "Contract").expect("contract section");
        assert!(contract.contains("crates/vesc-pkg"));
        assert!(contract.contains("device-side BTLE loopback package"));

        let shape = markdown_section_body(&text, "Intended Shape").expect("intended shape section");
        assert!(shape.contains("cargo vescpkg build"));
        assert!(shape.contains("thumbv7em-none-eabihf"));
    }

    #[test]
    fn parses_the_default_build_invocation() {
        let invocation = parse_args(["build"]).expect("parse build invocation");

        assert_eq!(invocation.mode(), CargoVescPkgMode::Build);
        assert_eq!(invocation.package_version(), DEFAULT_PACKAGE_VERSION);
        assert_eq!(invocation.target_triple(), DEFAULT_TARGET_TRIPLE);
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--target".to_owned(),
                DEFAULT_TARGET_TRIPLE.to_owned(),
            ]
        );
        assert_eq!(invocation.package_target_mode(), PackageTargetMode::Package);
    }

    #[test]
    fn parses_the_package_only_invocation() {
        let invocation = parse_args([
            "build",
            "--package-only",
            "--target",
            "thumbv7em-none-eabihf",
        ])
        .expect("parse package-only invocation");

        assert_eq!(invocation.mode(), CargoVescPkgMode::BuildPackageOnly);
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--package-only".to_owned(),
                "--target".to_owned(),
                "thumbv7em-none-eabihf".to_owned(),
            ]
        );
        assert_eq!(
            PathBuf::from("/tmp/repo").join(
                invocation
                    .package_target_plan("/tmp/repo")
                    .package_output_path()
            ),
            PathBuf::from(
                "/tmp/repo/target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            invocation
                .package_target_plan("/tmp/repo")
                .package_output_path(),
            PathBuf::from(
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
    }

    #[test]
    fn package_target_plan_tracks_the_package_version_and_mode() {
        let invocation = CargoVescPkgInvocation::new(CargoVescPkgMode::Build)
            .with_package_version("0.1.0")
            .with_target_triple("thumbv7em-none-eabihf");

        let plan = invocation.package_target_plan("/workspace");

        assert_eq!(plan.mode(), PackageTargetMode::Package);
        assert_eq!(
            PathBuf::from("/workspace").join(plan.package_output_path()),
            PathBuf::from(
                "/workspace/target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            plan.package_output_path(),
            PathBuf::from(
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
    }

    #[test]
    fn parse_args_requires_the_build_subcommand() {
        assert!(matches!(
            parse_args(std::iter::empty::<&str>()),
            Err(super::CargoVescPkgParseError::MissingSubcommand)
        ));
    }

    #[test]
    fn run_with_executes_build_invocations() {
        let harness = PackageTestHarness::new();
        let root = harness.root().to_path_buf();
        let runner = FakeConversionRunner::materializing();

        let output = run_with(&root, ["build"], &runner).expect("run build invocation");
        assert_eq!(
            output,
            PathBuf::from(
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            runner.calls(),
            vec![PackageBinaryConversionCommand::new(
                root.join("scripts/conv.py"),
                root.join("target/native-lib-baseline/native_lib.bin"),
                root.join("target/native-lib-baseline/package_lib.bin"),
            )]
        );
        assert!(
            root.join(&output).exists(),
            "expected cargo vescpkg output to exist"
        );

        let package_only = run_with(&root, ["build", "--package-only"], &runner)
            .expect("run package-only invocation");
        assert_eq!(output, package_only);
        assert_eq!(runner.calls().len(), 2);
        assert!(root.join(&package_only).exists());
    }

    #[test]
    fn run_with_rejects_unknown_subcommands() {
        let harness = PackageTestHarness::new();
        let error = run_with(
            harness.root(),
            ["spoon"],
            &FakeConversionRunner::recording(),
        )
        .expect_err("unknown subcommand should fail");

        assert!(matches!(
            error,
            CargoVescPkgError::Parse(super::CargoVescPkgParseError::UnexpectedSubcommand(_))
        ));
    }
}
