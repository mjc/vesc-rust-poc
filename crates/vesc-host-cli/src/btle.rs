use crate::ble_discovery::{
    collect_discovered_peripherals, find_matching_peripheral, vesc_tool_scan_filter,
    DiscoveredPeripheral, DiscoveryError,
};
use crate::loopback::{LoopbackTarget, LoopbackTransport, LoopbackTransportError};
use crate::vesc_uart::{encode_packet, PacketDecoder};
use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::time;
use uuid::Uuid;

const VESC_BLE_UART_SERVICE_UUID: Uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_RX_UUID: Uuid = Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: Uuid = Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);
const COMM_CUSTOM_APP_DATA: u8 = 36;
const COMM_LISP_PRINT: u8 = 135;
const COMM_LISP_REPL_CMD: u8 = 138;

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const LISP_PROBE_TIMEOUT: Duration = Duration::from_secs(90);
const LISP_PROBE_PROGRESS_INTERVAL: Duration = Duration::from_secs(5);
const LISP_PROBE_QUIET_AFTER_PRINT: Duration = Duration::from_secs(2);
const LISP_PROBE_REPL_ATTEMPTS: usize = 5;
const LISP_PROBE_REPL_RETRY_DELAY: Duration = Duration::from_secs(1);
const BLE_WRITE_CHUNK_SIZE: usize = 20;

#[derive(Debug)]
struct BtleSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    pending: VecDeque<Vec<u8>>,
}

#[derive(Debug)]
pub struct BtleLoopbackTransport {
    runtime: Runtime,
    session: RefCell<Option<BtleSession>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProbeReport {
    prints: Vec<String>,
    attempts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LispProbeProgress {
    StartingRuntime,
    OpeningSession,
    SessionOpened,
    SendingReplCommand {
        attempt: usize,
        bytes: usize,
    },
    WaitingForPrints,
    ReceivedNotification {
        bytes: usize,
    },
    DecodedPackets {
        count: usize,
    },
    LispPrint {
        line: String,
    },
    StillWaiting {
        elapsed_secs: u64,
        prints: usize,
    },
    QuietAfterPrints {
        prints: usize,
    },
    ExpectedResultReceived,
    RetryingReplCommand {
        next_attempt: usize,
        delay_secs: u64,
    },
    FinishedWaiting {
        prints: usize,
    },
}

impl LispProbeProgress {
    pub fn should_print_to_cli(&self) -> bool {
        !matches!(
            self,
            Self::ReceivedNotification { .. } | Self::DecodedPackets { .. }
        )
    }

    pub fn describe(&self) -> String {
        match self {
            Self::StartingRuntime => "starting BLE runtime".to_owned(),
            Self::OpeningSession => "scanning/connecting to VESC BLE UART".to_owned(),
            Self::SessionOpened => "BLE session open and notifications subscribed".to_owned(),
            Self::SendingReplCommand { attempt, bytes } => {
                format!("sending Lisp REPL probe packet attempt {attempt} ({bytes} bytes)")
            }
            Self::WaitingForPrints => "waiting for Lisp print replies".to_owned(),
            Self::ReceivedNotification { bytes } => {
                format!("received BLE notification ({bytes} bytes)")
            }
            Self::DecodedPackets { count } => {
                format!("decoded {count} VESC packet(s) from notifications")
            }
            Self::LispPrint { line } => format!("lisp print: {line}"),
            Self::StillWaiting {
                elapsed_secs,
                prints,
            } => format!(
                "still waiting for Lisp print replies after {elapsed_secs}s ({prints} print(s) so far)"
            ),
            Self::QuietAfterPrints { prints } => {
                format!("no more Lisp print replies after quiet period ({prints} print(s))")
            }
            Self::ExpectedResultReceived => "received expected Lisp probe result".to_owned(),
            Self::RetryingReplCommand {
                next_attempt,
                delay_secs,
            } => format!(
                "retrying Lisp REPL probe as attempt {next_attempt} after {delay_secs}s"
            ),
            Self::FinishedWaiting { prints } => {
                format!("finished waiting for Lisp print replies ({prints} print(s))")
            }
        }
    }
}

impl LispProbeReport {
    pub fn prints(&self) -> &[String] {
        &self.prints
    }

    pub fn attempts(&self) -> usize {
        self.attempts
    }
}

impl BtleLoopbackTransport {
    pub fn new() -> Result<Self, LoopbackTransportError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .map_err(|_| LoopbackTransportError::Device("failed to start the BLE runtime"))?;

        Ok(Self {
            runtime,
            session: RefCell::new(None),
        })
    }

    fn open_session(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        let session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        *self.session.borrow_mut() = Some(session);
        Ok(())
    }

    fn session(
        &self,
    ) -> Result<std::cell::RefMut<'_, Option<BtleSession>>, LoopbackTransportError> {
        if self.session.borrow().is_none() {
            return Err(LoopbackTransportError::Device(
                "BLE transport has not been opened",
            ));
        }

        Ok(self.session.borrow_mut())
    }
}

impl LoopbackTransport for BtleLoopbackTransport {
    fn open(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        self.open_session(target)
    }

    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError> {
        let mut session = self.session()?;
        let session = session.as_mut().expect("session checked above");
        self.runtime
            .block_on(write_ble_uart_packet(
                &session.peripheral,
                &session.rx_char,
                &build_custom_app_data_packet(request),
            ))
            .map_err(|_| LoopbackTransportError::Device("failed to write BLE request"))?;
        session.runtime_receive()
    }
}

impl BtleSession {
    fn runtime_receive(&mut self) -> Result<Vec<u8>, LoopbackTransportError> {
        if let Some(packet) = self.take_pending_response() {
            return Ok(packet);
        }

        loop {
            if let Some(packet) = self.decoder.pop_ready() {
                if packet.first().copied() == Some(COMM_CUSTOM_APP_DATA) {
                    return Ok(packet[1..].to_vec());
                }

                self.pending.push_back(packet);
                continue;
            }

            let bytes = self.responses.recv_timeout(RESPONSE_TIMEOUT).map_err(|_| {
                LoopbackTransportError::Device("timed out waiting for a loopback reply")
            })?;

            let packets = self
                .decoder
                .push(&bytes)
                .map_err(|_| LoopbackTransportError::Device("failed to decode a loopback reply"))?;
            for packet in packets {
                if packet.first().copied() == Some(COMM_CUSTOM_APP_DATA) {
                    return Ok(packet[1..].to_vec());
                }

                self.pending.push_back(packet);
            }

            if let Some(packet) = self.take_pending_response() {
                return Ok(packet);
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

    fn receive_lisp_prints_with_progress(
        &mut self,
        mut progress: impl FnMut(LispProbeProgress),
    ) -> Result<Vec<String>, LoopbackTransportError> {
        let mut prints = self.take_pending_lisp_prints();
        progress(LispProbeProgress::WaitingForPrints);
        let start = std::time::Instant::now();
        let deadline = start + LISP_PROBE_TIMEOUT;
        let mut next_progress = start + LISP_PROBE_PROGRESS_INTERVAL;
        let mut quiet_deadline = (!prints.is_empty()).then(|| start + LISP_PROBE_QUIET_AFTER_PRINT);

        while std::time::Instant::now() < deadline {
            let now = std::time::Instant::now();
            let mut wait_until = deadline.min(next_progress);
            if let Some(quiet_deadline) = quiet_deadline {
                wait_until = wait_until.min(quiet_deadline);
            }
            let remaining = wait_until.saturating_duration_since(now);
            match self.responses.recv_timeout(remaining) {
                Ok(bytes) => {
                    quiet_deadline = None;
                    progress(LispProbeProgress::ReceivedNotification { bytes: bytes.len() });
                    let packets = self.decoder.push(&bytes).map_err(|_| {
                        LoopbackTransportError::Device("failed to decode a Lisp probe reply")
                    })?;
                    progress(LispProbeProgress::DecodedPackets {
                        count: packets.len(),
                    });
                    packets
                        .into_iter()
                        .for_each(|packet| self.pending.push_back(packet));
                    let new_prints = self.take_pending_lisp_prints();
                    for line in &new_prints {
                        progress(LispProbeProgress::LispPrint { line: line.clone() });
                    }
                    prints.extend(new_prints);
                    if prints.iter().any(|line| lisp_probe_line_is_success(line)) {
                        progress(LispProbeProgress::ExpectedResultReceived);
                        break;
                    }
                    if !prints.is_empty() {
                        quiet_deadline =
                            Some(std::time::Instant::now() + LISP_PROBE_QUIET_AFTER_PRINT);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    let now = std::time::Instant::now();
                    if quiet_deadline.is_some_and(|deadline| now >= deadline) {
                        progress(LispProbeProgress::QuietAfterPrints {
                            prints: prints.len(),
                        });
                        break;
                    }
                    if now >= deadline {
                        break;
                    }
                    progress(LispProbeProgress::StillWaiting {
                        elapsed_secs: start.elapsed().as_secs(),
                        prints: prints.len(),
                    });
                    next_progress = std::time::Instant::now() + LISP_PROBE_PROGRESS_INTERVAL;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(LoopbackTransportError::Device(
                        "BLE notification stream disconnected",
                    ));
                }
            }
        }

        progress(LispProbeProgress::FinishedWaiting {
            prints: prints.len(),
        });
        Ok(prints)
    }

    fn take_pending_lisp_prints(&mut self) -> Vec<String> {
        let mut prints = Vec::new();
        let mut retained = VecDeque::new();

        while let Some(packet) = self.pending.pop_front() {
            if packet.first().copied() == Some(COMM_LISP_PRINT) {
                prints.push(parse_lisp_print(&packet[1..]));
            } else {
                retained.push_back(packet);
            }
        }

        self.pending = retained;
        prints
    }
}

async fn open_session(target: LoopbackTarget) -> Result<BtleSession, LoopbackTransportError> {
    let manager = Manager::new()
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to initialize Bluetooth"))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to enumerate Bluetooth adapters"))?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or(LoopbackTransportError::ScanTimeout)?;

    adapter
        .start_scan(vesc_tool_scan_filter())
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to start BLE scan"))?;

    let discovered = time::timeout(SCAN_TIMEOUT, find_matching_peripheral(&adapter, &target))
        .await
        .map_err(|_| LoopbackTransportError::ScanTimeout)?
        .map_err(map_discovery_error)?;

    let peripheral = discovered;
    let _ = adapter.stop_scan().await;
    time::timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| LoopbackTransportError::ConnectFailed)?
        .map_err(|_| LoopbackTransportError::ConnectFailed)?;
    time::timeout(CONNECT_TIMEOUT, peripheral.discover_services())
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?
        .map_err(|_| LoopbackTransportError::MissingService)?;

    let characteristics = peripheral.characteristics();
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

    Ok(BtleSession {
        peripheral,
        rx_char,
        responses: responses_rx,
        decoder: PacketDecoder::new(),
        pending: VecDeque::new(),
    })
}

fn build_custom_app_data_packet(payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(payload.len() + 1);
    data.push(COMM_CUSTOM_APP_DATA);
    data.extend_from_slice(payload);
    encode_packet(&data)
}

fn build_lisp_repl_packet(command: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(command.len() + 2);
    data.push(COMM_LISP_REPL_CMD);
    data.extend_from_slice(command.as_bytes());
    data.push(0);
    encode_packet(&data)
}

fn parse_lisp_print(payload: &[u8]) -> String {
    let end = payload
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(payload.len());
    String::from_utf8_lossy(&payload[..end]).into_owned()
}

fn lisp_probe_command() -> &'static str {
    r#"(progn
    (print "vesc-rust-probe-v24")
    (match (trap (ext-c-probe-v12 14))
        ((exit-ok (? v)) (if (= v 42) (print "vesc-rust-probe-ok-42") (print v)))
        ((exit-error (? e)) (print e))))"#
}

fn lisp_probe_report(prints: Vec<String>, attempts: usize) -> LispProbeReport {
    LispProbeReport { prints, attempts }
}

pub fn run_lisp_probe(target: LoopbackTarget) -> Result<LispProbeReport, LoopbackTransportError> {
    run_lisp_probe_with_progress(target, |_| {})
}

pub fn run_lisp_probe_with_progress(
    target: LoopbackTarget,
    mut progress: impl FnMut(LispProbeProgress),
) -> Result<LispProbeReport, LoopbackTransportError> {
    progress(LispProbeProgress::StartingRuntime);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| LoopbackTransportError::Device("failed to start the BLE runtime"))?;

    progress(LispProbeProgress::OpeningSession);
    let mut session = runtime.block_on(open_session(target))?;
    progress(LispProbeProgress::SessionOpened);
    let packet = build_lisp_repl_packet(lisp_probe_command());
    let mut all_prints = Vec::new();
    let mut attempts = 0;

    for attempt in 1..=LISP_PROBE_REPL_ATTEMPTS {
        attempts = attempt;
        progress(LispProbeProgress::SendingReplCommand {
            attempt,
            bytes: packet.len(),
        });
        runtime.block_on(write_ble_uart_packet(
            &session.peripheral,
            &session.rx_char,
            &packet,
        ))?;
        let prints = session.receive_lisp_prints_with_progress(&mut progress)?;
        let expected_result = lisp_probe_has_expected_result(&prints);
        all_prints.extend(prints);

        if expected_result || lisp_probe_has_expected_result(&all_prints) {
            break;
        }

        if attempt < LISP_PROBE_REPL_ATTEMPTS {
            progress(LispProbeProgress::RetryingReplCommand {
                next_attempt: attempt + 1,
                delay_secs: LISP_PROBE_REPL_RETRY_DELAY.as_secs(),
            });
            std::thread::sleep(LISP_PROBE_REPL_RETRY_DELAY);
        }
    }

    Ok(lisp_probe_report(all_prints, attempts))
}

fn lisp_probe_has_expected_result(prints: &[String]) -> bool {
    prints.iter().any(|line| lisp_probe_line_is_success(line))
}

fn lisp_probe_line_is_success(line: &str) -> bool {
    line.contains("vesc-rust-probe-ok-42")
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
            .map_err(|_| LoopbackTransportError::Device("failed to write BLE request"))?;
    }
    Ok(())
}

pub fn scan_devices() -> Result<Vec<DiscoveredPeripheral>, LoopbackTransportError> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| LoopbackTransportError::Device("failed to start the BLE runtime"))?;

    runtime.block_on(async move {
        let manager = Manager::new()
            .await
            .map_err(|_| LoopbackTransportError::Device("failed to initialize Bluetooth"))?;
        let adapters = manager.adapters().await.map_err(|_| {
            LoopbackTransportError::Device("failed to enumerate Bluetooth adapters")
        })?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or(LoopbackTransportError::ScanTimeout)?;

        adapter
            .start_scan(vesc_tool_scan_filter())
            .await
            .map_err(|_| LoopbackTransportError::Device("failed to start BLE scan"))?;

        time::sleep(SCAN_TIMEOUT).await;
        let devices = collect_discovered_peripherals(&adapter)
            .await
            .map_err(map_discovery_error)?;
        let _ = adapter.stop_scan().await;
        Ok(devices)
    })
}

fn map_discovery_error(error: DiscoveryError) -> LoopbackTransportError {
    match error {
        DiscoveryError::InspectFailed => {
            LoopbackTransportError::Device("failed to inspect BLE peripherals")
        }
    }
}

pub fn vesc_ble_uart_service_uuid() -> Uuid {
    VESC_BLE_UART_SERVICE_UUID
}

pub fn vesc_ble_uart_rx_uuid() -> Uuid {
    VESC_BLE_UART_RX_UUID
}

pub fn vesc_ble_uart_tx_uuid() -> Uuid {
    VESC_BLE_UART_TX_UUID
}

#[cfg(test)]
mod tests {
    use super::{
        build_custom_app_data_packet, build_lisp_repl_packet, lisp_probe_command,
        lisp_probe_report, parse_lisp_print, vesc_ble_uart_rx_uuid, vesc_ble_uart_service_uuid,
        vesc_ble_uart_tx_uuid, LispProbeProgress, COMM_CUSTOM_APP_DATA, COMM_LISP_REPL_CMD,
    };
    use crate::vesc_uart::PacketDecoder;

    #[test]
    fn exports_the_vesc_ble_uart_profile_uuids() {
        assert_eq!(
            vesc_ble_uart_service_uuid().to_string(),
            "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
        );
        assert_eq!(
            vesc_ble_uart_rx_uuid().to_string(),
            "6e400002-b5a3-f393-e0a9-e50e24dcca9e"
        );
        assert_eq!(
            vesc_ble_uart_tx_uuid().to_string(),
            "6e400003-b5a3-f393-e0a9-e50e24dcca9e"
        );
    }

    #[test]
    fn wraps_loopback_requests_in_custom_app_data_packets() {
        let packet = build_custom_app_data_packet(&[1, 2, 3]);
        let decoded = PacketDecoder::new()
            .push(&packet)
            .expect("decoded packet")
            .pop()
            .expect("complete packet");

        assert_eq!(decoded, vec![COMM_CUSTOM_APP_DATA, 1, 2, 3]);
    }

    #[test]
    fn wraps_lisp_probe_commands_in_repl_packets() {
        let command = lisp_probe_command();
        assert!(command.contains("vesc-rust-probe-v24"));
        assert!(
            !command.contains("load-native-lib"),
            "lisp-probe should exercise the already-loaded package extensions, not retry native loading"
        );
        assert!(
            !command.contains("(sleep"),
            "VESC Tool sends REPL expressions directly; host-side progress handles waiting"
        );
        assert!(command.contains("(trap (ext-c-probe-v12 14))"));
        assert!(command.contains("vesc-rust-probe-ok-42"));

        let packet = build_lisp_repl_packet(command);
        let decoded = PacketDecoder::new()
            .push(&packet)
            .expect("valid packet")
            .pop()
            .expect("complete packet");

        let mut expected = Vec::with_capacity(command.len() + 2);
        expected.push(COMM_LISP_REPL_CMD);
        expected.extend_from_slice(command.as_bytes());
        expected.push(0);
        assert_eq!(decoded, expected);
        assert_eq!(decoded[0], COMM_LISP_REPL_CMD);
    }

    #[test]
    fn lisp_probe_reports_empty_print_attempts_to_the_outer_loop() {
        assert!(lisp_probe_report(Vec::new(), 1).prints().is_empty());
        assert_eq!(lisp_probe_report(Vec::new(), 1).attempts(), 1);
    }

    #[test]
    fn lisp_probe_progress_messages_explain_wait_state() {
        assert_eq!(
            LispProbeProgress::StillWaiting {
                elapsed_secs: 15,
                prints: 0,
            }
            .describe(),
            "still waiting for Lisp print replies after 15s (0 print(s) so far)"
        );
        assert_eq!(
            LispProbeProgress::SendingReplCommand {
                attempt: 2,
                bytes: 42,
            }
            .describe(),
            "sending Lisp REPL probe packet attempt 2 (42 bytes)"
        );
        assert_eq!(
            LispProbeProgress::QuietAfterPrints { prints: 1 }.describe(),
            "no more Lisp print replies after quiet period (1 print(s))"
        );
        assert_eq!(
            LispProbeProgress::RetryingReplCommand {
                next_attempt: 3,
                delay_secs: 1,
            }
            .describe(),
            "retrying Lisp REPL probe as attempt 3 after 1s"
        );
        assert!(!LispProbeProgress::ReceivedNotification { bytes: 20 }.should_print_to_cli());
        assert!(!LispProbeProgress::DecodedPackets { count: 1 }.should_print_to_cli());
        assert!(LispProbeProgress::WaitingForPrints.should_print_to_cli());
        assert_eq!(
            LispProbeProgress::LispPrint {
                line: "vesc-rust-probe-v24".to_owned(),
            }
            .describe(),
            "lisp print: vesc-rust-probe-v24"
        );
    }

    #[test]
    fn lisp_probe_stops_retrying_once_prints_arrive() {
        let prints = vec!["vesc-rust-probe-ok-42".to_owned()];

        assert_eq!(lisp_probe_report(prints.clone(), 1).prints(), prints);
        assert_eq!(lisp_probe_report(prints, 1).attempts(), 1);
    }

    #[test]
    fn parses_lisp_print_packets_as_lossy_strings() {
        assert_eq!(parse_lisp_print(b"42\0ignored"), "42");
        assert_eq!(parse_lisp_print(&[0xff, b'a']), "\u{fffd}a");
    }
}
