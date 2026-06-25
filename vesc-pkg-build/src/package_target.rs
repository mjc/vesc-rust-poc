use std::path::{Path, PathBuf};

use crate::package_artifacts::PackageArtifactInspectionError;
use crate::package_build::PackageBuildPlan;
use crate::package_conversion::{PackageBinaryConversionError, PackageBinaryConversionRunner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageTargetMode {
    Package,
    PackageOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageTargetError {
    Stage { path: PathBuf, reason: String },
    Conversion(PackageBinaryConversionError),
    Inspection(PackageArtifactInspectionError),
    ToolUnavailable { tool_path: PathBuf, reason: String },
    ToolFailed { tool_path: PathBuf, reason: String },
}

pub trait PackageTargetRunner {
    fn ensure_vesc_tool_available(&self, tool_path: &Path) -> Result<(), String>;
    fn run_vesc_tool(&self, tool_path: &Path, args: &[String]) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTargetPlan {
    build_plan: PackageBuildPlan,
    mode: PackageTargetMode,
    vesc_tool_path: PathBuf,
}

impl PackageTargetPlan {
    pub fn new(
        source_root: impl Into<PathBuf>,
        package_name: impl Into<String>,
        version: impl Into<String>,
        mode: PackageTargetMode,
        vesc_tool_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            build_plan: PackageBuildPlan::new(source_root, package_name, version),
            mode,
            vesc_tool_path: vesc_tool_path.into(),
        }
    }

    pub fn build_plan(&self) -> &PackageBuildPlan {
        &self.build_plan
    }

    pub fn mode(&self) -> PackageTargetMode {
        self.mode
    }

    pub fn vesc_tool_path(&self) -> &Path {
        &self.vesc_tool_path
    }

    pub fn package_output_path(&self) -> PathBuf {
        self.build_plan.package_output_path()
    }

    pub fn execute_with<C, R>(
        &self,
        conversion_runner: &C,
        target_runner: &R,
    ) -> Result<PathBuf, PackageTargetError>
    where
        C: PackageBinaryConversionRunner,
        R: PackageTargetRunner,
    {
        if self.mode == PackageTargetMode::Package {
            target_runner
                .ensure_vesc_tool_available(self.vesc_tool_path.as_path())
                .map_err(|reason| PackageTargetError::ToolUnavailable {
                    tool_path: self.vesc_tool_path.clone(),
                    reason,
                })?;
        }

        self.build_plan
            .stage_package_assets()
            .map_err(|error| PackageTargetError::Stage {
                path: self.build_plan.inspection_plan().staging_dir_path(),
                reason: error.to_string(),
            })?;
        self.build_plan
            .convert_package_binary_with(conversion_runner)
            .map_err(PackageTargetError::Conversion)?;
        self.build_plan
            .inspect_package_artifacts()
            .map_err(PackageTargetError::Inspection)?;

        if self.mode == PackageTargetMode::Package {
            target_runner
                .run_vesc_tool(
                    self.vesc_tool_path.as_path(),
                    &self.build_plan.vesc_tool_args(),
                )
                .map_err(|reason| PackageTargetError::ToolFailed {
                    tool_path: self.vesc_tool_path.clone(),
                    reason,
                })?;
        }

        Ok(self.package_output_path())
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageTargetError, PackageTargetMode, PackageTargetPlan, PackageTargetRunner};
    use crate::package_conversion::{
        PackageBinaryConversionCommand, PackageBinaryConversionRunner,
    };
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct FakeConversionRunner {
        calls: RefCell<Vec<PackageBinaryConversionCommand>>,
    }

    impl FakeConversionRunner {
        fn calls(&self) -> Vec<PackageBinaryConversionCommand> {
            self.calls.borrow().clone()
        }
    }

    impl PackageBinaryConversionRunner for FakeConversionRunner {
        fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
            self.calls.borrow_mut().push(command.clone());
            if let Some(parent) = command.package_binary_path().parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(command.package_binary_path(), b"payload").map_err(|error| error.to_string())
        }
    }

    struct FakeTargetRunner {
        available: RefCell<Result<(), String>>,
        availability_checks: RefCell<Vec<PathBuf>>,
        runs: RefCell<Vec<(PathBuf, Vec<String>)>>,
    }

    impl FakeTargetRunner {
        fn package() -> Self {
            Self {
                available: RefCell::new(Ok(())),
                availability_checks: RefCell::new(Vec::new()),
                runs: RefCell::new(Vec::new()),
            }
        }

        fn unavailable(reason: impl Into<String>) -> Self {
            Self {
                available: RefCell::new(Err(reason.into())),
                availability_checks: RefCell::new(Vec::new()),
                runs: RefCell::new(Vec::new()),
            }
        }

        fn availability_checks(&self) -> Vec<PathBuf> {
            self.availability_checks.borrow().clone()
        }

        fn runs(&self) -> Vec<(PathBuf, Vec<String>)> {
            self.runs.borrow().clone()
        }
    }

    impl PackageTargetRunner for FakeTargetRunner {
        fn ensure_vesc_tool_available(&self, tool_path: &Path) -> Result<(), String> {
            self.availability_checks
                .borrow_mut()
                .push(tool_path.to_path_buf());
            self.available.borrow().clone()
        }

        fn run_vesc_tool(&self, tool_path: &Path, args: &[String]) -> Result<(), String> {
            self.runs
                .borrow_mut()
                .push((tool_path.to_path_buf(), args.to_vec()));
            Ok(())
        }
    }

    fn unique_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vesc-rust-poc-package-target-{nanos}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn package_only_stages_inspects_and_skips_vesc_tool() {
        let root = unique_root();
        let target = PackageTargetPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageTargetMode::PackageOnly,
            "vesc_tool",
        );
        let conversion_runner = FakeConversionRunner::default();
        let target_runner = FakeTargetRunner::package();

        let output = target
            .execute_with(&conversion_runner, &target_runner)
            .expect("package target");

        assert_eq!(output, target.package_output_path());
        assert_eq!(
            conversion_runner.calls(),
            vec![target.build_plan().conversion_plan().command()]
        );
        assert!(target_runner.availability_checks().is_empty());
        assert!(target_runner.runs().is_empty());
        assert!(target.build_plan().inspect_package_artifacts().is_ok());
        assert!(root
            .join("target/vescpkg/Rust-VESC-package-0.1.0/README.md")
            .exists());
        assert!(root
            .join("target/native-lib-baseline/package_lib.bin")
            .exists());
    }

    #[test]
    fn package_mode_checks_vesc_tool_and_runs_it() {
        let root = unique_root();
        let target = PackageTargetPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageTargetMode::Package,
            "/nix/store/fake-vesc-tool/bin/vesc_tool",
        );
        let conversion_runner = FakeConversionRunner::default();
        let target_runner = FakeTargetRunner::package();

        let output = target
            .execute_with(&conversion_runner, &target_runner)
            .expect("package target");

        assert_eq!(output, target.package_output_path());
        assert_eq!(
            target_runner.availability_checks(),
            vec![PathBuf::from("/nix/store/fake-vesc-tool/bin/vesc_tool")]
        );
        assert_eq!(
            target_runner.runs(),
            vec![(
                PathBuf::from("/nix/store/fake-vesc-tool/bin/vesc_tool"),
                vec![
                    "--buildPkgFromDesc".to_owned(),
                    "target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml".to_owned(),
                ],
            )]
        );
    }

    #[test]
    fn package_mode_reports_missing_vesc_tool() {
        let root = unique_root();
        let target = PackageTargetPlan::new(
            &root,
            "Rust VESC package",
            "0.1.0",
            PackageTargetMode::Package,
            "vesc_tool",
        );
        let conversion_runner = FakeConversionRunner::default();
        let target_runner = FakeTargetRunner::unavailable("vesc_tool not found");

        assert_eq!(
            target.execute_with(&conversion_runner, &target_runner),
            Err(PackageTargetError::ToolUnavailable {
                tool_path: PathBuf::from("vesc_tool"),
                reason: "vesc_tool not found".to_owned(),
            })
        );
        assert_eq!(
            target_runner.availability_checks(),
            vec![PathBuf::from("vesc_tool")]
        );
        assert!(target_runner.runs().is_empty());
    }
}
