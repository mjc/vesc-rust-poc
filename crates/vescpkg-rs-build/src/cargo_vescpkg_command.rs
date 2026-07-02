use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BLE_LOOPBACK_PACKAGE_NAME, Package, PackageBinaryConversionRunner, PackageExample,
    PackageTargetError, PackageTargetMode, PackageTargetPlan, SNAKE_PACKAGE_NAME,
};

/// Default package version used by the cargo subcommand wrapper.
pub const DEFAULT_PACKAGE_VERSION: &str = "0.1.0";
/// Default embedded target triple used by the cargo subcommand wrapper.
pub const DEFAULT_TARGET_TRIPLE: &str = "thumbv7em-none-eabihf";
/// Cargo subcommand name accepted by this parser.
pub const BUILD_SUBCOMMAND: &str = "build";

/// Supported `cargo vescpkg` modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoVescPkgMode {
    /// Build the full package target.
    Build,
    /// Only build the package payload and staging output.
    BuildPackageOnly,
}

/// Example package selected by `cargo vescpkg build`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoVescPkgExample {
    /// Existing BLE loopback package example.
    Loopback,
    /// Snake package example.
    Snake,
}

/// Parsed `cargo vescpkg` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoVescPkgInvocation {
    mode: CargoVescPkgMode,
    example: CargoVescPkgExample,
    package_version: String,
    target_triple: String,
    manifest_path: Option<PathBuf>,
}

/// Errors produced while parsing `cargo vescpkg` arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoVescPkgParseError {
    /// No subcommand was provided.
    MissingSubcommand,
    /// The first positional argument was not a supported subcommand.
    UnexpectedSubcommand(String),
    /// `--target` was provided without a value.
    MissingTargetValue,
    /// `--example` was provided without a value.
    MissingExampleValue,
    /// `--manifest` was provided without a value.
    MissingManifestValue,
    /// `--example` selected an unsupported package example.
    UnsupportedExample(String),
    /// An unsupported flag or positional argument was provided.
    UnexpectedArgument(String),
}

/// Top-level errors returned by the `cargo vescpkg` runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoVescPkgError {
    /// Argument parsing failed.
    Parse(CargoVescPkgParseError),
    /// Package execution failed after parsing.
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
            Self::MissingExampleValue => f.write_str("missing value for --example"),
            Self::MissingManifestValue => f.write_str("missing value for --manifest"),
            Self::UnsupportedExample(example) => {
                write!(f, "unsupported cargo vescpkg example: {example}")
            }
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
    /// Construct a new invocation with default version and target values.
    pub fn new(mode: CargoVescPkgMode) -> Self {
        Self {
            mode,
            example: CargoVescPkgExample::Loopback,
            package_version: DEFAULT_PACKAGE_VERSION.to_owned(),
            target_triple: DEFAULT_TARGET_TRIPLE.to_owned(),
            manifest_path: None,
        }
    }

    /// Override the selected package example.
    pub fn with_example(mut self, example: CargoVescPkgExample) -> Self {
        self.example = example;
        self
    }

    /// Override the package version used by the invocation.
    pub fn with_package_version(mut self, package_version: impl Into<String>) -> Self {
        self.package_version = package_version.into();
        self
    }

    /// Override the target triple used by the invocation.
    pub fn with_target_triple(mut self, target_triple: impl Into<String>) -> Self {
        self.target_triple = target_triple.into();
        self
    }

    /// Use an existing package descriptor instead of rendering example staging assets.
    pub fn with_manifest_path(mut self, manifest_path: impl Into<PathBuf>) -> Self {
        self.mode = CargoVescPkgMode::BuildPackageOnly;
        self.manifest_path = Some(manifest_path.into());
        self
    }

    /// Return the requested invocation mode.
    pub fn mode(&self) -> CargoVescPkgMode {
        self.mode
    }

    /// Return the selected package example.
    pub fn example(&self) -> CargoVescPkgExample {
        self.example
    }

    /// Return the package version.
    pub fn package_version(&self) -> &str {
        &self.package_version
    }

    /// Return the target triple.
    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    /// Return the package descriptor path when this invocation builds from one.
    pub fn manifest_path(&self) -> Option<&Path> {
        self.manifest_path.as_deref()
    }

    /// Return the package name associated with the selected example.
    pub fn package_name(&self) -> &'static str {
        match self.example {
            CargoVescPkgExample::Loopback => BLE_LOOPBACK_PACKAGE_NAME,
            CargoVescPkgExample::Snake => SNAKE_PACKAGE_NAME,
        }
    }

    /// Return the build-layer package example associated with this invocation.
    pub fn package_example(&self) -> PackageExample {
        match self.example {
            CargoVescPkgExample::Loopback => PackageExample::Loopback,
            CargoVescPkgExample::Snake => PackageExample::Snake,
        }
    }

    /// Render the cargo subcommand arguments implied by this invocation.
    pub fn subcommand_args(&self) -> Vec<String> {
        let mut args = vec![BUILD_SUBCOMMAND.to_owned()];
        if matches!(self.mode, CargoVescPkgMode::BuildPackageOnly) {
            args.push("--package-only".to_owned());
        }
        if !matches!(self.example, CargoVescPkgExample::Loopback) {
            args.push("--example".to_owned());
            args.push(
                match self.example {
                    CargoVescPkgExample::Loopback => "loopback",
                    CargoVescPkgExample::Snake => "snake",
                }
                .to_owned(),
            );
        }
        if let Some(manifest_path) = &self.manifest_path {
            args.push("--manifest".to_owned());
            args.push(manifest_path.display().to_string());
        }
        args.push("--target".to_owned());
        args.push(self.target_triple.clone());
        args
    }

    /// Translate the invocation mode into a package target mode.
    pub fn package_target_mode(&self) -> PackageTargetMode {
        match self.mode {
            CargoVescPkgMode::Build => PackageTargetMode::Package,
            CargoVescPkgMode::BuildPackageOnly => PackageTargetMode::PackageOnly,
        }
    }

    /// Build the package target plan for this invocation.
    pub fn package_target_plan(&self, repo_root: impl Into<PathBuf>) -> PackageTargetPlan {
        PackageTargetPlan::for_example(
            repo_root,
            self.package_name(),
            self.package_version.clone(),
            self.package_example(),
            self.package_target_mode(),
        )
    }

    /// Execute the invocation with a custom conversion runner.
    pub fn execute_with<C>(
        &self,
        repo_root: impl Into<PathBuf>,
        conversion_runner: &C,
    ) -> Result<PathBuf, CargoVescPkgError>
    where
        C: PackageBinaryConversionRunner,
    {
        if let Some(manifest_path) = &self.manifest_path {
            let repo_root = repo_root.into();
            let output =
                Package::write_from_manifest(repo_root.join(manifest_path)).map_err(|error| {
                    CargoVescPkgError::Package(PackageTargetError::PackageOutput {
                        path: manifest_path.clone(),
                        reason: error.to_string(),
                    })
                })?;
            return Ok(output
                .strip_prefix(&repo_root)
                .unwrap_or(&output)
                .to_path_buf());
        }

        self.package_target_plan(repo_root)
            .execute_with(conversion_runner)
            .map_err(CargoVescPkgError::Package)
    }
}

/// Parse the argument tail passed to `cargo vescpkg`.
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
    let mut example = CargoVescPkgExample::Loopback;
    let mut target_triple = DEFAULT_TARGET_TRIPLE.to_owned();
    let mut manifest_path = None;
    while let Some(argument) = args.next() {
        match argument.as_ref() {
            "--package-only" => mode = CargoVescPkgMode::BuildPackageOnly,
            "--example" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingExampleValue);
                };
                example = parse_example(value.as_ref())?;
            }
            "--target" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingTargetValue);
                };
                target_triple = value.as_ref().to_owned();
            }
            "--manifest" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingManifestValue);
                };
                mode = CargoVescPkgMode::BuildPackageOnly;
                manifest_path = Some(PathBuf::from(value.as_ref()));
            }
            other if other.starts_with('-') => {
                return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned()));
            }
            other => return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned())),
        }
    }

    let invocation = CargoVescPkgInvocation::new(mode)
        .with_example(example)
        .with_target_triple(target_triple);

    Ok(match manifest_path {
        Some(path) => invocation.with_manifest_path(path),
        None => invocation,
    })
}

fn parse_example(value: &str) -> Result<CargoVescPkgExample, CargoVescPkgParseError> {
    match value {
        "loopback" => Ok(CargoVescPkgExample::Loopback),
        "snake" => Ok(CargoVescPkgExample::Snake),
        other => Err(CargoVescPkgParseError::UnsupportedExample(other.to_owned())),
    }
}

/// Parse and execute a `cargo vescpkg` invocation with a custom runner.
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

/// Return the design-note path for the cargo subcommand contract.
pub fn command_design_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/cargo-vescpkg-command.md")
}

/// Read the cargo subcommand design note text.
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
        assert!(contract.contains("crates/vescpkg-rs-build"));
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
    fn parses_the_manifest_build_invocation() {
        let invocation =
            parse_args(["build", "--manifest", "refloat/pkgdesc.qml"]).expect("parse manifest");

        assert_eq!(invocation.mode(), CargoVescPkgMode::BuildPackageOnly);
        assert_eq!(
            invocation.manifest_path(),
            Some(PathBuf::from("refloat/pkgdesc.qml").as_path())
        );
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--package-only".to_owned(),
                "--manifest".to_owned(),
                "refloat/pkgdesc.qml".to_owned(),
                "--target".to_owned(),
                DEFAULT_TARGET_TRIPLE.to_owned(),
            ]
        );
    }

    #[test]
    fn parses_the_snake_example_invocation() {
        let invocation =
            parse_args(["build", "--example", "snake"]).expect("parse snake example invocation");

        assert_eq!(invocation.mode(), CargoVescPkgMode::Build);
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--example".to_owned(),
                "snake".to_owned(),
                "--target".to_owned(),
                DEFAULT_TARGET_TRIPLE.to_owned(),
            ]
        );
        let plan = invocation.package_target_plan("/tmp/repo");
        assert_eq!(
            plan.package_output_path(),
            PathBuf::from(
                "target/vescpkg/Rust-Snake-example-package-0.1.0/Rust-Snake-example-package-0.1.0.vescpkg"
            )
        );
        assert_eq!(
            plan.build_plan().native_artifact_input_path(),
            PathBuf::from("target/thumbv7em-none-eabihf/release/libvesc_example_snake.a")
        );
        assert_eq!(
            plan.build_plan().example_source_path(),
            PathBuf::from("examples/snake")
        );
        assert_eq!(
            plan.build_plan().conversion_plan().command().example(),
            crate::PackageExample::Snake
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
    fn run_with_writes_manifest_package_without_conversion_runner() {
        let harness = PackageTestHarness::new().ensure_loopback_staging();
        let root = harness.root().to_path_buf();
        let staging = harness.loopback_staging_dir();
        std::fs::create_dir_all(staging.join("lisp")).unwrap();
        std::fs::create_dir_all(staging.join("src")).unwrap();
        std::fs::write(staging.join("package_README-gen.md"), "Refloat readme").unwrap();
        std::fs::write(
            staging.join("lisp/package.lisp"),
            "(import \"src/package_lib.bin\" 'refloat-native)\n",
        )
        .unwrap();
        std::fs::write(staging.join("src/package_lib.bin"), b"refloat-native\0").unwrap();
        std::fs::write(staging.join("ui.qml"), "import QtQuick 2.15\nItem {}\n").unwrap();
        std::fs::write(
            staging.join("pkgdesc.qml"),
            "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: true\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
        )
        .unwrap();

        let runner = FakeConversionRunner::recording();
        let output = run_with(
            &root,
            [
                "build",
                "--manifest",
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/pkgdesc.qml",
            ],
            &runner,
        )
        .expect("run manifest package invocation");

        assert_eq!(
            output,
            PathBuf::from("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/refloat.vescpkg")
        );
        assert!(root.join(output).exists());
        assert!(runner.calls().is_empty());
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

    #[test]
    fn parse_args_rejects_unknown_examples() {
        assert_eq!(
            parse_args(["build", "--example"]),
            Err(super::CargoVescPkgParseError::MissingExampleValue)
        );
        assert_eq!(
            parse_args(["build", "--manifest"]),
            Err(super::CargoVescPkgParseError::MissingManifestValue)
        );
        assert_eq!(
            parse_args(["build", "--example", "pong"]),
            Err(super::CargoVescPkgParseError::UnsupportedExample(
                "pong".to_owned()
            ))
        );
    }
}
