//! Install a package and run BLE loopback on the same open session.

use std::time::Duration;

use tokio::runtime::Runtime;
use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackPacket;

use crate::loopback::{LoopbackReport, LoopbackTarget, LoopbackTransportError};
use crate::package_install::{PackageInstallError, PackageInstallReport, read_package_from_path};
use crate::package_transport::{BtlePackageInstallTransport, VescSession};
use crate::vesc_uart::encode_packet;

const COMM_CUSTOM_APP_DATA: u8 = 36;
const POST_INSTALL_SETTLE: Duration = Duration::from_millis(1500);
const LOOPBACK_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);

/// Opens BLE and runs the standard package app-data loopback sequence.
pub fn run_loopback_probe(
    target: LoopbackTarget,
    mut progress: impl FnMut(String),
) -> Result<LoopbackReport, DeployError> {
    let transport = BtlePackageInstallTransport::new().map_err(DeployError::Transport)?;
    transport
        .open(target.clone())
        .map_err(DeployError::Transport)?;
    progress("BLE session open".to_owned());
    let result = transport
        .with_loopback_session(|runtime, session| {
            run_loopback_on_session(runtime, session, target, &mut progress)
        })
        .map_err(DeployError::Loopback);
    transport.close();
    result
}

/// Reads a `.vescpkg`, installs it over BLE, and runs a loopback smoke test.
pub fn run_deploy(
    package_path: &str,
    target: LoopbackTarget,
    mut progress: impl FnMut(String),
) -> Result<(PackageInstallReport, LoopbackReport), DeployError> {
    let package = read_package_from_path(package_path).map_err(DeployError::Package)?;
    let transport = BtlePackageInstallTransport::new().map_err(DeployError::Transport)?;
    transport
        .open(target.clone())
        .map_err(DeployError::Transport)?;

    let result = (|| {
        let install = crate::package_install::install_package(&package, &transport)
            .map_err(DeployError::Transport)?;

        progress("BLE session open".to_owned());
        std::thread::sleep(POST_INSTALL_SETTLE);

        let loopback = transport
            .with_loopback_session(|runtime, session| {
                run_loopback_on_session(runtime, session, target.clone(), &mut progress)
            })
            .map_err(DeployError::Loopback)?;

        Ok((install, loopback))
    })();

    transport.close();
    result
}

fn run_loopback_on_session(
    runtime: &Runtime,
    session: &mut VescSession,
    target: LoopbackTarget,
    progress: &mut impl FnMut(String),
) -> Result<LoopbackReport, LoopbackTransportError> {
    progress("step fw-version-preflight: starting".to_owned());
    session
        .confirm_fw_version(runtime)
        .map_err(map_package_device_error)?;
    progress("step fw-version-preflight: received COMM_FW_VERSION ok".to_owned());

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
        progress(format!("step {step}: starting"));
        let (payload, len) = packet.encode();
        let wire = build_custom_app_data_packet(&payload[..len]);
        progress(format!(
            "step {step}: sending {} byte(s) wire={}",
            wire.len(),
            hex_snippet(&wire, 48)
        ));
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
        let decoded =
            LoopbackPacket::decode(&response).map_err(LoopbackTransportError::Protocol)?;
        progress(format!(
            "step {step}: reply {:?} wire={}",
            decoded.frame().command(),
            hex_snippet(&response, 32)
        ));
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

fn hex_snippet(bytes: &[u8], max_bytes: usize) -> String {
    let shown = bytes.len().min(max_bytes);
    let mut hex = bytes[..shown]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if bytes.len() > max_bytes {
        hex.push('…');
    }
    hex
}

fn map_package_device_error(error: PackageInstallError) -> LoopbackTransportError {
    match error {
        PackageInstallError::Device(reason) => LoopbackTransportError::Device(reason),
        other => LoopbackTransportError::Device(other.to_string()),
    }
}

/// Errors returned by the build, install, and loopback deploy flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployError {
    /// Reading or decoding the package failed.
    Package(PackageInstallError),
    /// Installing or erasing the package over the transport failed.
    Transport(PackageInstallError),
    /// The post-deploy loopback smoke test failed.
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
