//! Cargo subcommand entrypoint for VESC package workflows.

fn main() -> std::process::ExitCode {
    cargo_vescpkg::run_args(std::env::args())
}
