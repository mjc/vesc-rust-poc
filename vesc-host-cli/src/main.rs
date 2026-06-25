use std::process::ExitCode;

fn main() -> ExitCode {
    match vesc_host_cli::parse_args(std::env::args()) {
        Ok(vesc_host_cli::Command::Help) => {
            println!("vesc-host-cli: use `layout` or `status`");
            ExitCode::SUCCESS
        }
        Ok(vesc_host_cli::Command::Layout) => {
            println!("workspace layout is documented in docs/workspace-layout.md");
            ExitCode::SUCCESS
        }
        Ok(vesc_host_cli::Command::Status) => {
            println!("status: placeholder host surface");
            ExitCode::SUCCESS
        }
        Err(vesc_host_cli::ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}
