use std::path::PathBuf;

use crate::PackageLayout;

/// Relative path to the built native binary used by the conversion script.
pub const NATIVE_LIB_BINARY_PATH: &str = "target/native-lib-baseline/native_lib.bin";
/// Relative path to the package-library binary written by the conversion script.
pub const PACKAGE_LIB_BINARY_PATH: &str = "target/native-lib-baseline/package_lib.bin";
/// Relative path to the package conversion script.
pub const CONVERSION_SCRIPT_PATH: &str = "scripts/conv.py";

/// Command line used to convert the native binary into a package binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBinaryConversionCommand {
    script_path: PathBuf,
    native_binary_path: PathBuf,
    package_binary_path: PathBuf,
}

impl PackageBinaryConversionCommand {
    /// Construct a conversion command from its paths.
    pub fn new(
        script_path: impl Into<PathBuf>,
        native_binary_path: impl Into<PathBuf>,
        package_binary_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            script_path: script_path.into(),
            native_binary_path: native_binary_path.into(),
            package_binary_path: package_binary_path.into(),
        }
    }

    /// Return the conversion script path.
    pub fn script_path(&self) -> &PathBuf {
        &self.script_path
    }

    /// Return the native binary path.
    pub fn native_binary_path(&self) -> &PathBuf {
        &self.native_binary_path
    }

    /// Return the package binary output path.
    pub fn package_binary_path(&self) -> &PathBuf {
        &self.package_binary_path
    }

    /// Return the command-line arguments for the conversion runner.
    pub fn arguments(&self) -> Vec<String> {
        vec![
            self.script_path.to_string_lossy().into_owned(),
            self.native_binary_path.to_string_lossy().into_owned(),
            self.package_binary_path.to_string_lossy().into_owned(),
        ]
    }
}

/// Errors returned when the package conversion runner fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageBinaryConversionError {
    /// The conversion command failed before producing the package payload.
    Failed {
        /// Human-readable command failure reason.
        reason: String,
        /// Native binary input path.
        native_binary_path: PathBuf,
        /// Package binary output path.
        package_binary_path: PathBuf,
    },
}

/// Runner abstraction used to execute the package conversion script.
pub trait PackageBinaryConversionRunner {
    /// Run the supplied conversion command.
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String>;
}

/// Conversion plan for a package's native and package-library binaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBinaryConversionPlan {
    source_root: PathBuf,
    layout: PackageLayout,
}

impl PackageBinaryConversionPlan {
    /// Construct a conversion plan for one package.
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            source_root: source_root.into(),
            layout: PackageLayout::new(package_name, version),
        }
    }

    /// Return the package layout for this plan.
    pub fn layout(&self) -> &PackageLayout {
        &self.layout
    }

    /// Return the command used to perform the conversion.
    pub fn command(&self) -> PackageBinaryConversionCommand {
        PackageBinaryConversionCommand::new(
            self.conversion_script_path(),
            self.native_binary_path(),
            self.package_binary_path(),
        )
    }

    /// Return the conversion script path.
    pub fn conversion_script_path(&self) -> PathBuf {
        self.source_root.join(CONVERSION_SCRIPT_PATH)
    }

    /// Return the native binary path.
    pub fn native_binary_path(&self) -> PathBuf {
        self.source_root.join(NATIVE_LIB_BINARY_PATH)
    }

    /// Return the package binary path.
    pub fn package_binary_path(&self) -> PathBuf {
        self.source_root.join(PACKAGE_LIB_BINARY_PATH)
    }

    /// Return the files that feed the conversion step.
    pub fn conversion_inputs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [self.conversion_script_path(), self.native_binary_path()].into_iter()
    }

    /// Return the conversion runner arguments.
    pub fn conversion_command_args(&self) -> Vec<String> {
        self.command().arguments()
    }

    /// Run the conversion step with a custom runner.
    pub fn run_with<R: PackageBinaryConversionRunner>(
        &self,
        runner: &R,
    ) -> Result<(), PackageBinaryConversionError> {
        runner
            .run(&self.command())
            .map_err(|reason| PackageBinaryConversionError::Failed {
                reason,
                native_binary_path: self.native_binary_path(),
                package_binary_path: self.package_binary_path(),
            })
    }

    /// Render a short failure message with the input and output paths.
    pub fn failure_context(&self, reason: &str) -> String {
        format!(
            "{reason}: {} -> {}",
            self.native_binary_path().display(),
            self.package_binary_path().display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CONVERSION_SCRIPT_PATH, NATIVE_LIB_BINARY_PATH, PACKAGE_LIB_BINARY_PATH,
        PackageBinaryConversionError, PackageBinaryConversionPlan,
    };
    use crate::test_support::FakeConversionRunner;
    #[test]
    fn renders_the_expected_conversion_plan() {
        let plan = PackageBinaryConversionPlan::new(
            "fixtures/native-lib-baseline",
            "Rust VESC package",
            "0.1.0",
        );

        assert_eq!(
            plan.conversion_inputs().collect::<Vec<_>>(),
            vec![
                std::path::PathBuf::from("fixtures/native-lib-baseline/scripts/conv.py"),
                std::path::PathBuf::from(
                    "fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin"
                ),
            ]
        );
        assert_eq!(
            plan.conversion_command_args(),
            vec![
                "fixtures/native-lib-baseline/scripts/conv.py".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/package_lib.bin"
                    .to_owned(),
            ]
        );
        assert_eq!(
            plan.failure_context("conversion failed"),
            "conversion failed: fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin -> fixtures/native-lib-baseline/target/native-lib-baseline/package_lib.bin"
        );
        assert_eq!(
            plan.conversion_script_path(),
            std::path::PathBuf::from("fixtures/native-lib-baseline").join(CONVERSION_SCRIPT_PATH)
        );
        assert_eq!(
            plan.native_binary_path(),
            std::path::PathBuf::from("fixtures/native-lib-baseline").join(NATIVE_LIB_BINARY_PATH)
        );
        assert_eq!(
            plan.package_binary_path(),
            std::path::PathBuf::from("fixtures/native-lib-baseline").join(PACKAGE_LIB_BINARY_PATH)
        );
        assert_eq!(
            plan.command().arguments(),
            vec![
                "fixtures/native-lib-baseline/scripts/conv.py".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin".to_owned(),
                "fixtures/native-lib-baseline/target/native-lib-baseline/package_lib.bin"
                    .to_owned(),
            ]
        );
    }

    #[test]
    fn run_with_invokes_the_fake_runner() {
        let plan = PackageBinaryConversionPlan::new(
            "fixtures/native-lib-baseline",
            "Rust VESC package",
            "0.1.0",
        );
        let runner = FakeConversionRunner::recording();

        assert_eq!(plan.run_with(&runner), Ok(()));
        assert_eq!(runner.calls(), vec![plan.command()]);
    }

    #[test]
    fn run_with_wraps_runner_failures_with_path_context() {
        let plan = PackageBinaryConversionPlan::new(
            "fixtures/native-lib-baseline",
            "Rust VESC package",
            "0.1.0",
        );
        let runner = FakeConversionRunner::failing("conv.py blew up");

        assert_eq!(
            plan.run_with(&runner),
            Err(PackageBinaryConversionError::Failed {
                reason: "conv.py blew up".to_owned(),
                native_binary_path: std::path::PathBuf::from(
                    "fixtures/native-lib-baseline/target/native-lib-baseline/native_lib.bin"
                ),
                package_binary_path: std::path::PathBuf::from(
                    "fixtures/native-lib-baseline/target/native-lib-baseline/package_lib.bin"
                ),
            })
        );
    }
}
