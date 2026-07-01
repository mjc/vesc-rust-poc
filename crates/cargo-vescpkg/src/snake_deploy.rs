//! Install the Snake example package and prove its app-data handler responds.

use std::time::Duration;

use tokio::runtime::Runtime;

use crate::loopback::LoopbackTarget;
use crate::package_install::{PackageInstallError, PackageInstallReport, read_package_from_path};
use crate::package_transport::{BtlePackageInstallTransport, VescSession};

const COMM_CUSTOM_APP_DATA: u8 = 36;
const POST_INSTALL_SETTLE: Duration = Duration::from_millis(1500);
const SNAKE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);

const SNAKE_RESET: u8 = b'X';
const SNAKE_TICK: u8 = b'T';
const SNAKE_STATE: u8 = b'S';
const SNAKE_RESPONSE: u8 = b'S';

/// Installs a Snake package and exercises its device-side app-data handler.
pub fn run_snake_deploy(
    package_path: &str,
    target: LoopbackTarget,
) -> Result<(PackageInstallReport, SnakeDeployReport), SnakeDeployError> {
    let package = read_package_from_path(package_path).map_err(SnakeDeployError::Package)?;
    let transport = BtlePackageInstallTransport::new().map_err(SnakeDeployError::Transport)?;
    transport
        .open(target.clone())
        .map_err(SnakeDeployError::Transport)?;

    let install = crate::package_install::install_package(&package, &transport)
        .map_err(SnakeDeployError::Transport)?;

    std::thread::sleep(POST_INSTALL_SETTLE);

    let report = transport
        .with_app_data_session(|runtime, session| run_snake_smoke(runtime, session, target))
        .map_err(SnakeDeployError::Smoke)?;

    transport.close();
    Ok((install, report))
}

fn run_snake_smoke(
    runtime: &Runtime,
    session: &mut VescSession,
    target: LoopbackTarget,
) -> Result<SnakeDeployReport, PackageInstallError> {
    session.confirm_fw_version(runtime)?;
    session.clear_packet_state();

    let reset = send_snake_command(runtime, session, SNAKE_RESET)?;
    let tick = send_snake_command(runtime, session, SNAKE_TICK)?;
    let state = send_snake_command(runtime, session, SNAKE_STATE)?;

    if tick.tick <= reset.tick {
        return Err(PackageInstallError::Device(format!(
            "snake tick did not advance on device: reset={} tick={}",
            reset.tick, tick.tick
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

fn send_snake_command(
    runtime: &Runtime,
    session: &mut VescSession,
    command: u8,
) -> Result<SnakeAppResponse, PackageInstallError> {
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
    use super::{SNAKE_RESPONSE, SnakeAppResponse};

    #[test]
    fn decodes_snake_app_data_response() {
        let response =
            SnakeAppResponse::decode(&[SNAKE_RESPONSE, 1, 2, 0, 3, 0, 12, 14]).expect("response");

        assert_eq!(response.state, 1);
        assert_eq!(response.tick, 2);
        assert_eq!(response.score, 3);
        assert_eq!(response.head_x, 12);
        assert_eq!(response.head_y, 14);
    }

    #[test]
    fn rejects_non_snake_app_data_response() {
        assert!(SnakeAppResponse::decode(&[b'?', 1, 2, 0, 3, 0, 12, 14]).is_err());
        assert!(SnakeAppResponse::decode(&[SNAKE_RESPONSE, 1, 2]).is_err());
    }
}
