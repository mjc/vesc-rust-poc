//! Verbose BLE loopback diagnostics for hardware bring-up.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use tokio::runtime::Builder;
use tokio::time;
use uuid::Uuid;
use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackPacket;

use crate::ble_discovery::{
    DiscoveryError, collect_discovered_peripherals, describe_discovered_peripheral,
    describe_loopback_target, find_matching_peripheral_with_progress, vesc_tool_scan_filter,
};
use crate::loopback::{LoopbackReport, LoopbackTarget, LoopbackTransportError};
use crate::vesc_uart::{PacketDecoder, encode_packet};

const VESC_BLE_UART_RX_UUID: Uuid = Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: Uuid = Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);
const COMM_FW_VERSION: u8 = 0;
const COMM_CUSTOM_APP_DATA: u8 = 36;
const COMM_LISP_PRINT: u8 = 135;
const COMM_LISP_REPL_CMD: u8 = 138;

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const BLE_WRITE_CHUNK_SIZE: usize = 20;
const POST_INSTALL_SETTLE: Duration = Duration::from_millis(1500);
const FW_VERSION_PREFLIGHT_ATTEMPTS: usize = 5;
const FW_VERSION_PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(2);
const FW_VERSION_PREFLIGHT_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Progress event emitted by the diagnostic loopback runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopbackProgress {
    /// The diagnostic runner is starting its BLE runtime.
    StartingRuntime,
    /// Device discovery and connection are starting for a target.
    OpeningSession {
        /// Human-readable target description.
        target: String,
    },
    /// BLE scanning started.
    ScanStarted {
        /// Scan timeout in seconds.
        timeout_secs: u64,
    },
    /// A BLE device was seen during scanning.
    ScanSeen {
        /// Human-readable device description.
        device: String,
        /// Whether this device matched the target selector.
        matched: bool,
    },
    /// A BLE device connection was established.
    Connected {
        /// Human-readable device description.
        device: String,
    },
    /// Services and characteristics were discovered from GATT.
    DiscoveredGatt {
        /// Discovered service UUIDs or names.
        services: Vec<String>,
        /// Discovered characteristic UUIDs or names.
        characteristics: Vec<String>,
    },
    /// BLE UART notifications are subscribed and the session is ready.
    SessionOpened,
    /// A named loopback step is starting.
    StepStarted {
        /// Step name, such as `ping` or `status`.
        step: &'static str,
    },
    /// A loopback request is being sent.
    Sending {
        /// Step name associated with the request.
        step: &'static str,
        /// Hex preview of the encoded wire packet.
        wire_hex: String,
        /// Encoded packet size in bytes.
        bytes: usize,
    },
    /// The runner is waiting for a reply for a step.
    WaitingForReply {
        /// Step name awaiting a reply.
        step: &'static str,
        /// Reply timeout in seconds.
        timeout_secs: u64,
    },
    /// A BLE notification arrived while waiting for a reply.
    ReceivedNotification {
        /// Step name active when the notification arrived.
        step: &'static str,
        /// Notification size in bytes.
        bytes: usize,
    },
    /// One or more VESC packets were decoded from notifications.
    DecodedPackets {
        /// Step name active when packets were decoded.
        step: &'static str,
        /// Number of decoded packets.
        count: usize,
    },
    /// A decoded packet was unrelated to loopback app data.
    NonAppDataPacket {
        /// Step name active when the unrelated packet arrived.
        step: &'static str,
        /// Human-readable packet summary.
        summary: String,
    },
    /// A loopback reply packet was decoded.
    ReplyReceived {
        /// Step name associated with the reply.
        step: &'static str,
        /// Hex preview of the reply payload.
        wire_hex: String,
        /// Decoded loopback command.
        command: WireCommand,
    },
    /// A loopback step completed successfully.
    StepSucceeded {
        /// Step name that completed.
        step: &'static str,
        /// Decoded response command for the step.
        command: WireCommand,
    },
    /// Reply waiting timed out.
    TimeoutWaiting {
        /// Step name that timed out.
        step: &'static str,
        /// Seconds elapsed before timeout.
        elapsed_secs: u64,
        /// Summaries of packets that were decoded but did not satisfy the step.
        pending: Vec<String>,
    },
}

impl LoopbackProgress {
    /// Returns whether this event should be printed in normal CLI output.
    pub fn should_print_to_cli(&self) -> bool {
        !matches!(
            self,
            Self::ReceivedNotification { .. } | Self::DecodedPackets { .. }
        )
    }

    /// Returns a human-readable description of this progress event.
    pub fn describe(&self) -> String {
        match self {
            Self::StartingRuntime => "starting BLE runtime".to_owned(),
            Self::OpeningSession { target } => format!("opening session for {target}"),
            Self::ScanStarted { timeout_secs } => {
                format!("scanning for BLE devices (timeout {timeout_secs}s)")
            }
            Self::ScanSeen { device, matched } => {
                if *matched {
                    format!("scan match: {device}")
                } else {
                    format!("scan seen: {device}")
                }
            }
            Self::Connected { device } => format!("connected to {device}"),
            Self::DiscoveredGatt {
                services,
                characteristics,
            } => format!(
                "discovered GATT: {} service(s), {} characteristic(s) [services={}; characteristics={}]",
                services.len(),
                characteristics.len(),
                services.join(", "),
                characteristics.join(", ")
            ),
            Self::SessionOpened => "BLE session open and notifications subscribed".to_owned(),
            Self::StepStarted { step } => format!("step {step}: starting"),
            Self::Sending {
                step,
                wire_hex,
                bytes,
            } => format!("step {step}: sending {bytes} byte(s) wire={wire_hex}"),
            Self::WaitingForReply { step, timeout_secs } => {
                if *step == "fw-version-preflight" {
                    format!("step {step}: waiting up to {timeout_secs}s for COMM_FW_VERSION reply")
                } else {
                    format!(
                        "step {step}: waiting up to {timeout_secs}s for COMM_CUSTOM_APP_DATA reply"
                    )
                }
            }
            Self::ReceivedNotification { step, bytes } => {
                format!("step {step}: received BLE notification ({bytes} bytes)")
            }
            Self::DecodedPackets { step, count } => {
                format!("step {step}: decoded {count} VESC packet(s)")
            }
            Self::NonAppDataPacket { step, summary } => {
                format!("step {step}: ignoring unrelated packet: {summary}")
            }
            Self::ReplyReceived {
                step,
                wire_hex,
                command,
            } => format!("step {step}: reply command={command:?} wire={wire_hex}"),
            Self::StepSucceeded { step, command } => {
                format!("step {step}: ok command={command:?}")
            }
            Self::TimeoutWaiting {
                step,
                elapsed_secs,
                pending,
            } => {
                if pending.is_empty() {
                    format!(
                        "step {step}: timed out after {elapsed_secs}s with no VESC packets received"
                    )
                } else {
                    format!(
                        "step {step}: timed out after {elapsed_secs}s; pending packets: {}",
                        pending.join("; ")
                    )
                }
            }
        }
    }
}

#[derive(Debug)]
struct DiagnosticSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    pending: VecDeque<Vec<u8>>,
}

/// Runs the loopback protocol over BLE while reporting detailed diagnostic progress.

pub fn run_loopback_with_diagnostics(
    target: LoopbackTarget,
    mut progress: impl FnMut(LoopbackProgress),
) -> Result<LoopbackReport, LoopbackTransportError> {
    progress(LoopbackProgress::StartingRuntime);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| device_error("failed to start the BLE runtime"))?;

    progress(LoopbackProgress::OpeningSession {
        target: describe_loopback_target(&target),
    });
    let mut session =
        runtime.block_on(open_session_with_progress(target.clone(), &mut progress))?;
    progress(LoopbackProgress::SessionOpened);
    std::thread::sleep(POST_INSTALL_SETTLE);

    runtime.block_on(run_fw_version_preflight(&mut session, &mut progress))?;
    session.clear_pending();

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
        session.clear_pending();
        progress(LoopbackProgress::StepStarted { step });
        let (payload, len) = packet.encode();
        let wire = build_custom_app_data_packet(&payload[..len]);
        progress(LoopbackProgress::Sending {
            step,
            wire_hex: hex_snippet(&wire, 48),
            bytes: wire.len(),
        });
        runtime.block_on(write_ble_uart_packet(
            &session.peripheral,
            &session.rx_char,
            &wire,
        ))?;

        let response = session.receive_loopback_reply_with_progress(step, &mut progress)?;
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

async fn open_session_with_progress(
    target: LoopbackTarget,
    progress: &mut impl FnMut(LoopbackProgress),
) -> Result<DiagnosticSession, LoopbackTransportError> {
    let manager = Manager::new()
        .await
        .map_err(|_| device_error("failed to initialize Bluetooth"))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|_| device_error("failed to enumerate Bluetooth adapters"))?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or(LoopbackTransportError::ScanTimeout)?;

    adapter
        .start_scan(vesc_tool_scan_filter())
        .await
        .map_err(|_| device_error("failed to start BLE scan"))?;

    progress(LoopbackProgress::ScanStarted {
        timeout_secs: SCAN_TIMEOUT.as_secs(),
    });

    let discovered = match time::timeout(
        SCAN_TIMEOUT,
        find_matching_peripheral_with_progress(&adapter, &target, |device, matched| {
            progress(LoopbackProgress::ScanSeen {
                device: describe_discovered_peripheral(&device),
                matched,
            });
        }),
    )
    .await
    {
        Ok(result) => result.map_err(map_discovery_error)?,
        Err(_) => return Err(scan_timeout_detail(&adapter, &target).await),
    };

    let peripheral = discovered;
    let device_label = peripheral
        .properties()
        .await
        .ok()
        .flatten()
        .map(|properties| {
            describe_discovered_peripheral(&crate::ble_discovery::DiscoveredPeripheral {
                identifier: properties.address.to_string(),
                local_name: properties.local_name,
                services: properties.services,
            })
        })
        .unwrap_or_else(|| "unknown device".to_owned());

    let _ = adapter.stop_scan().await;
    time::timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| LoopbackTransportError::ConnectFailed)?
        .map_err(|_| LoopbackTransportError::ConnectFailed)?;

    progress(LoopbackProgress::Connected {
        device: device_label,
    });

    time::timeout(CONNECT_TIMEOUT, peripheral.discover_services())
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?
        .map_err(|_| LoopbackTransportError::MissingService)?;

    let characteristics = peripheral.characteristics();
    let services: Vec<String> = peripheral
        .services()
        .iter()
        .map(|service| service.uuid.to_string())
        .collect();
    let characteristic_labels: Vec<String> = characteristics
        .iter()
        .map(|characteristic| characteristic.uuid.to_string())
        .collect();
    progress(LoopbackProgress::DiscoveredGatt {
        services: services.clone(),
        characteristics: characteristic_labels.clone(),
    });

    let rx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_RX_UUID)
        .cloned()
        .ok_or(LoopbackTransportError::MissingService)?;
    let tx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_TX_UUID)
        .cloned()
        .ok_or(LoopbackTransportError::MissingService)?;

    let (responses_tx, responses_rx) = mpsc::channel();
    let notification_uuid = tx_char.uuid;
    let mut notifications = peripheral
        .notifications()
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?;

    peripheral
        .subscribe(&tx_char)
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?;

    tokio::spawn(async move {
        while let Some(notification) = notifications.next().await {
            if notification.uuid == notification_uuid
                && responses_tx.send(notification.value).is_err()
            {
                break;
            }
        }
    });

    Ok(DiagnosticSession {
        peripheral,
        rx_char,
        responses: responses_rx,
        decoder: PacketDecoder::new(),
        pending: VecDeque::new(),
    })
}

impl DiagnosticSession {
    fn clear_pending(&mut self) {
        self.pending.clear();
        while self.decoder.pop_ready().is_some() {}
    }

    fn receive_vesc_packet_with_progress(
        &mut self,
        step: &'static str,
        expected_comm: u8,
        timeout: Duration,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Vec<u8>, LoopbackTransportError> {
        if let Some(packet) = self.take_pending_packet(expected_comm) {
            progress(LoopbackProgress::NonAppDataPacket {
                step,
                summary: describe_vesc_packet(&packet),
            });
            return Ok(packet);
        }

        progress(LoopbackProgress::WaitingForReply {
            step,
            timeout_secs: timeout.as_secs(),
        });

        let start = std::time::Instant::now();
        loop {
            if let Some(packet) = self.decoder.pop_ready() {
                if packet.first().copied() == Some(expected_comm) {
                    progress(LoopbackProgress::NonAppDataPacket {
                        step,
                        summary: describe_vesc_packet(&packet),
                    });
                    return Ok(packet);
                }

                progress(LoopbackProgress::NonAppDataPacket {
                    step,
                    summary: describe_vesc_packet(&packet),
                });
                self.pending.push_back(packet);
                continue;
            }

            match self.responses.recv_timeout(timeout) {
                Ok(bytes) => {
                    progress(LoopbackProgress::ReceivedNotification {
                        step,
                        bytes: bytes.len(),
                    });
                    let packets = self
                        .decoder
                        .push(&bytes)
                        .map_err(|_| device_error("failed to decode a VESC reply"))?;
                    progress(LoopbackProgress::DecodedPackets {
                        step,
                        count: packets.len(),
                    });
                    for packet in packets {
                        if packet.first().copied() == Some(expected_comm) {
                            progress(LoopbackProgress::NonAppDataPacket {
                                step,
                                summary: describe_vesc_packet(&packet),
                            });
                            return Ok(packet);
                        }

                        progress(LoopbackProgress::NonAppDataPacket {
                            step,
                            summary: describe_vesc_packet(&packet),
                        });
                        self.pending.push_back(packet);
                    }

                    if let Some(packet) = self.take_pending_packet(expected_comm) {
                        progress(LoopbackProgress::NonAppDataPacket {
                            step,
                            summary: describe_vesc_packet(&packet),
                        });
                        return Ok(packet);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    let pending: Vec<String> = self
                        .pending
                        .iter()
                        .map(|packet| describe_vesc_packet(packet))
                        .collect();
                    progress(LoopbackProgress::TimeoutWaiting {
                        step,
                        elapsed_secs: start.elapsed().as_secs(),
                        pending: pending.clone(),
                    });
                    return Err(timeout_error(step, start.elapsed(), &pending));
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(device_error("BLE notification stream disconnected"));
                }
            }
        }
    }

    fn take_pending_packet(&mut self, expected_comm: u8) -> Option<Vec<u8>> {
        let index = self
            .pending
            .iter()
            .position(|packet| packet.first().copied() == Some(expected_comm))?;
        self.pending.remove(index)
    }

    fn receive_loopback_reply_with_progress(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Vec<u8>, LoopbackTransportError> {
        if let Some(packet) = self.take_pending_response() {
            let wire_hex = hex_snippet(&packet, 32);
            let command = decode_loopback_command(&packet)?;
            progress(LoopbackProgress::ReplyReceived {
                step,
                wire_hex,
                command,
            });
            return Ok(packet);
        }

        progress(LoopbackProgress::WaitingForReply {
            step,
            timeout_secs: RESPONSE_TIMEOUT.as_secs(),
        });

        let start = std::time::Instant::now();
        loop {
            if let Some(packet) = self.decoder.pop_ready() {
                if packet.first().copied() == Some(COMM_CUSTOM_APP_DATA) {
                    let payload = packet[1..].to_vec();
                    let wire_hex = hex_snippet(&payload, 32);
                    let command = decode_loopback_command(&payload)?;
                    progress(LoopbackProgress::ReplyReceived {
                        step,
                        wire_hex,
                        command,
                    });
                    return Ok(payload);
                }

                progress(LoopbackProgress::NonAppDataPacket {
                    step,
                    summary: describe_vesc_packet(&packet),
                });
                self.pending.push_back(packet);
                continue;
            }

            match self.responses.recv_timeout(RESPONSE_TIMEOUT) {
                Ok(bytes) => {
                    progress(LoopbackProgress::ReceivedNotification {
                        step,
                        bytes: bytes.len(),
                    });
                    let packets = self
                        .decoder
                        .push(&bytes)
                        .map_err(|_| device_error("failed to decode a loopback reply"))?;
                    progress(LoopbackProgress::DecodedPackets {
                        step,
                        count: packets.len(),
                    });
                    for packet in packets {
                        if packet.first().copied() == Some(COMM_CUSTOM_APP_DATA) {
                            let payload = packet[1..].to_vec();
                            let wire_hex = hex_snippet(&payload, 32);
                            let command = decode_loopback_command(&payload)?;
                            progress(LoopbackProgress::ReplyReceived {
                                step,
                                wire_hex,
                                command,
                            });
                            return Ok(payload);
                        }

                        progress(LoopbackProgress::NonAppDataPacket {
                            step,
                            summary: describe_vesc_packet(&packet),
                        });
                        self.pending.push_back(packet);
                    }

                    if let Some(packet) = self.take_pending_response() {
                        let wire_hex = hex_snippet(&packet, 32);
                        let command = decode_loopback_command(&packet)?;
                        progress(LoopbackProgress::ReplyReceived {
                            step,
                            wire_hex,
                            command,
                        });
                        return Ok(packet);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    let pending: Vec<String> = self
                        .pending
                        .iter()
                        .map(|packet| describe_vesc_packet(packet))
                        .collect();
                    progress(LoopbackProgress::TimeoutWaiting {
                        step,
                        elapsed_secs: start.elapsed().as_secs(),
                        pending: pending.clone(),
                    });
                    return Err(timeout_error(step, start.elapsed(), &pending));
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(device_error("BLE notification stream disconnected"));
                }
            }
        }
    }

    fn take_pending_response(&mut self) -> Option<Vec<u8>> {
        let response_index = self
            .pending
            .iter()
            .position(|packet| packet.first().copied() == Some(COMM_CUSTOM_APP_DATA))?;
        let packet = self.pending.remove(response_index)?;
        Some(packet[1..].to_vec())
    }
}

async fn scan_timeout_detail(
    adapter: &btleplug::platform::Adapter,
    target: &LoopbackTarget,
) -> LoopbackTransportError {
    let devices = collect_discovered_peripherals(adapter)
        .await
        .unwrap_or_default();
    let listing: Vec<String> = devices.iter().map(describe_discovered_peripheral).collect();
    LoopbackTransportError::Device(format!(
        "scan timed out for {}; seen {} device(s): {}",
        describe_loopback_target(target),
        listing.len(),
        if listing.is_empty() {
            "<none>".to_owned()
        } else {
            listing.join(", ")
        }
    ))
}

fn timeout_error(step: &str, elapsed: Duration, pending: &[String]) -> LoopbackTransportError {
    if pending.is_empty() {
        LoopbackTransportError::Device(format!(
            "step {step}: timed out after {}s with no VESC packets received",
            elapsed.as_secs()
        ))
    } else {
        LoopbackTransportError::Device(format!(
            "step {step}: timed out after {}s; received instead: {}",
            elapsed.as_secs(),
            pending.join("; ")
        ))
    }
}

fn device_error(message: impl Into<String>) -> LoopbackTransportError {
    LoopbackTransportError::Device(message.into())
}

fn map_discovery_error(error: DiscoveryError) -> LoopbackTransportError {
    match error {
        DiscoveryError::InspectFailed => device_error("failed to inspect BLE peripherals"),
    }
}

fn build_custom_app_data_packet(payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(payload.len() + 1);
    data.push(COMM_CUSTOM_APP_DATA);
    data.extend_from_slice(payload);
    encode_packet(&data)
}

async fn run_fw_version_preflight(
    session: &mut DiagnosticSession,
    progress: &mut impl FnMut(LoopbackProgress),
) -> Result<(), LoopbackTransportError> {
    for attempt in 1..=FW_VERSION_PREFLIGHT_ATTEMPTS {
        progress(LoopbackProgress::StepStarted {
            step: "fw-version-preflight",
        });
        session.clear_pending();
        let wire = encode_packet(&[COMM_FW_VERSION]);
        progress(LoopbackProgress::Sending {
            step: "fw-version-preflight",
            wire_hex: hex_snippet(&wire, 48),
            bytes: wire.len(),
        });
        write_ble_uart_packet(&session.peripheral, &session.rx_char, &wire).await?;
        match session.receive_vesc_packet_with_progress(
            "fw-version-preflight",
            COMM_FW_VERSION,
            FW_VERSION_PREFLIGHT_TIMEOUT,
            progress,
        ) {
            Ok(_) => return Ok(()),
            Err(_error) if attempt < FW_VERSION_PREFLIGHT_ATTEMPTS => {
                progress(LoopbackProgress::TimeoutWaiting {
                    step: "fw-version-preflight",
                    elapsed_secs: FW_VERSION_PREFLIGHT_TIMEOUT.as_secs(),
                    pending: vec![format!(
                        "retry {attempt}/{FW_VERSION_PREFLIGHT_ATTEMPTS} after {}ms",
                        FW_VERSION_PREFLIGHT_RETRY_DELAY.as_millis()
                    )],
                });
                tokio::time::sleep(FW_VERSION_PREFLIGHT_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

async fn write_ble_uart_packet(
    peripheral: &Peripheral,
    rx_char: &Characteristic,
    packet: &[u8],
) -> Result<(), LoopbackTransportError> {
    for chunk in packet.chunks(BLE_WRITE_CHUNK_SIZE) {
        peripheral
            .write(rx_char, chunk, WriteType::WithoutResponse)
            .await
            .map_err(|_| device_error("failed to write BLE request"))?;
    }
    Ok(())
}

fn decode_loopback_command(payload: &[u8]) -> Result<WireCommand, LoopbackTransportError> {
    LoopbackPacket::decode(payload)
        .map(|packet| packet.frame().command())
        .map_err(LoopbackTransportError::Protocol)
}

/// Returns a concise human-readable summary of a decoded VESC UART packet payload.

pub fn describe_vesc_packet(packet: &[u8]) -> String {
    let command = packet.first().copied().unwrap_or(0xff);
    let payload = &packet[1..];
    match command {
        COMM_CUSTOM_APP_DATA => {
            format!("COMM_CUSTOM_APP_DATA payload={}", hex_snippet(payload, 24))
        }
        COMM_LISP_PRINT => {
            let end = payload
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(payload.len());
            format!(
                "COMM_LISP_PRINT {:?}",
                String::from_utf8_lossy(&payload[..end])
            )
        }
        COMM_LISP_REPL_CMD => format!("COMM_LISP_REPL_CMD ({} byte payload)", payload.len()),
        _ => format!("COMM_{command} payload={}", hex_snippet(payload, 16)),
    }
}

/// Formats at most `max_bytes` bytes as a compact hexadecimal preview.

pub fn hex_snippet(bytes: &[u8], max_bytes: usize) -> String {
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

#[cfg(test)]
mod tests {
    use super::{LoopbackProgress, describe_vesc_packet, hex_snippet, timeout_error};
    use crate::loopback::LoopbackTransportError;
    use crate::vesc_uart::encode_packet;
    use std::time::Duration;

    #[test]
    fn loopback_progress_describe_covers_key_events() {
        assert_eq!(
            LoopbackProgress::StepStarted { step: "ping" }.describe(),
            "step ping: starting"
        );
        assert_eq!(
            LoopbackProgress::WaitingForReply {
                step: "echo",
                timeout_secs: 8,
            }
            .describe(),
            "step echo: waiting up to 8s for COMM_CUSTOM_APP_DATA reply"
        );
        assert!(
            !LoopbackProgress::ReceivedNotification {
                step: "ping",
                bytes: 20
            }
            .should_print_to_cli()
        );
    }

    #[test]
    fn describe_vesc_packet_summarizes_common_commands() {
        let lisp_print = encode_packet(&[135, b'h', b'i', 0]);
        let decoded = crate::vesc_uart::PacketDecoder::new()
            .push(&lisp_print)
            .expect("packet")
            .pop()
            .expect("ready");
        assert!(describe_vesc_packet(&decoded).contains("COMM_LISP_PRINT"));

        let app_data = encode_packet(&[36, 1, 1, 0]);
        let decoded = crate::vesc_uart::PacketDecoder::new()
            .push(&app_data)
            .expect("packet")
            .pop()
            .expect("ready");
        assert!(describe_vesc_packet(&decoded).contains("COMM_CUSTOM_APP_DATA"));
    }

    #[test]
    fn timeout_error_includes_pending_packets_and_handler_hint() {
        let empty = timeout_error("ping", Duration::from_secs(8), &[]);
        assert!(
            matches!(empty, LoopbackTransportError::Device(ref msg) if msg.contains("no VESC packets received"))
        );

        let with_pending = timeout_error(
            "echo",
            Duration::from_secs(8),
            &["COMM_LISP_PRINT \"diag\"".to_owned()],
        );
        assert!(matches!(
            with_pending,
            LoopbackTransportError::Device(ref msg) if msg.contains("COMM_LISP_PRINT")
        ));
    }

    #[test]
    fn hex_snippet_truncates_long_buffers() {
        assert_eq!(hex_snippet(&[0x01, 0x02, 0x03], 2), "0102…");
    }
}
