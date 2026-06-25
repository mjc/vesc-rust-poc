use std::path::Path;
use std::process::{Command, ExitCode};

use vesc_pkg_build::{
    PackageBinaryConversionCommand, PackageBinaryConversionRunner, PackageTargetMode,
    PackageTargetPlan, PackageTargetRunner,
};

struct RealPackageRunner;

impl PackageBinaryConversionRunner for RealPackageRunner {
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
        let status = Command::new("python3")
            .arg(command.script_path())
            .arg(command.native_binary_path())
            .arg(command.package_binary_path())
            .status()
            .map_err(|error| error.to_string())?;

        status
            .success()
            .then_some(())
            .ok_or_else(|| format!("conversion helper exited with {status}"))
    }
}

impl PackageTargetRunner for RealPackageRunner {
    fn ensure_vesc_tool_available(&self, tool_path: &Path) -> Result<(), String> {
        Command::new(tool_path)
            .arg("--help")
            .output()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn run_vesc_tool(&self, tool_path: &Path, args: &[String]) -> Result<(), String> {
        let status = Command::new(tool_path)
            .args(args)
            .status()
            .map_err(|error| error.to_string())?;

        status
            .success()
            .then_some(())
            .ok_or_else(|| format!("VESC Tool exited with {status}"))
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(mode) = args.next() else {
        eprintln!("usage: vesc-pkg package|package-only");
        return ExitCode::from(2);
    };

    if args.next().is_some() {
        eprintln!("usage: vesc-pkg package|package-only");
        return ExitCode::from(2);
    }

    let mode = match mode.as_str() {
        "package" => PackageTargetMode::Package,
        "package-only" => PackageTargetMode::PackageOnly,
        other => {
            eprintln!("unknown package mode: {other}");
            return ExitCode::from(2);
        }
    };

    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };

    let tool_path = std::env::var("VESC_TOOL").unwrap_or_else(|_| "vesc_tool".to_owned());
    let plan = PackageTargetPlan::new(root, "Rust VESC package", "0.1.0", mode, tool_path);
    let runner = RealPackageRunner;

    match plan.execute_with(&runner, &runner) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package target failed: {error:?}");
            ExitCode::from(1)
        }
    }
}
