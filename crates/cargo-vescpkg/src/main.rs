//! Cargo subcommand entrypoint for VESC package workflows.

use std::process::ExitCode;

use vescpkg_rs_build::RealPackageRunner;
use vescpkg_rs_build::cargo_vescpkg_command::{CargoVescPkgError, run_with};

fn print_error(error: CargoVescPkgError) -> ExitCode {
    match error {
        CargoVescPkgError::Parse(parse_error) => {
            eprintln!("cargo vescpkg error: {parse_error}");
            eprintln!("usage: cargo vescpkg build [--package-only] [--target <triple>]");
            ExitCode::from(2)
        }
        CargoVescPkgError::Package(package_error) => {
            eprintln!("cargo vescpkg package failed: {package_error:?}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !matches!(args.first().map(String::as_str), Some("build")) {
        return cargo_vescpkg::run_args(std::iter::once("cargo vescpkg".to_owned()).chain(args));
    }

    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let runner = RealPackageRunner;

    match run_with(root, args, &runner) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => print_error(error),
    }
}
