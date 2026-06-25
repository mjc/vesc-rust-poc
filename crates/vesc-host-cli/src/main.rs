use std::process::ExitCode;

fn main() -> ExitCode {
    match vesc_host_cli::parse_args(std::env::args()) {
        Ok(vesc_host_cli::Command::Help) => {
            println!("vesc-host-cli: use `layout`, `status`, or `loopback`");
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
        Ok(vesc_host_cli::Command::Loopback) => {
            match vesc_host_cli::btle::BtleLoopbackTransport::new() {
                Ok(transport) => match vesc_host_cli::loopback::run_loopback(&transport) {
                    Ok(report) => {
                        println!(
                            "loopback ok on device={} service={}: {:?}",
                            report.target().device_name_hint(),
                            report.target().service_name_hint(),
                            report.commands()
                        );
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("loopback failed: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("failed to initialize BLE transport: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(vesc_host_cli::Command::PackageInstall(path)) => {
            let package = match vesc_host_cli::package_install::read_package_from_path(&path) {
                Ok(package) => package,
                Err(error) => {
                    eprintln!("failed to read package {path}: {error}");
                    return ExitCode::from(1);
                }
            };

            let transport = vesc_host_cli::package_install::FakePackageInstallTransport::default();
            match vesc_host_cli::package_install::install_package(&package, &transport) {
                Ok(report) => {
                    println!(
                        "package install ok for {}: {:?}",
                        report.package_name, report.steps
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("package install failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(vesc_host_cli::ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}
