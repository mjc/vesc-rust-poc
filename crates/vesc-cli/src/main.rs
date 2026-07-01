//! Compatibility binary for VESC package and loopback workflows.

use std::process::ExitCode;

fn main() -> ExitCode {
    vesc_cli::run_args(std::env::args())
}
