//! Standalone package builder for the loopback VESC package fixture.

use std::process::ExitCode;

use vesc_pkg::{
    BLE_LOOPBACK_PACKAGE_NAME, PackageTargetMode, PackageTargetPlan, RealPackageRunner,
    package_provenance_from_env,
};

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
