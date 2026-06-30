use crate::ble_discovery::{
    DiscoveredPeripheral, DiscoveryError, collect_discovered_peripherals, find_matching_peripheral,
    vesc_tool_scan_filter,
};
use crate::loopback::{LoopbackTarget, LoopbackTransport, LoopbackTransportError};
use crate::vesc_uart::{PacketDecoder, encode_packet};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LispPrintReceiveConfig {
    timeout: Duration,
    quiet_after_print: Duration,
    progress_interval: Duration,
}

impl Default for LispPrintReceiveConfig {
    fn default() -> Self {
        Self {
            timeout: LISP_PROBE_TIMEOUT,
            quiet_after_print: LISP_PROBE_QUIET_AFTER_PRINT,
            progress_interval: LISP_PROBE_PROGRESS_INTERVAL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContinuousProbeStep {
    ContinueImmediately { next_attempt: usize },
    RetryAfterDelay { next_attempt: usize },
}

fn continuous_probe_step(attempt: usize, prints: &[String]) -> ContinuousProbeStep {
    if lisp_probe_has_expected_result(prints) {
        return ContinuousProbeStep::ContinueImmediately { next_attempt: 1 };
    }

    if attempt >= LISP_PROBE_REPL_ATTEMPTS {
        ContinuousProbeStep::RetryAfterDelay { next_attempt: 1 }
    } else {
        ContinuousProbeStep::RetryAfterDelay {
            next_attempt: attempt + 1,
        }
    }
}

#[derive(Debug)]
struct BtleSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    pending: VecDeque<Vec<u8>>,
}

/// BLE UART-backed transport for loopback protocol exchanges.
#[derive(Debug)]
pub struct BtleLoopbackTransport {
    runtime: Runtime,
    session: RefCell<Option<BtleSession>>,
}

/// Summary returned by a Lisp probe diagnostic run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProbeReport {
    prints: Vec<String>,
    attempts: usize,
}

/// Progress event emitted while running the Lisp probe diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LispProbeProgress {
    /// The Tokio runtime is being started.
    StartingRuntime,
    /// Device discovery and BLE connection are starting.
    OpeningSession,
    /// BLE notifications are subscribed and the session is ready.
    SessionOpened,
    /// A Lisp REPL command packet is being sent.
    SendingReplCommand {
        /// One-based send attempt number.
        attempt: usize,
        /// Encoded packet size in bytes.
        bytes: usize,
    },
    /// The probe is waiting for Lisp print replies.
    WaitingForPrints,
    /// A BLE notification was received.
    ReceivedNotification {
        /// Notification size in bytes.
        bytes: usize,
    },
    /// One or more VESC UART packets were decoded.
    DecodedPackets {
        /// Number of decoded packets.
        count: usize,
    },
    /// A Lisp print line was decoded.
    LispPrint {
        /// Printed Lisp line.
        line: String,
    },
    /// The probe is still waiting for more print replies.
    StillWaiting {
        /// Seconds elapsed while waiting.
        elapsed_secs: u64,
        /// Number of print lines observed so far.
        prints: usize,
    },
    /// The probe saw print output and then a quiet period.
    QuietAfterPrints {
        /// Number of print lines observed.
        prints: usize,
    },
    /// The expected Lisp probe result was observed.
    ExpectedResultReceived,
    /// The probe will retry the REPL command after a delay.
    RetryingReplCommand {
        /// One-based attempt number that will be sent next.
        next_attempt: usize,
        /// Delay before the next attempt in seconds.
        delay_secs: u64,
    },
    /// The wait loop finished with the collected print count.
    FinishedWaiting {
        /// Number of print lines observed.
        prints: usize,
    },
}

impl LispProbeProgress {
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
            } => format!("retrying Lisp REPL probe as attempt {next_attempt} after {delay_secs}s"),
            Self::FinishedWaiting { prints } => {
                format!("finished waiting for Lisp print replies ({prints} print(s))")
            }
        }
    }
}

impl LispProbeReport {
    /// Returns Lisp print lines collected during the probe.
    pub fn prints(&self) -> &[String] {
        &self.prints
    }

    /// Returns how many REPL send attempts were needed.
    pub fn attempts(&self) -> usize {
        self.attempts
    }
}

impl BtleLoopbackTransport {
    /// Creates a BLE loopback transport with its own single-worker runtime.
    pub fn new() -> Result<Self, LoopbackTransportError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .map_err(|_| {
                LoopbackTransportError::Device("failed to start the BLE runtime".to_owned())
            })?;

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
                "BLE transport has not been opened".to_owned(),
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
            .map_err(|_| {
                LoopbackTransportError::Device("failed to write BLE request".to_owned())
            })?;
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
                LoopbackTransportError::Device("timed out waiting for a loopback reply".to_owned())
            })?;

            let packets = self.decoder.push(&bytes).map_err(|_| {
                LoopbackTransportError::Device("failed to decode a loopback reply".to_owned())
            })?;
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
        progress: impl FnMut(LispProbeProgress),
    ) -> Result<Vec<String>, LoopbackTransportError> {
        receive_lisp_prints_from_channel(
            &self.responses,
            &mut self.decoder,
            &mut self.pending,
            LispPrintReceiveConfig::default(),
            progress,
        )
    }
}

async fn open_session(target: LoopbackTarget) -> Result<BtleSession, LoopbackTransportError> {
    let manager = Manager::new()
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to initialize Bluetooth".to_owned()))?;
    let adapters = manager.adapters().await.map_err(|_| {
        LoopbackTransportError::Device("failed to enumerate Bluetooth adapters".to_owned())
    })?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or(LoopbackTransportError::ScanTimeout)?;

    adapter
        .start_scan(vesc_tool_scan_filter())
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to start BLE scan".to_owned()))?;

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
    (print "vesc-rust-probe-rust-diag-v4")
    (match (trap (ext-rust-probe-diag-v4 14))
        ((exit-ok (? v)) (if (= v 42) (print "vesc-rust-probe-ok-42") (print v)))
        ((exit-error (? e)) (print e))))"#
}

fn lisp_probe_report(prints: Vec<String>, attempts: usize) -> LispProbeReport {
    LispProbeReport { prints, attempts }
}

/// Runs the Lisp probe diagnostic and returns collected print output.

pub fn run_lisp_probe(target: LoopbackTarget) -> Result<LispProbeReport, LoopbackTransportError> {
    run_lisp_probe_with_progress(target, |_| {})
}

/// Runs the Lisp probe diagnostic while reporting progress events.

pub fn run_lisp_probe_with_progress(
    target: LoopbackTarget,
    mut progress: impl FnMut(LispProbeProgress),
) -> Result<LispProbeReport, LoopbackTransportError> {
    progress(LispProbeProgress::StartingRuntime);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| {
            LoopbackTransportError::Device("failed to start the BLE runtime".to_owned())
        })?;

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

/// Repeatedly runs the Lisp probe diagnostic until the expected result or retry limit is reached.

pub fn run_lisp_probe_continuously_with_progress(
    target: LoopbackTarget,
    mut progress: impl FnMut(LispProbeProgress),
) -> Result<(), LoopbackTransportError> {
    progress(LispProbeProgress::StartingRuntime);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| {
            LoopbackTransportError::Device("failed to start the BLE runtime".to_owned())
        })?;

    progress(LispProbeProgress::OpeningSession);
    let mut session = runtime.block_on(open_session(target))?;
    progress(LispProbeProgress::SessionOpened);
    let packet = build_lisp_repl_packet(lisp_probe_command());
    let mut attempt = 1;

    loop {
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

        match continuous_probe_step(attempt, &prints) {
            ContinuousProbeStep::ContinueImmediately { next_attempt } => {
                attempt = next_attempt;
                continue;
            }
            ContinuousProbeStep::RetryAfterDelay { next_attempt } => {
                progress(LispProbeProgress::RetryingReplCommand {
                    next_attempt,
                    delay_secs: LISP_PROBE_REPL_RETRY_DELAY.as_secs(),
                });
                attempt = next_attempt;
                std::thread::sleep(LISP_PROBE_REPL_RETRY_DELAY);
            }
        }
    }
}

fn receive_lisp_prints_from_channel(
    responses: &Receiver<Vec<u8>>,
    decoder: &mut PacketDecoder,
    pending: &mut VecDeque<Vec<u8>>,
    config: LispPrintReceiveConfig,
    mut progress: impl FnMut(LispProbeProgress),
) -> Result<Vec<String>, LoopbackTransportError> {
    let mut prints = take_pending_lisp_prints(pending);
    progress(LispProbeProgress::WaitingForPrints);
    let start = std::time::Instant::now();
    let deadline = start + config.timeout;
    let mut next_progress = start + config.progress_interval;
    let mut quiet_deadline = (!prints.is_empty()).then(|| start + config.quiet_after_print);

    while std::time::Instant::now() < deadline {
        let now = std::time::Instant::now();
        let mut wait_until = deadline.min(next_progress);
        if let Some(quiet_deadline) = quiet_deadline {
            wait_until = wait_until.min(quiet_deadline);
        }
        let remaining = wait_until.saturating_duration_since(now);
        match responses.recv_timeout(remaining) {
            Ok(bytes) => {
                quiet_deadline = None;
                progress(LispProbeProgress::ReceivedNotification { bytes: bytes.len() });
                let packets = decoder.push(&bytes).map_err(|_| {
                    LoopbackTransportError::Device("failed to decode a Lisp probe reply".to_owned())
                })?;
                progress(LispProbeProgress::DecodedPackets {
                    count: packets.len(),
                });
                packets
                    .into_iter()
                    .for_each(|packet| pending.push_back(packet));
                let new_prints = take_pending_lisp_prints(pending);
                for line in &new_prints {
                    progress(LispProbeProgress::LispPrint { line: line.clone() });
                }
                prints.extend(new_prints);
                if prints.iter().any(|line| lisp_probe_line_is_success(line)) {
                    progress(LispProbeProgress::ExpectedResultReceived);
                    break;
                }
                if !prints.is_empty() {
                    quiet_deadline = Some(std::time::Instant::now() + config.quiet_after_print);
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
                next_progress = std::time::Instant::now() + config.progress_interval;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(LoopbackTransportError::Device(
                    "BLE notification stream disconnected".to_owned(),
                ));
            }
        }
    }

    progress(LispProbeProgress::FinishedWaiting {
        prints: prints.len(),
    });
    Ok(prints)
}

fn take_pending_lisp_prints(pending: &mut VecDeque<Vec<u8>>) -> Vec<String> {
    let mut prints = Vec::new();
    let mut retained = VecDeque::new();

    while let Some(packet) = pending.pop_front() {
        if packet.first().copied() == Some(COMM_LISP_PRINT) {
            prints.push(parse_lisp_print(&packet[1..]));
        } else {
            retained.push_back(packet);
        }
    }

    *pending = retained;
    prints
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
            .map_err(|_| {
                LoopbackTransportError::Device("failed to write BLE request".to_owned())
            })?;
    }
    Ok(())
}

/// Scans for BLE peripherals that expose VESC BLE UART characteristics.

pub fn scan_devices() -> Result<Vec<DiscoveredPeripheral>, LoopbackTransportError> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| {
            LoopbackTransportError::Device("failed to start the BLE runtime".to_owned())
        })?;

    runtime.block_on(async move {
        let manager = Manager::new().await.map_err(|_| {
            LoopbackTransportError::Device("failed to initialize Bluetooth".to_owned())
        })?;
        let adapters = manager.adapters().await.map_err(|_| {
            LoopbackTransportError::Device("failed to enumerate Bluetooth adapters".to_owned())
        })?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or(LoopbackTransportError::ScanTimeout)?;

        adapter
            .start_scan(vesc_tool_scan_filter())
            .await
            .map_err(|_| LoopbackTransportError::Device("failed to start BLE scan".to_owned()))?;

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
            LoopbackTransportError::Device("failed to inspect BLE peripherals".to_owned())
        }
    }
}

/// Returns the VESC BLE UART service UUID.

pub fn vesc_ble_uart_service_uuid() -> Uuid {
    VESC_BLE_UART_SERVICE_UUID
}

/// Returns the VESC BLE UART RX characteristic UUID.

pub fn vesc_ble_uart_rx_uuid() -> Uuid {
    VESC_BLE_UART_RX_UUID
}

/// Returns the VESC BLE UART TX characteristic UUID.

pub fn vesc_ble_uart_tx_uuid() -> Uuid {
    VESC_BLE_UART_TX_UUID
}

#[cfg(test)]
mod tests {
    use super::{
        COMM_CUSTOM_APP_DATA, COMM_LISP_PRINT, COMM_LISP_REPL_CMD, ContinuousProbeStep,
        LispPrintReceiveConfig, LispProbeProgress, build_custom_app_data_packet,
        build_lisp_repl_packet, continuous_probe_step, lisp_probe_command,
        lisp_probe_has_expected_result, lisp_probe_line_is_success, lisp_probe_report,
        parse_lisp_print, receive_lisp_prints_from_channel, take_pending_lisp_prints,
        vesc_ble_uart_rx_uuid, vesc_ble_uart_service_uuid, vesc_ble_uart_tx_uuid,
    };
    use crate::loopback::LoopbackTransportError;
    use crate::vesc_uart::{PacketDecoder, encode_packet};
    use std::collections::VecDeque;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    fn fast_lisp_print_receive_config() -> LispPrintReceiveConfig {
        LispPrintReceiveConfig {
            timeout: Duration::from_millis(200),
            quiet_after_print: Duration::from_millis(10),
            progress_interval: Duration::from_secs(5),
        }
    }

    fn build_lisp_print_notification(line: &str) -> Vec<u8> {
        let mut payload = vec![COMM_LISP_PRINT];
        payload.extend_from_slice(line.as_bytes());
        payload.push(0);
        encode_packet(&payload)
    }

    fn receive_with_tracked_progress(
        responses: &Receiver<Vec<u8>>,
        decoder: &mut PacketDecoder,
        pending: &mut VecDeque<Vec<u8>>,
    ) -> (
        Result<Vec<String>, LoopbackTransportError>,
        Vec<LispProbeProgress>,
    ) {
        let mut events = Vec::new();
        let result = receive_lisp_prints_from_channel(
            responses,
            decoder,
            pending,
            fast_lisp_print_receive_config(),
            |event| events.push(event),
        );
        (result, events)
    }

    fn simulate_lisp_probe_reconnects<F>(mut run: F, max: usize) -> usize
    where
        F: FnMut() -> Result<(), LoopbackTransportError>,
    {
        let mut reconnects = 0;
        while reconnects < max {
            if run().is_ok() {
                break;
            }
            reconnects += 1;
        }
        reconnects
    }

    #[test]
    fn lisp_probe_unit_behavior() {
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

        let packet = build_custom_app_data_packet(&[1, 2, 3]);
        let decoded = PacketDecoder::new()
            .push(&packet)
            .expect("decoded packet")
            .pop()
            .expect("complete packet");
        assert_eq!(decoded, vec![COMM_CUSTOM_APP_DATA, 1, 2, 3]);

        let command = lisp_probe_command();
        assert!(command.contains("vesc-rust-probe-rust-diag-v4"));
        assert!(
            !command.contains("load-native-lib"),
            "lisp-probe should exercise the already-loaded package extensions, not retry native loading"
        );
        assert!(
            !command.contains("(sleep"),
            "VESC Tool sends REPL expressions directly; host-side progress handles waiting"
        );
        assert!(command.contains("(trap (ext-rust-probe-diag-v4 14))"));
        assert!(command.contains("vesc-rust-probe-ok-42"));

        let repl_packet = build_lisp_repl_packet(command);
        let repl_decoded = PacketDecoder::new()
            .push(&repl_packet)
            .expect("valid packet")
            .pop()
            .expect("complete packet");
        let mut expected = Vec::with_capacity(command.len() + 2);
        expected.push(COMM_LISP_REPL_CMD);
        expected.extend_from_slice(command.as_bytes());
        expected.push(0);
        assert_eq!(repl_decoded, expected);
        assert_eq!(repl_decoded[0], COMM_LISP_REPL_CMD);

        assert!(lisp_probe_report(Vec::new(), 1).prints().is_empty());
        assert_eq!(lisp_probe_report(Vec::new(), 1).attempts(), 1);

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
                line: "vesc-rust-probe-rust-diag-v4".to_owned(),
            }
            .describe(),
            "lisp print: vesc-rust-probe-rust-diag-v4"
        );
        assert_eq!(
            LispProbeProgress::ExpectedResultReceived.describe(),
            "received expected Lisp probe result"
        );

        let prints = vec!["vesc-rust-probe-ok-42".to_owned()];
        assert_eq!(lisp_probe_report(prints.clone(), 1).prints(), prints);
        assert_eq!(lisp_probe_report(prints, 1).attempts(), 1);

        assert_eq!(parse_lisp_print(b"42\0ignored"), "42");
        assert_eq!(parse_lisp_print(&[0xff, b'a']), "\u{fffd}a");

        assert!(lisp_probe_line_is_success("vesc-rust-probe-ok-42"));
        assert!(lisp_probe_line_is_success(
            "prefix vesc-rust-probe-ok-42 suffix"
        ));
        assert!(!lisp_probe_line_is_success("vesc-rust-probe-rust-diag-v4"));
        assert!(!lisp_probe_line_is_success("41"));
        assert!(!lisp_probe_has_expected_result(&[]));
        assert!(lisp_probe_has_expected_result(&[
            "vesc-rust-probe-rust-diag-v4".to_owned(),
            "vesc-rust-probe-ok-42".to_owned(),
        ]));

        assert_eq!(
            continuous_probe_step(4, &["vesc-rust-probe-ok-42".to_owned()]),
            ContinuousProbeStep::ContinueImmediately { next_attempt: 1 }
        );
        assert_eq!(
            continuous_probe_step(1, &[]),
            ContinuousProbeStep::RetryAfterDelay { next_attempt: 2 }
        );
        assert_eq!(
            continuous_probe_step(4, &["unexpected".to_owned()]),
            ContinuousProbeStep::RetryAfterDelay { next_attempt: 5 }
        );
        assert_eq!(
            continuous_probe_step(5, &[]),
            ContinuousProbeStep::RetryAfterDelay { next_attempt: 1 }
        );
        assert_ne!(
            continuous_probe_step(2, &["vesc-rust-probe-ok-42".to_owned()]),
            ContinuousProbeStep::RetryAfterDelay { next_attempt: 3 }
        );

        let mut attempt = 3;
        let mut retry_events = 0;
        for round in 0..4 {
            let round_prints = if round % 2 == 0 {
                vec!["vesc-rust-probe-ok-42".to_owned()]
            } else {
                vec!["vesc-rust-probe-rust-diag-v4".to_owned()]
            };
            match continuous_probe_step(attempt, &round_prints) {
                ContinuousProbeStep::ContinueImmediately { next_attempt } => {
                    assert!(lisp_probe_has_expected_result(&round_prints));
                    assert_eq!(next_attempt, 1);
                    attempt = next_attempt;
                }
                ContinuousProbeStep::RetryAfterDelay { next_attempt } => {
                    retry_events += 1;
                    attempt = next_attempt;
                }
            }
        }
        assert_eq!(attempt, 2);
        assert_eq!(retry_events, 2);

        let mut pending = VecDeque::from([
            vec![COMM_LISP_PRINT, b'a', 0],
            vec![COMM_CUSTOM_APP_DATA, 1],
            vec![COMM_LISP_PRINT, b'b', 0],
        ]);
        assert_eq!(
            take_pending_lisp_prints(&mut pending),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(pending, VecDeque::from([vec![COMM_CUSTOM_APP_DATA, 1]]));
    }

    #[test]
    fn receive_lisp_prints_covers_channel_and_pending_behavior() {
        {
            let (responses_tx, responses_rx) = mpsc::channel();
            let mut decoder = PacketDecoder::new();
            let mut pending = VecDeque::new();

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                responses_tx
                    .send(build_lisp_print_notification(
                        "vesc-rust-probe-rust-diag-v4",
                    ))
                    .expect("diag print");
                responses_tx
                    .send(build_lisp_print_notification("vesc-rust-probe-ok-42"))
                    .expect("success print");
            });

            let (prints, events) =
                receive_with_tracked_progress(&responses_rx, &mut decoder, &mut pending);

            assert_eq!(
                prints.expect("prints"),
                vec![
                    "vesc-rust-probe-rust-diag-v4".to_owned(),
                    "vesc-rust-probe-ok-42".to_owned(),
                ]
            );
            assert!(events.contains(&LispProbeProgress::ExpectedResultReceived));
            assert!(
                !events
                    .iter()
                    .any(|event| { matches!(event, LispProbeProgress::QuietAfterPrints { .. }) })
            );
        }

        {
            let (responses_tx, responses_rx) = mpsc::channel();
            drop(responses_tx);

            let mut decoder = PacketDecoder::new();
            let mut pending = VecDeque::new();
            let error = receive_lisp_prints_from_channel(
                &responses_rx,
                &mut decoder,
                &mut pending,
                fast_lisp_print_receive_config(),
                |_| {},
            )
            .expect_err("disconnect");

            assert_eq!(
                error,
                LoopbackTransportError::Device("BLE notification stream disconnected".to_owned())
            );
        }

        {
            let (responses_tx, responses_rx) = mpsc::channel();
            let mut decoder = PacketDecoder::new();
            let mut pending = VecDeque::new();

            responses_tx
                .send(build_lisp_print_notification("only-diag"))
                .expect("diag print");

            let (prints, events) =
                receive_with_tracked_progress(&responses_rx, &mut decoder, &mut pending);

            assert_eq!(prints.expect("prints"), vec!["only-diag".to_owned()]);
            assert!(events.iter().any(|event| {
                matches!(event, LispProbeProgress::QuietAfterPrints { prints: 1 })
            }));
            assert!(!events.contains(&LispProbeProgress::ExpectedResultReceived));
        }

        {
            let (_responses_tx, responses_rx) = mpsc::channel();
            let mut decoder = PacketDecoder::new();
            let mut pending = VecDeque::from([vec![
                COMM_LISP_PRINT,
                b'p',
                b'r',
                b'e',
                b's',
                b'e',
                b'e',
                b'd',
                0,
            ]]);

            let (prints, events) =
                receive_with_tracked_progress(&responses_rx, &mut decoder, &mut pending);

            assert_eq!(prints.expect("prints"), vec!["preseed".to_owned()]);
            assert!(events.iter().any(|event| {
                matches!(event, LispProbeProgress::QuietAfterPrints { prints: 1 })
            }));
        }

        {
            let (responses_tx, responses_rx) = mpsc::channel();
            let mut decoder = PacketDecoder::new();
            let mut pending = VecDeque::new();

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                responses_tx
                    .send(encode_packet(&[COMM_CUSTOM_APP_DATA, 9, 8, 7]))
                    .expect("custom app data");
                responses_tx
                    .send(build_lisp_print_notification("vesc-rust-probe-ok-42"))
                    .expect("success print");
            });

            let prints = receive_lisp_prints_from_channel(
                &responses_rx,
                &mut decoder,
                &mut pending,
                fast_lisp_print_receive_config(),
                |_| {},
            )
            .expect("prints");

            assert_eq!(prints, vec!["vesc-rust-probe-ok-42".to_owned()]);
            assert_eq!(
                pending,
                VecDeque::from([vec![COMM_CUSTOM_APP_DATA, 9, 8, 7]])
            );
        }
    }

    #[test]
    fn reconnect_loop_behavior() {
        let mut session_opens = 0;
        let reconnects = simulate_lisp_probe_reconnects(
            || {
                session_opens += 1;
                Err(LoopbackTransportError::Device(
                    "BLE notification stream disconnected".to_owned(),
                ))
            },
            3,
        );
        assert_eq!(session_opens, 3);
        assert_eq!(reconnects, 3);

        session_opens = 0;
        let reconnects = simulate_lisp_probe_reconnects(
            || {
                session_opens += 1;
                Ok(())
            },
            3,
        );
        assert_eq!(session_opens, 1);
        assert_eq!(reconnects, 0);
    }
}
