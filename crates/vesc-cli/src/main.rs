use std::{process::ExitCode, thread, time::Duration};

const LISP_PROBE_RETRY_DELAY: Duration = Duration::from_secs(1);

fn main() -> ExitCode {
    match vesc_cli::parse_args(std::env::args()) {
        Ok(vesc_cli::Command::Help) => {
            println!(
                "vesc-cli: use `layout`, `status`, `scan`, `loopback`, `lisp-probe`, `package-install`, or `erase-package`"
            );
            ExitCode::SUCCESS
        }
        Ok(vesc_cli::Command::Layout) => {
            println!("workspace layout is documented in docs/workspace-layout.md");
            ExitCode::SUCCESS
        }
        Ok(vesc_cli::Command::Status) => {
            println!("status: placeholder host surface");
            ExitCode::SUCCESS
        }
        Ok(vesc_cli::Command::Scan) => match vesc_cli::btle::scan_devices() {
            Ok(devices) => {
                devices.into_iter().for_each(|device| {
                    println!(
                        "{} {:?} {:?}",
                        device.identifier, device.local_name, device.services
                    );
                });
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("scan failed: {error}");
                ExitCode::from(1)
            }
        },
        Ok(vesc_cli::Command::Loopback) => match vesc_cli::btle::BtleLoopbackTransport::new() {
            Ok(transport) => match vesc_cli::loopback::run_loopback(&transport) {
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
        },
        Ok(vesc_cli::Command::LispProbe(command)) => {
            let target = match (command.address, command.device_name) {
                (Some(address), _) => vesc_cli::loopback::LoopbackTarget::addressed(address),
                (None, Some(device_name)) => vesc_cli::loopback::LoopbackTarget::named(device_name),
                (None, None) => vesc_cli::loopback::LoopbackTarget::default(),
            };

            loop {
                if let Err(error) = vesc_cli::btle::run_lisp_probe_continuously_with_progress(
                    target.clone(),
                    |event| {
                        if event.should_print_to_cli() {
                            println!("lisp probe: {}", event.describe());
                        }
                    },
                ) {
                    eprintln!("lisp probe failed: {error}; reconnecting");
                    thread::sleep(LISP_PROBE_RETRY_DELAY);
                }
            }
        }
        Ok(vesc_cli::Command::PackageInstall(command)) => {
            let package =
                match vesc_cli::package_install::read_package_from_path(&command.package_path) {
                    Ok(package) => package,
                    Err(error) => {
                        eprintln!("failed to read package {}: {error}", command.package_path);
                        return ExitCode::from(1);
                    }
                };

            let transport = match vesc_cli::package_transport::BtlePackageInstallTransport::new() {
                Ok(transport) => transport,
                Err(error) => {
                    eprintln!("failed to initialize package transport: {error}");
                    return ExitCode::from(1);
                }
            };

            let target = match (command.address, command.device_name) {
                (Some(address), _) => vesc_cli::loopback::LoopbackTarget::addressed(address),
                (None, Some(device_name)) => vesc_cli::loopback::LoopbackTarget::named(device_name),
                (None, None) => vesc_cli::loopback::LoopbackTarget::default(),
            };

            if let Err(error) = transport.open(target) {
                eprintln!("failed to open package transport: {error}");
                return ExitCode::from(1);
            }

            match vesc_cli::package_install::install_package(&package, &transport) {
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
        Ok(vesc_cli::Command::ErasePackage(command)) => {
            let transport = match vesc_cli::package_transport::BtlePackageInstallTransport::new() {
                Ok(transport) => transport,
                Err(error) => {
                    eprintln!("failed to initialize package transport: {error}");
                    return ExitCode::from(1);
                }
            };

            let target = match (command.address, command.device_name) {
                (Some(address), _) => vesc_cli::loopback::LoopbackTarget::addressed(address),
                (None, Some(device_name)) => vesc_cli::loopback::LoopbackTarget::named(device_name),
                (None, None) => vesc_cli::loopback::LoopbackTarget::default(),
            };

            if let Err(error) = transport.open(target) {
                eprintln!("failed to open package transport: {error}");
                return ExitCode::from(1);
            }

            match vesc_cli::package_install::erase_package(&transport) {
                Ok(report) => {
                    println!("package erase ok: {:?}", report.steps);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("package erase failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(vesc_cli::ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}
