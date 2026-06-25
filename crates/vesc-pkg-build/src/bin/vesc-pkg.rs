use std::fs;
use std::process::ExitCode;

use vesc_pkg_build::{
    PackageBinaryConversionCommand, PackageBinaryConversionRunner, PackageProvenance,
    PackageTargetMode, PackageTargetPlan, BLE_LOOPBACK_PACKAGE_NAME,
};

struct RealPackageRunner;

impl PackageBinaryConversionRunner for RealPackageRunner {
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
        vesc_pkg_build::symbol_audit::build_final_native_lib_binary(
            command.native_binary_path().as_path(),
        );

        if let Some(parent) = command.package_binary_path().parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        fs::copy(command.native_binary_path(), command.package_binary_path())
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn package_provenance_from_env() -> PackageProvenance {
    PackageProvenance::new(
        std::env::var("VESC_PKG_GIT_COMMIT").ok(),
        std::env::var("VESC_PKG_BUILD_DATE").ok(),
    )
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

    let plan = PackageTargetPlan::with_provenance(
        root,
        BLE_LOOPBACK_PACKAGE_NAME,
        "0.1.0",
        package_provenance_from_env(),
        mode,
    );
    let runner = RealPackageRunner;

    match plan.execute_with(&runner) {
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
