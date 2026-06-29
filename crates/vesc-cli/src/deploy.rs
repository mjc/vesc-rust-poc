//! Install a package and run BLE loopback on the same open session.

use std::time::Duration;

use tokio::runtime::Runtime;
use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackPacket;

use crate::loopback::{LoopbackReport, LoopbackTarget, LoopbackTransportError};
use crate::loopback_debug::{LoopbackProgress, hex_snippet};
use crate::package_install::{PackageInstallError, PackageInstallReport, read_package_from_path};
use crate::package_transport::{BtlePackageInstallTransport, VescSession};
use crate::vesc_uart::encode_packet;

const COMM_CUSTOM_APP_DATA: u8 = 36;
const POST_INSTALL_SETTLE: Duration = Duration::from_millis(1500);
const LOOPBACK_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);

pub fn run_deploy(
    package_path: &str,
    target: LoopbackTarget,
    mut progress: impl FnMut(LoopbackProgress),
) -> Result<(PackageInstallReport, LoopbackReport), DeployError> {
    let package = read_package_from_path(package_path).map_err(DeployError::Package)?;
    let transport = BtlePackageInstallTransport::new().map_err(DeployError::Transport)?;
    transport
        .open(target.clone())
        .map_err(DeployError::Transport)?;

    let install = crate::package_install::install_package(&package, &transport)
        .map_err(DeployError::Transport)?;

    progress(LoopbackProgress::SessionOpened);
    std::thread::sleep(POST_INSTALL_SETTLE);

    let loopback = transport
        .with_loopback_session(|runtime, session| {
            run_loopback_on_session(runtime, session, target.clone(), &mut progress)
        })
        .map_err(DeployError::Loopback)?;

    transport.close();
    Ok((install, loopback))
}

fn run_loopback_on_session(
    runtime: &Runtime,
    session: &mut VescSession,
    target: LoopbackTarget,
    progress: &mut impl FnMut(LoopbackProgress),
) -> Result<LoopbackReport, LoopbackTransportError> {
    progress(LoopbackProgress::StepStarted {
        step: "fw-version-preflight",
    });
    session
        .confirm_fw_version(runtime)
        .map_err(map_package_device_error)?;
    progress(LoopbackProgress::NonAppDataPacket {
        step: "fw-version-preflight",
        summary: "COMM_FW_VERSION ok".to_owned(),
    });

    session.clear_packet_state();

    let steps: [(&'static str, LoopbackPacket); 4] = [
        (
            "ping",
            LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping"),
        ),
        (
            "echo",
            LoopbackPacket::new(WireCommand::Echo, &[9, 8]).expect("echo"),
        ),
        (
            "status",
            LoopbackPacket::new(WireCommand::Status, &[]).expect("status"),
        ),
        (
            "teardown",
            LoopbackPacket::new(WireCommand::Teardown, &[]).expect("teardown"),
        ),
    ];

    let mut commands = Vec::with_capacity(steps.len());
    for (step, packet) in steps {
        session.clear_packet_state();
        progress(LoopbackProgress::StepStarted { step });
        let (payload, len) = packet.encode();
        let wire = build_custom_app_data_packet(&payload[..len]);
        progress(LoopbackProgress::Sending {
            step,
            wire_hex: hex_snippet(&wire, 48),
            bytes: wire.len(),
        });
        runtime
            .block_on(crate::package_transport::write_ble_uart_packet(
                &session.peripheral,
                &session.rx_char,
                &wire,
            ))
            .map_err(map_package_device_error)?;

        let response = session
            .receive_custom_app_data(LOOPBACK_RESPONSE_TIMEOUT)
            .map_err(map_package_device_error)?;
        progress(LoopbackProgress::ReplyReceived {
            step,
            wire_hex: hex_snippet(&response, 32),
            command: LoopbackPacket::decode(&response)
                .map_err(LoopbackTransportError::Protocol)?
                .frame()
                .command(),
        });
        let decoded =
            LoopbackPacket::decode(&response).map_err(LoopbackTransportError::Protocol)?;
        progress(LoopbackProgress::StepSucceeded {
            step,
            command: decoded.frame().command(),
        });
        commands.push(decoded.frame().command());
    }

    Ok(LoopbackReport::new(target, commands))
}

fn build_custom_app_data_packet(payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(payload.len() + 1);
    data.push(COMM_CUSTOM_APP_DATA);
    data.extend_from_slice(payload);
    encode_packet(&data)
}

fn map_package_device_error(error: PackageInstallError) -> LoopbackTransportError {
    match error {
        PackageInstallError::Device(reason) => LoopbackTransportError::Device(reason),
        other => LoopbackTransportError::Device(other.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployError {
    Package(PackageInstallError),
    Transport(PackageInstallError),
    Loopback(LoopbackTransportError),
}

impl std::fmt::Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Package(error) => write!(f, "failed to read package: {error}"),
            Self::Transport(error) => write!(f, "package install failed: {error}"),
            Self::Loopback(error) => write!(f, "loopback failed: {error}"),
        }
    }
}

impl std::error::Error for DeployError {}
