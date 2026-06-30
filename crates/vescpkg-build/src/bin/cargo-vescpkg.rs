//! Cargo subcommand entrypoint for building VESC package artifacts.

use std::process::ExitCode;

use vescpkg_build::RealPackageRunner;
use vescpkg_build::cargo_vescpkg_command::{CargoVescPkgError, run_with};

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
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let runner = RealPackageRunner;

    match run_with(root, std::env::args().skip(1), &runner) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => print_error(error),
    }
}
