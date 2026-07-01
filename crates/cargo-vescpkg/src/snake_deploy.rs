//! Install the Snake example package and prove its app-data handler responds.

use std::time::Duration;

use tokio::runtime::Runtime;

use crate::loopback::LoopbackTarget;
use crate::package_install::{PackageInstallError, PackageInstallReport, read_package_from_path};
use crate::package_transport::{BtlePackageInstallTransport, VescSession};

const COMM_CUSTOM_APP_DATA: u8 = 36;
const POST_INSTALL_SETTLE: Duration = Duration::from_millis(1500);
const SNAKE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const SNAKE_STARTUP_ATTEMPTS: usize = 8;
const SNAKE_STARTUP_RETRY_DELAY: Duration = Duration::from_secs(2);

const SNAKE_RESET: u8 = b'X';
const SNAKE_PROBE: u8 = b'P';
const SNAKE_TICK: u8 = b'T';
const SNAKE_STATE: u8 = b'S';
const SNAKE_RESPONSE: u8 = b'S';

/// Installs a Snake package and exercises its device-side app-data handler.
pub fn run_snake_deploy(
    package_path: &str,
    target: LoopbackTarget,
) -> Result<(PackageInstallReport, SnakeDeployReport), SnakeDeployError> {
    eprintln!("snake-deploy: reading package {package_path}");
    let package = read_package_from_path(package_path).map_err(SnakeDeployError::Package)?;
    let transport = BtlePackageInstallTransport::new().map_err(SnakeDeployError::Transport)?;
    eprintln!(
        "snake-deploy: opening BLE transport device={} service={}",
        target.device_name_hint(),
        target.service_name_hint()
    );
    transport
        .open(target.clone())
        .map_err(|error| stage_error("open BLE package transport", error))
        .map_err(SnakeDeployError::Transport)?;

    eprintln!("snake-deploy: installing package {}", package.name);
    let install = crate::package_install::install_package(&package, &transport)
        .map_err(|error| stage_error("install Snake package", error))
        .map_err(SnakeDeployError::Transport)?;

    eprintln!("snake-deploy: waiting for package loader settle");
    std::thread::sleep(POST_INSTALL_SETTLE);

    eprintln!("snake-deploy: running app-data smoke");
    let report = transport
        .with_app_data_session(|runtime, session| run_snake_smoke(runtime, session, target))
        .map_err(|error| stage_error("run Snake app-data smoke", error))
        .map_err(SnakeDeployError::Smoke)?;

    transport.close();
    Ok((install, report))
}

fn stage_error(stage: impl AsRef<str>, error: PackageInstallError) -> PackageInstallError {
    match error {
        PackageInstallError::Device(reason) => {
            PackageInstallError::Device(format!("{}: {reason}", stage.as_ref()))
        }
        PackageInstallError::Io(reason) => {
            PackageInstallError::Io(format!("{}: {reason}", stage.as_ref()))
        }
        PackageInstallError::InvalidPackage => PackageInstallError::InvalidPackage,
    }
}

fn run_snake_smoke(
    runtime: &Runtime,
    session: &mut VescSession,
    target: LoopbackTarget,
) -> Result<SnakeDeployReport, PackageInstallError> {
    eprintln!("snake-deploy: confirming firmware after install");
    session
        .confirm_fw_version(runtime)
        .map_err(|error| stage_error("post-install firmware preflight", error))?;
    session.clear_packet_state();

    eprintln!("snake-deploy: sending handler probe");
    let probe = send_snake_command(runtime, session, SNAKE_PROBE)
        .map_err(|error| stage_error("send Snake handler probe command", error))?;
    eprintln!("snake-deploy: handler probe response {probe:?}");
    eprintln!("snake-deploy: sending reset");
    let reset = wait_for_snake_handler(runtime, session)?;
    eprintln!("snake-deploy: reset response {reset:?}");
    eprintln!("snake-deploy: sending tick");
    let tick = send_snake_command(runtime, session, SNAKE_TICK)
        .map_err(|error| stage_error("send Snake tick command", error))?;
    eprintln!("snake-deploy: tick response {tick:?}");
    eprintln!("snake-deploy: sending state query");
    let state = send_snake_command(runtime, session, SNAKE_STATE)
        .map_err(|error| stage_error("send Snake state command", error))?;
    eprintln!("snake-deploy: state response {state:?}");

    if probe.command != SNAKE_PROBE
        || reset.command != SNAKE_RESET
        || tick.command != SNAKE_TICK
        || state.command != SNAKE_STATE
    {
        return Err(PackageInstallError::Device(format!(
            "snake command echo mismatch: probe={probe:?} reset={reset:?} tick={tick:?} state={state:?}"
        )));
    }
    if tick.handler_count <= reset.handler_count {
        return Err(PackageInstallError::Device(format!(
            "snake handler counter did not advance: reset={reset:?} tick={tick:?}"
        )));
    }
    if tick.tick <= reset.tick {
        return Err(PackageInstallError::Device(format!(
            "snake tick did not advance on device: reset={reset:?} tick={tick:?}"
        )));
    }
    if state.tick != tick.tick {
        return Err(PackageInstallError::Device(format!(
            "snake state query changed tick unexpectedly: tick={} state={}",
            tick.tick, state.tick
        )));
    }

    Ok(SnakeDeployReport {
        target,
        reset,
        tick,
        state,
    })
}

fn wait_for_snake_handler(
    runtime: &Runtime,
    session: &mut VescSession,
) -> Result<SnakeAppResponse, PackageInstallError> {
    let mut last_error = None;
    for attempt in 1..=SNAKE_STARTUP_ATTEMPTS {
        session.clear_packet_state();
        match send_snake_command(runtime, session, SNAKE_RESET) {
            Ok(response) => return Ok(response),
            Err(error) => {
                last_error = Some(error);
                if attempt != SNAKE_STARTUP_ATTEMPTS {
                    std::thread::sleep(SNAKE_STARTUP_RETRY_DELAY);
                }
            }
        }
    }

    Err(stage_error(
        format!("wait for Snake app-data handler after {SNAKE_STARTUP_ATTEMPTS} attempts"),
        last_error.unwrap_or_else(|| {
            PackageInstallError::Device("Snake app-data handler did not reply".to_owned())
        }),
    ))
}

fn send_snake_command(
    runtime: &Runtime,
    session: &mut VescSession,
    command: u8,
) -> Result<SnakeAppResponse, PackageInstallError> {
    session.clear_packet_state();
    let packet = crate::package_transport::build_command_packet(COMM_CUSTOM_APP_DATA, &[command]);
    runtime.block_on(crate::package_transport::write_ble_uart_packet(
        &session.peripheral,
        &session.rx_char,
        &packet,
    ))?;

    let response = session.receive_custom_app_data(SNAKE_RESPONSE_TIMEOUT)?;
    SnakeAppResponse::decode(&response)
}

/// Parsed response returned by the Snake package app-data handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeAppResponse {
    /// Raw Snake state byte from the package.
    pub state: u8,
    /// Package-side game tick counter.
    pub tick: u16,
    /// Package-side score.
    pub score: u16,
    /// Snake head x coordinate.
    pub head_x: u8,
    /// Snake head y coordinate.
    pub head_y: u8,
    /// Command byte observed by the package-side handler.
    pub command: u8,
    /// Wrapping package-side handler invocation counter.
    pub handler_count: u8,
}

impl SnakeAppResponse {
    fn decode(bytes: &[u8]) -> Result<Self, PackageInstallError> {
        let [
            kind,
            state,
            tick_lo,
            tick_hi,
            score_lo,
            score_hi,
            head_x,
            head_y,
            command,
            handler_count,
        ] = bytes
        else {
            return Err(PackageInstallError::Device(format!(
                "snake response had wrong length: {} bytes",
                bytes.len()
            )));
        };
        if *kind != SNAKE_RESPONSE {
            return Err(PackageInstallError::Device(format!(
                "snake response had wrong kind: 0x{kind:02x}"
            )));
        }

        Ok(Self {
            state: *state,
            tick: u16::from_le_bytes([*tick_lo, *tick_hi]),
            score: u16::from_le_bytes([*score_lo, *score_hi]),
            head_x: *head_x,
            head_y: *head_y,
            command: *command,
            handler_count: *handler_count,
        })
    }
}

/// Evidence returned by the Snake deploy smoke test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeDeployReport {
    /// Device target that was selected for the deploy.
    pub target: LoopbackTarget,
    /// Response after resetting the package-side game.
    pub reset: SnakeAppResponse,
    /// Response after advancing the package-side game one tick.
    pub tick: SnakeAppResponse,
    /// Response after querying package-side state.
    pub state: SnakeAppResponse,
}

/// Errors returned by the Snake deploy path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnakeDeployError {
    /// Reading or decoding the package failed.
    Package(PackageInstallError),
    /// Installing the package over BLE failed.
    Transport(PackageInstallError),
    /// The post-install Snake app-data smoke test failed.
    Smoke(PackageInstallError),
}

impl std::fmt::Display for SnakeDeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Package(error) => write!(f, "failed to read package: {error}"),
            Self::Transport(error) => write!(f, "package install failed: {error}"),
            Self::Smoke(error) => write!(f, "snake app-data smoke failed: {error}"),
        }
    }
}

impl std::error::Error for SnakeDeployError {}

#[cfg(test)]
mod tests {
    use super::{SNAKE_RESPONSE, SnakeAppResponse, stage_error};
    use crate::package_install::PackageInstallError;

    #[test]
    fn decodes_snake_app_data_response() {
        let response = SnakeAppResponse::decode(&[SNAKE_RESPONSE, 1, 2, 0, 3, 0, 12, 14, b'T', 9])
            .expect("response");

        assert_eq!(response.state, 1);
        assert_eq!(response.tick, 2);
        assert_eq!(response.score, 3);
        assert_eq!(response.head_x, 12);
        assert_eq!(response.head_y, 14);
        assert_eq!(response.command, b'T');
        assert_eq!(response.handler_count, 9);
    }

    #[test]
    fn rejects_non_snake_app_data_response() {
        assert!(SnakeAppResponse::decode(&[b'?', 1, 2, 0, 3, 0, 12, 14, b'T', 9]).is_err());
        assert!(SnakeAppResponse::decode(&[SNAKE_RESPONSE, 1, 2]).is_err());
    }

    #[test]
    fn stage_errors_keep_deploy_phase() {
        let error = stage_error(
            "open BLE package transport",
            PackageInstallError::Device("timed out waiting for a BLE reply".to_owned()),
        );

        assert!(
            error
                .to_string()
                .contains("open BLE package transport: timed out")
        );
    }
}
