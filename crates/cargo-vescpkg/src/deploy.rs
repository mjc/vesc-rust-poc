//! Install a package and run BLE loopback on the same open session.

use std::time::{Duration, Instant};

use tokio::runtime::Runtime;
use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackPacket;
use vesc_protocol::control_loop::{
    CommandError as ControlLoopCommandError, ControlLoopStatus, encode_setpoint_command,
    encode_status_command,
};

use crate::loopback::{LoopbackReport, LoopbackTarget, LoopbackTransportError};
use crate::package_install::PackageInstallError;
use crate::package_transport::{BtlePackageInstallTransport, VescSession};
use crate::vesc_uart::encode_packet;

const COMM_CUSTOM_APP_DATA: u8 = 36;
const LOOPBACK_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const CONTROL_LOOP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
const CONTROL_LOOP_SETPOINT: i16 = 100;
const CONTROL_LOOP_STATUS_SAMPLES: usize = 4;

/// Opens BLE and runs the standard package app-data loopback sequence.
pub fn run_loopback_probe(
    target: LoopbackTarget,
    mut progress: impl FnMut(String),
) -> Result<LoopbackReport, DeployError> {
    let transport = BtlePackageInstallTransport::new().map_err(DeployError::Transport)?;
    transport
        .open_without_preflight(target.clone())
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

/// Report produced by the no-actuation control-loop probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLoopProbeReport {
    target: LoopbackTarget,
    statuses: Vec<ControlLoopStatus>,
    elapsed: Duration,
}

impl ControlLoopProbeReport {
    /// Return the target used for the probe.
    pub const fn target(&self) -> &LoopbackTarget {
        &self.target
    }

    /// Return each status sample in wire order.
    pub fn statuses(&self) -> &[ControlLoopStatus] {
        &self.statuses
    }

    /// Return host-observed probe duration.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Errors returned by the no-actuation control-loop probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlLoopProbeError {
    /// BLE transport or firmware preflight failed.
    Transport(PackageInstallError),
    /// The package returned an unexpected ACK or status payload.
    InvalidResponse(ControlLoopCommandError),
    /// The package returned a valid status but its loop did not advance.
    TickDidNotAdvance {
        /// Tick count from the first status sample.
        initial: u32,
        /// Tick count from the final status sample.
        final_tick: u32,
    },
    /// The package did not retain the requested setpoint.
    SetpointMismatch {
        /// Setpoint sent by the probe.
        expected: i16,
        /// Setpoint echoed by the package.
        observed: i16,
    },
}

impl std::fmt::Display for ControlLoopProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "control-loop transport failed: {error}"),
            Self::InvalidResponse(error) => write!(f, "invalid control-loop response: {error:?}"),
            Self::TickDidNotAdvance {
                initial,
                final_tick,
            } => {
                write!(
                    f,
                    "control-loop tick did not advance ({initial} -> {final_tick})"
                )
            }
            Self::SetpointMismatch { expected, observed } => {
                write!(
                    f,
                    "control-loop setpoint mismatch ({expected} != {observed})"
                )
            }
        }
    }
}

impl std::error::Error for ControlLoopProbeError {}

/// Opens BLE and probes the installed no-actuation control-loop package.
pub fn run_control_loop_probe(
    target: LoopbackTarget,
    mut progress: impl FnMut(String),
) -> Result<ControlLoopProbeReport, ControlLoopProbeError> {
    let transport = BtlePackageInstallTransport::new().map_err(ControlLoopProbeError::Transport)?;
    transport
        .open_without_preflight(target.clone())
        .map_err(ControlLoopProbeError::Transport)?;
    progress("BLE session open".to_owned());
    let result = transport.with_runtime_session(
        || {
            ControlLoopProbeError::Transport(PackageInstallError::Device(
                "BLE transport has not been opened".to_owned(),
            ))
        },
        |runtime, session| run_control_loop_on_session(runtime, session, target, &mut progress),
    );
    transport.close();
    result
}

fn run_control_loop_on_session(
    runtime: &Runtime,
    session: &mut VescSession,
    target: LoopbackTarget,
    progress: &mut impl FnMut(String),
) -> Result<ControlLoopProbeReport, ControlLoopProbeError> {
    progress("step fw-version-preflight: starting".to_owned());
    session
        .confirm_fw_version(runtime)
        .map_err(ControlLoopProbeError::Transport)?;
    progress("step fw-version-preflight: received COMM_FW_VERSION ok".to_owned());

    let started = Instant::now();
    let ack = send_control_loop_packet(
        runtime,
        session,
        &encode_setpoint_command(CONTROL_LOOP_SETPOINT),
    )?;
    if ack.as_slice() != [1, 0] {
        return Err(ControlLoopProbeError::InvalidResponse(
            ControlLoopCommandError::UnexpectedResponse,
        ));
    }
    progress(format!("step setpoint: accepted {CONTROL_LOOP_SETPOINT}"));

    let mut statuses = Vec::with_capacity(CONTROL_LOOP_STATUS_SAMPLES);
    for sample_index in 0..CONTROL_LOOP_STATUS_SAMPLES {
        if sample_index != 0 {
            std::thread::sleep(Duration::from_millis(100));
        }
        let response = send_control_loop_packet(runtime, session, &encode_status_command())?;
        let status =
            ControlLoopStatus::decode(&response).map_err(ControlLoopProbeError::InvalidResponse)?;
        progress(format!(
            "step status-{sample_index}: ticks={} output={}",
            status.tick_count(),
            status.output()
        ));
        statuses.push(status);
    }

    let initial = statuses[0];
    let final_status = *statuses.last().expect("status sample count is non-zero");
    if final_status.setpoint() != CONTROL_LOOP_SETPOINT {
        return Err(ControlLoopProbeError::SetpointMismatch {
            expected: CONTROL_LOOP_SETPOINT,
            observed: final_status.setpoint(),
        });
    }
    if final_status.tick_count() <= initial.tick_count() {
        return Err(ControlLoopProbeError::TickDidNotAdvance {
            initial: initial.tick_count(),
            final_tick: final_status.tick_count(),
        });
    }

    Ok(ControlLoopProbeReport {
        target,
        statuses,
        elapsed: started.elapsed(),
    })
}

fn send_control_loop_packet(
    runtime: &Runtime,
    session: &mut VescSession,
    payload: &[u8],
) -> Result<Vec<u8>, ControlLoopProbeError> {
    session.clear_packet_state();
    let wire = build_custom_app_data_packet(payload);
    runtime
        .block_on(crate::package_transport::write_ble_uart_packet(
            &session.peripheral,
            &session.rx_char,
            &wire,
        ))
        .map_err(ControlLoopProbeError::Transport)?;
    session
        .receive_custom_app_data(CONTROL_LOOP_RESPONSE_TIMEOUT)
        .map_err(ControlLoopProbeError::Transport)
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
        .collect::<Vec<_>>()
        .join("");
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

/// Errors returned by the loopback probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployError {
    /// Installing or erasing the package over the transport failed.
    Transport(PackageInstallError),
    /// The post-deploy loopback smoke test failed.
    Loopback(LoopbackTransportError),
    /// The installed control-loop package probe failed.
    ControlLoop(ControlLoopProbeError),
}

impl std::fmt::Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "package install failed: {error}"),
            Self::Loopback(error) => write!(f, "loopback failed: {error}"),
            Self::ControlLoop(error) => write!(f, "control-loop probe failed: {error}"),
        }
    }
}

impl std::error::Error for DeployError {}
