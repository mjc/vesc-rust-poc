use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BLE_LOOPBACK_PACKAGE_NAME, Package, PackageBinaryConversionRunner, PackageExample,
    PackageTargetError, PackageTargetMode, PackageTargetPlan,
    native_lib_toolchain::{NativeLibToolchain, RealNativeLibToolchain},
    refloat_native_build::{RefloatGitHash, RefloatNativeBuildPlan},
    refloat_package_assets::{RefloatBuildInfo, RefloatSourceAssets},
};

/// Default package version used by the cargo subcommand wrapper.
pub const DEFAULT_PACKAGE_VERSION: &str = "0.1.0";
/// Default embedded target triple used by the cargo subcommand wrapper.
pub const DEFAULT_TARGET_TRIPLE: &str = "thumbv7em-none-eabihf";
/// Default Refloat build date used when no deterministic value is supplied.
pub const DEFAULT_REFLOAT_BUILD_DATE: &str = "unknown";
/// Default Refloat git commit used when no deterministic value is supplied.
pub const DEFAULT_REFLOAT_GIT_COMMIT: &str = "unknown";
/// Default VESC Tool executable used for Refloat native config generation.
pub const DEFAULT_REFLOAT_VESC_TOOL: &str = "vesc_tool";
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
}

/// Mutually exclusive source selected by `cargo vescpkg build`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoVescPkgSource {
    /// Render one of the built-in examples.
    Example(CargoVescPkgExample),
    /// Build from an existing package descriptor.
    Manifest(PathBuf),
    /// Build from a Refloat source checkout.
    RefloatSource(PathBuf),
}

/// Parsed `cargo vescpkg` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoVescPkgInvocation {
    mode: CargoVescPkgMode,
    source: CargoVescPkgSource,
    package_version: String,
    target_triple: String,
    refloat_build_date: String,
    refloat_git_commit: String,
    refloat_vesc_tool: String,
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
    /// `--refloat-source` was provided without a value.
    MissingRefloatSourceValue,
    /// `--build-date` was provided without a value.
    MissingBuildDateValue,
    /// `--git-commit` was provided without a value.
    MissingGitCommitValue,
    /// `--vesc-tool` was provided without a value.
    MissingVescToolValue,
    /// `--example` selected an unsupported package example.
    UnsupportedExample(String),
    /// `--git-commit` was not a bare hexadecimal commit prefix.
    InvalidRefloatGitCommit(String),
    /// More than one package source selector was provided.
    ConflictingPackageSources,
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
            Self::MissingRefloatSourceValue => f.write_str("missing value for --refloat-source"),
            Self::MissingBuildDateValue => f.write_str("missing value for --build-date"),
            Self::MissingGitCommitValue => f.write_str("missing value for --git-commit"),
            Self::MissingVescToolValue => f.write_str("missing value for --vesc-tool"),
            Self::UnsupportedExample(example) => {
                write!(f, "unsupported cargo vescpkg example: {example}")
            }
            Self::InvalidRefloatGitCommit(value) => {
                write!(
                    f,
                    "--git-commit must be hexadecimal digits without a 0x prefix: {value}"
                )
            }
            Self::ConflictingPackageSources => {
                f.write_str("choose only one of --example, --manifest, or --refloat-source")
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
            source: CargoVescPkgSource::Example(CargoVescPkgExample::Loopback),
            package_version: DEFAULT_PACKAGE_VERSION.to_owned(),
            target_triple: DEFAULT_TARGET_TRIPLE.to_owned(),
            refloat_build_date: DEFAULT_REFLOAT_BUILD_DATE.to_owned(),
            refloat_git_commit: DEFAULT_REFLOAT_GIT_COMMIT.to_owned(),
            refloat_vesc_tool: DEFAULT_REFLOAT_VESC_TOOL.to_owned(),
        }
    }

    /// Override the selected package example.
    pub fn with_example(mut self, example: CargoVescPkgExample) -> Self {
        self.source = CargoVescPkgSource::Example(example);
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
        self.source = CargoVescPkgSource::Manifest(manifest_path.into());
        self
    }

    /// Use a Refloat source tree instead of the built-in examples.
    pub fn with_refloat_source_path(mut self, refloat_source_path: impl Into<PathBuf>) -> Self {
        self.mode = CargoVescPkgMode::BuildPackageOnly;
        self.source = CargoVescPkgSource::RefloatSource(refloat_source_path.into());
        self
    }

    /// Use deterministic metadata for Refloat generated package assets.
    pub fn with_refloat_build_info(
        mut self,
        build_date: impl Into<String>,
        git_commit: impl Into<String>,
    ) -> Self {
        self.refloat_build_date = build_date.into();
        self.refloat_git_commit = git_commit.into();
        self
    }

    /// Set the VESC Tool executable used by Refloat's native Makefile.
    pub fn with_refloat_vesc_tool(mut self, vesc_tool: impl Into<String>) -> Self {
        self.refloat_vesc_tool = vesc_tool.into();
        self
    }

    /// Return the requested invocation mode.
    pub fn mode(&self) -> CargoVescPkgMode {
        self.mode
    }

    /// Return the selected package example.
    pub fn example(&self) -> CargoVescPkgExample {
        match &self.source {
            CargoVescPkgSource::Example(example) => *example,
            CargoVescPkgSource::Manifest(_) | CargoVescPkgSource::RefloatSource(_) => {
                CargoVescPkgExample::Loopback
            }
        }
    }

    /// Return the selected package source.
    pub fn source(&self) -> &CargoVescPkgSource {
        &self.source
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
        match &self.source {
            CargoVescPkgSource::Manifest(path) => Some(path),
            CargoVescPkgSource::Example(_) | CargoVescPkgSource::RefloatSource(_) => None,
        }
    }

    /// Return the Refloat source path when this invocation builds from one.
    pub fn refloat_source_path(&self) -> Option<&Path> {
        match &self.source {
            CargoVescPkgSource::RefloatSource(path) => Some(path),
            CargoVescPkgSource::Example(_) | CargoVescPkgSource::Manifest(_) => None,
        }
    }

    /// Return the deterministic Refloat build date string.
    pub fn refloat_build_date(&self) -> &str {
        &self.refloat_build_date
    }

    /// Return the deterministic Refloat git commit string.
    pub fn refloat_git_commit(&self) -> &str {
        &self.refloat_git_commit
    }

    /// Return the VESC Tool executable used by Refloat's native Makefile.
    pub fn refloat_vesc_tool(&self) -> &str {
        &self.refloat_vesc_tool
    }

    /// Return the package name associated with the selected example.
    pub fn package_name(&self) -> &'static str {
        BLE_LOOPBACK_PACKAGE_NAME
    }

    /// Return the build-layer package example associated with this invocation.
    pub fn package_example(&self) -> PackageExample {
        PackageExample::Loopback
    }

    /// Render the cargo subcommand arguments implied by this invocation.
    pub fn subcommand_args(&self) -> Vec<String> {
        let mut args = vec![BUILD_SUBCOMMAND.to_owned()];
        if matches!(self.mode, CargoVescPkgMode::BuildPackageOnly) {
            args.push("--package-only".to_owned());
        }
        match &self.source {
            CargoVescPkgSource::Example(_) => {}
            CargoVescPkgSource::Manifest(manifest_path) => {
                args.push("--manifest".to_owned());
                args.push(manifest_path.display().to_string());
            }
            CargoVescPkgSource::RefloatSource(refloat_source_path) => {
                args.push("--refloat-source".to_owned());
                args.push(refloat_source_path.display().to_string());
                args.push("--build-date".to_owned());
                args.push(self.refloat_build_date.clone());
                args.push("--git-commit".to_owned());
                args.push(self.refloat_git_commit.clone());
                args.push("--vesc-tool".to_owned());
                args.push(self.refloat_vesc_tool.clone());
            }
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
        self.execute_with_toolchains(repo_root, conversion_runner, &RealNativeLibToolchain)
    }

    /// Execute the invocation with custom conversion and native toolchains.
    pub fn execute_with_toolchains<C, N>(
        &self,
        repo_root: impl Into<PathBuf>,
        conversion_runner: &C,
        native_toolchain: &N,
    ) -> Result<PathBuf, CargoVescPkgError>
    where
        C: PackageBinaryConversionRunner,
        N: NativeLibToolchain,
    {
        if let CargoVescPkgSource::RefloatSource(refloat_source_path) = &self.source {
            let repo_root = repo_root.into();
            let refloat_source_root = clean_path(repo_root.join(refloat_source_path));
            let vesc_tool = resolve_refloat_vesc_tool(&repo_root, self.refloat_vesc_tool());
            RefloatNativeBuildPlan::new(&refloat_source_root)
                .with_vesc_tool(vesc_tool)
                .with_git_hash(refloat_native_git_hash(self.refloat_git_commit()))
                .map_err(|error| package_output_error(refloat_source_path, error))?
                .build_with(native_toolchain)
                .map_err(|error| package_output_error(refloat_source_path, error))?;
            let output = RefloatSourceAssets::new(refloat_source_root)
                .write_package(&RefloatBuildInfo::new(
                    self.refloat_build_date(),
                    self.refloat_git_commit(),
                ))
                .map_err(|error| package_output_error(refloat_source_path, error))?;
            return Ok(output
                .strip_prefix(&repo_root)
                .unwrap_or(&output)
                .to_path_buf());
        }

        if let CargoVescPkgSource::Manifest(manifest_path) = &self.source {
            let repo_root = repo_root.into();
            let output = Package::write_from_manifest(repo_root.join(manifest_path))
                .map_err(|error| package_output_error(manifest_path, error))?;
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

fn package_output_error(path: &Path, error: impl fmt::Display) -> CargoVescPkgError {
    CargoVescPkgError::Package(PackageTargetError::PackageOutput {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })
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
    let mut explicit_example = false;
    let mut target_triple = DEFAULT_TARGET_TRIPLE.to_owned();
    let mut manifest_path = None;
    let mut refloat_source_path = None;
    let mut refloat_build_date = DEFAULT_REFLOAT_BUILD_DATE.to_owned();
    let mut refloat_git_commit = DEFAULT_REFLOAT_GIT_COMMIT.to_owned();
    let mut refloat_vesc_tool = DEFAULT_REFLOAT_VESC_TOOL.to_owned();
    while let Some(argument) = args.next() {
        match argument.as_ref() {
            "--package-only" => mode = CargoVescPkgMode::BuildPackageOnly,
            "--example" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingExampleValue);
                };
                example = parse_example(value.as_ref())?;
                explicit_example = true;
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
            "--refloat-source" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingRefloatSourceValue);
                };
                mode = CargoVescPkgMode::BuildPackageOnly;
                refloat_source_path = Some(PathBuf::from(value.as_ref()));
            }
            "--build-date" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingBuildDateValue);
                };
                refloat_build_date = value.as_ref().to_owned();
            }
            "--git-commit" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingGitCommitValue);
                };
                refloat_git_commit = parse_refloat_git_commit(value.as_ref())?.to_owned();
            }
            "--vesc-tool" => {
                let Some(value) = args.next() else {
                    return Err(CargoVescPkgParseError::MissingVescToolValue);
                };
                refloat_vesc_tool = value.as_ref().to_owned();
            }
            other if other.starts_with('-') => {
                return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned()));
            }
            other => return Err(CargoVescPkgParseError::UnexpectedArgument(other.to_owned())),
        }
    }

    let package_source_count = usize::from(explicit_example)
        + usize::from(manifest_path.is_some())
        + usize::from(refloat_source_path.is_some());
    if package_source_count > 1 {
        return Err(CargoVescPkgParseError::ConflictingPackageSources);
    }

    let invocation = CargoVescPkgInvocation::new(mode)
        .with_example(example)
        .with_target_triple(target_triple)
        .with_refloat_build_info(refloat_build_date, refloat_git_commit)
        .with_refloat_vesc_tool(refloat_vesc_tool);

    let invocation = match manifest_path {
        Some(path) => invocation.with_manifest_path(path),
        None => invocation,
    };

    Ok(match refloat_source_path {
        Some(path) => invocation.with_refloat_source_path(path),
        None => invocation,
    })
}

fn parse_example(value: &str) -> Result<CargoVescPkgExample, CargoVescPkgParseError> {
    match value {
        "loopback" => Ok(CargoVescPkgExample::Loopback),
        other => Err(CargoVescPkgParseError::UnsupportedExample(other.to_owned())),
    }
}

fn clean_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().components().collect()
}

fn resolve_refloat_vesc_tool(repo_root: &Path, vesc_tool: &str) -> String {
    let path = Path::new(vesc_tool);
    if path.is_absolute() {
        return clean_path(path).display().to_string();
    }
    if is_path_like(path) {
        return clean_path(repo_root.join(path)).display().to_string();
    }
    vesc_tool.to_owned()
}

fn refloat_native_git_hash(git_commit: &str) -> &str {
    match git_commit {
        DEFAULT_REFLOAT_GIT_COMMIT => "0",
        value => value,
    }
}

fn parse_refloat_git_commit(value: &str) -> Result<&str, CargoVescPkgParseError> {
    if RefloatGitHash::is_valid(value) {
        Ok(value)
    } else {
        Err(CargoVescPkgParseError::InvalidRefloatGitCommit(
            value.to_owned(),
        ))
    }
}

fn is_path_like(path: &Path) -> bool {
    let mut components = path.components();
    match components.next() {
        Some(std::path::Component::CurDir | std::path::Component::ParentDir) => true,
        Some(_) => components.next().is_some(),
        None => false,
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

/// Parse and execute a `cargo vescpkg` invocation with custom toolchains.
pub fn run_with_toolchains<I, S, C, N>(
    repo_root: impl Into<PathBuf>,
    args: I,
    conversion_runner: &C,
    native_toolchain: &N,
) -> Result<PathBuf, CargoVescPkgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    C: PackageBinaryConversionRunner,
    N: NativeLibToolchain,
{
    let invocation = parse_args(args).map_err(CargoVescPkgError::Parse)?;
    invocation.execute_with_toolchains(repo_root, conversion_runner, native_toolchain)
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
        CargoVescPkgError, CargoVescPkgInvocation, CargoVescPkgMode, CargoVescPkgSource,
        DEFAULT_PACKAGE_VERSION, DEFAULT_REFLOAT_BUILD_DATE, DEFAULT_REFLOAT_GIT_COMMIT,
        DEFAULT_REFLOAT_VESC_TOOL, DEFAULT_TARGET_TRIPLE, command_design_text, parse_args,
        run_with, run_with_toolchains,
    };
    use crate::package_conversion::PackageBinaryConversionCommand;
    use crate::test_support::{FakeConversionRunner, PackageTestHarness};
    use crate::{PackageTargetMode, native_lib_toolchain::NativeLibToolchain, parse_lisp_imports};
    use std::cell::RefCell;
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
        assert!(shape.contains("--manifest"));
        assert!(shape.contains("--refloat-source"));
        assert!(shape.contains("--build-date"));
        assert!(shape.contains("--git-commit"));
        assert!(shape.contains("--vesc-tool"));
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
    fn parses_the_refloat_source_invocation() {
        let invocation = parse_args([
            "build",
            "--refloat-source",
            "refloat",
            "--build-date",
            "2026-07-02 06:00:00-06:00",
            "--git-commit",
            "0ef6e99",
            "--vesc-tool",
            "/opt/vesc_tool",
        ])
        .expect("parse refloat source invocation");

        assert_eq!(invocation.mode(), CargoVescPkgMode::BuildPackageOnly);
        assert_eq!(
            invocation.refloat_source_path(),
            Some(PathBuf::from("refloat").as_path())
        );
        assert_eq!(invocation.refloat_build_date(), "2026-07-02 06:00:00-06:00");
        assert_eq!(invocation.refloat_git_commit(), "0ef6e99");
        assert_eq!(invocation.refloat_vesc_tool(), "/opt/vesc_tool");
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--package-only".to_owned(),
                "--refloat-source".to_owned(),
                "refloat".to_owned(),
                "--build-date".to_owned(),
                "2026-07-02 06:00:00-06:00".to_owned(),
                "--git-commit".to_owned(),
                "0ef6e99".to_owned(),
                "--vesc-tool".to_owned(),
                "/opt/vesc_tool".to_owned(),
                "--target".to_owned(),
                DEFAULT_TARGET_TRIPLE.to_owned(),
            ]
        );
    }

    #[test]
    fn parse_args_rejects_refloat_git_commits_that_are_not_hex_digits() {
        for git_commit in ["feature/refloat", "0x0ef6e99", "0ef6e99-dirty", ""] {
            assert_eq!(
                parse_args([
                    "build",
                    "--refloat-source",
                    "refloat",
                    "--git-commit",
                    git_commit,
                ]),
                Err(super::CargoVescPkgParseError::InvalidRefloatGitCommit(
                    git_commit.to_owned()
                ))
            );
        }
    }

    #[test]
    fn parse_args_rejects_conflicting_package_sources() {
        for args in [
            [
                "build",
                "--manifest",
                "refloat/pkgdesc.qml",
                "--refloat-source",
                "refloat",
            ],
            [
                "build",
                "--example",
                "loopback",
                "--manifest",
                "refloat/pkgdesc.qml",
            ],
            [
                "build",
                "--example",
                "loopback",
                "--refloat-source",
                "refloat",
            ],
        ] {
            assert_eq!(
                parse_args(args),
                Err(super::CargoVescPkgParseError::ConflictingPackageSources)
            );
        }
    }

    #[test]
    fn builder_package_source_is_mutually_exclusive() {
        let invocation = CargoVescPkgInvocation::new(CargoVescPkgMode::Build)
            .with_manifest_path("refloat/pkgdesc.qml")
            .with_refloat_source_path("refloat");

        assert_eq!(
            invocation.source(),
            &CargoVescPkgSource::RefloatSource(PathBuf::from("refloat"))
        );
        assert_eq!(invocation.manifest_path(), None);
        assert_eq!(
            invocation.subcommand_args(),
            vec![
                "build".to_owned(),
                "--package-only".to_owned(),
                "--refloat-source".to_owned(),
                "refloat".to_owned(),
                "--build-date".to_owned(),
                DEFAULT_REFLOAT_BUILD_DATE.to_owned(),
                "--git-commit".to_owned(),
                DEFAULT_REFLOAT_GIT_COMMIT.to_owned(),
                "--vesc-tool".to_owned(),
                DEFAULT_REFLOAT_VESC_TOOL.to_owned(),
                "--target".to_owned(),
                DEFAULT_TARGET_TRIPLE.to_owned(),
            ]
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
    fn run_with_writes_refloat_source_package_without_conversion_runner() {
        let harness = write_refloat_source(PackageTestHarness::new());
        let root = harness.root().to_path_buf();

        let runner = FakeConversionRunner::recording();
        let native_toolchain = RefloatWritingNativeToolchain::new(&root);
        let output = run_with_toolchains(
            &root,
            [
                "build",
                "--refloat-source",
                ".",
                "--build-date",
                "2026-07-02 06:00:00-06:00",
                "--git-commit",
                "0ef6e99",
            ],
            &runner,
            &native_toolchain,
        )
        .expect("run refloat source invocation");

        assert_eq!(output, PathBuf::from("refloat.vescpkg"));
        let package = crate::Package::read(root.join(output)).expect("written package");
        assert_eq!(package.name, "Refloat");
        assert!(package.description_md.contains("- Git Commit: #0ef6e99"));
        assert_eq!(package.qml_file, "Item{property string title:\"Refloat\"}");
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn run_with_refloat_source_builds_native_payload_before_packaging() {
        let harness = write_refloat_source(PackageTestHarness::new());
        let root = harness.root().to_path_buf();
        std::fs::remove_file(root.join("src/package_lib.bin")).unwrap();

        let conversion_runner = FakeConversionRunner::recording();
        let native_toolchain = RefloatWritingNativeToolchain::new(&root);
        let output = run_with_toolchains(
            &root,
            [
                "build",
                "--refloat-source",
                ".",
                "--build-date",
                "2026-07-02 06:00:00-06:00",
                "--git-commit",
                "0ef6e99",
                "--vesc-tool",
                "custom-vesc-tool",
            ],
            &conversion_runner,
            &native_toolchain,
        )
        .expect("run refloat source invocation");

        assert_eq!(output, PathBuf::from("refloat.vescpkg"));
        assert_eq!(
            std::fs::read_to_string(root.join("src/conf/conf_general.h"))
                .expect("generated conf_general.h"),
            "#define PACKAGE_NAME \"Refloat\"\n#define VERSION \"1.2.1\"\n#define GIT_HASH 0x0ef6e99\n"
        );
        assert_eq!(
            native_toolchain.calls.borrow().as_slice(),
            &[(
                "make".to_owned(),
                vec![
                    "-C".to_owned(),
                    root.join("src").display().to_string(),
                    "VESC_TOOL=custom-vesc-tool".to_owned()
                ]
            )]
        );
        assert!(conversion_runner.calls().is_empty());

        let package = crate::Package::read(root.join(output)).expect("written package");
        let (_code, imports) = parse_lisp_imports(&package.lisp_data).expect("lisp imports");
        assert_eq!(imports[0].payload, b"refloat-native-built\0\0");
    }

    #[test]
    fn run_with_refloat_source_resolves_relative_vesc_tool_paths_before_make() {
        let harness = write_refloat_source(PackageTestHarness::new());
        let root = harness.root().to_path_buf();
        let conversion_runner = FakeConversionRunner::recording();
        let native_toolchain = RefloatWritingNativeToolchain::new(&root);

        run_with_toolchains(
            &root,
            [
                "build",
                "--refloat-source",
                ".",
                "--vesc-tool",
                "target/refloat-tools/vesc_tool",
            ],
            &conversion_runner,
            &native_toolchain,
        )
        .expect("run refloat source invocation");

        assert_eq!(
            native_toolchain.calls.borrow().as_slice(),
            &[(
                "make".to_owned(),
                vec![
                    "-C".to_owned(),
                    root.join("src").display().to_string(),
                    format!(
                        "VESC_TOOL={}",
                        root.join("target/refloat-tools/vesc_tool").display()
                    )
                ]
            )]
        );
        assert_eq!(
            std::fs::read_to_string(root.join("src/conf/conf_general.h"))
                .expect("generated conf_general.h"),
            "#define PACKAGE_NAME \"Refloat\"\n#define VERSION \"1.2.1\"\n#define GIT_HASH 0x0\n"
        );

        let package = crate::Package::read(root.join("refloat.vescpkg")).expect("package");
        assert!(package.description_md.contains("- Git Commit: #unknown"));
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
            parse_args(["build", "--refloat-source"]),
            Err(super::CargoVescPkgParseError::MissingRefloatSourceValue)
        );
        assert_eq!(
            parse_args(["build", "--build-date"]),
            Err(super::CargoVescPkgParseError::MissingBuildDateValue)
        );
        assert_eq!(
            parse_args(["build", "--git-commit"]),
            Err(super::CargoVescPkgParseError::MissingGitCommitValue)
        );
        assert_eq!(
            parse_args(["build", "--vesc-tool"]),
            Err(super::CargoVescPkgParseError::MissingVescToolValue)
        );
        assert_eq!(
            parse_args(["build", "--example", "pong"]),
            Err(super::CargoVescPkgParseError::UnsupportedExample(
                "pong".to_owned()
            ))
        );
    }

    fn write_refloat_source(harness: PackageTestHarness) -> PackageTestHarness {
        harness
            .write_text("package_README.md", "# Refloat\n")
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "Item { property string title: \"{{PACKAGE_NAME}}\" }\n",
            )
            .write_text(
                "pkgdesc.qml",
                "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
            )
            .write_text(
                "lisp/package.lisp",
                "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n",
            )
            .write_text(
                "src/conf/conf_general.h.in",
                "#define PACKAGE_NAME \"{{PACKAGE_NAME}}\"\n#define VERSION \"{{VERSION}}\"\n#define GIT_HASH 0x{{GIT_HASH}}\n",
            )
            .write_text("src/conf/settings.xml", "<config />\n")
            .write_bytes("src/package_lib.bin", b"refloat-native\0")
    }

    struct RefloatWritingNativeToolchain {
        source_root: PathBuf,
        calls: RefCell<Vec<(String, Vec<String>)>>,
    }

    impl RefloatWritingNativeToolchain {
        fn new(source_root: impl Into<PathBuf>) -> Self {
            Self {
                source_root: source_root.into(),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl NativeLibToolchain for RefloatWritingNativeToolchain {
        fn run(&self, program: &str, args: &[&str]) -> Result<(), String> {
            self.calls.borrow_mut().push((
                program.to_owned(),
                args.iter().copied().map(str::to_owned).collect(),
            ));
            if program == "make" {
                std::fs::write(
                    self.source_root.join("src/package_lib.bin"),
                    b"refloat-native-built\0",
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    }
}
