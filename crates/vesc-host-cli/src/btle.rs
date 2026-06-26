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
const LISP_PROBE_TIMEOUT: Duration = Duration::from_secs(4);
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
}

impl LispProbeReport {
    pub fn prints(&self) -> &[String] {
        &self.prints
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

    fn receive_lisp_prints(&mut self) -> Result<Vec<String>, LoopbackTransportError> {
        let mut prints = self.take_pending_lisp_prints();
        let deadline = std::time::Instant::now() + LISP_PROBE_TIMEOUT;

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            match self.responses.recv_timeout(remaining) {
                Ok(bytes) => {
                    let packets = self.decoder.push(&bytes).map_err(|_| {
                        LoopbackTransportError::Device("failed to decode a Lisp probe reply")
                    })?;
                    packets
                        .into_iter()
                        .for_each(|packet| self.pending.push_back(packet));
                    prints.extend(self.take_pending_lisp_prints());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(LoopbackTransportError::Device(
                        "BLE notification stream disconnected",
                    ));
                }
            }
        }

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

pub fn run_lisp_probe() -> Result<LispProbeReport, LoopbackTransportError> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| LoopbackTransportError::Device("failed to start the BLE runtime"))?;

    let mut session = runtime.block_on(open_session(LoopbackTarget::default()))?;
    let command = r#"(progn
    (print "vesc-rust-probe-v4")
    (print (ext-rust-add 20 22)))"#;
    runtime.block_on(write_ble_uart_packet(
        &session.peripheral,
        &session.rx_char,
        &build_lisp_repl_packet(command),
    ))?;
    let prints = session.receive_lisp_prints()?;

    Ok(LispProbeReport { prints })
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
        build_custom_app_data_packet, build_lisp_repl_packet, parse_lisp_print,
        vesc_ble_uart_rx_uuid, vesc_ble_uart_service_uuid, vesc_ble_uart_tx_uuid,
        COMM_CUSTOM_APP_DATA, COMM_LISP_REPL_CMD,
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
        let command = r#"(progn
    (print "vesc-rust-probe-v4")
    (print (ext-rust-add 20 22)))"#;
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
    fn parses_lisp_print_packets_as_lossy_strings() {
        assert_eq!(parse_lisp_print(b"42\0ignored"), "42");
        assert_eq!(parse_lisp_print(&[0xff, b'a']), "\u{fffd}a");
    }
}
