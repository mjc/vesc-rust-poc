use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use tokio::runtime::{Builder, Runtime};
use tokio::time;

use crate::loopback::LoopbackTarget;
use crate::package_install::{PackageInstallError, PackageInstallTransport};
use crate::vesc_uart::{encode_packet, PacketDecoder};

const VESC_BLE_UART_SERVICE_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_RX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const CHUNK_SIZE: usize = 384;

const COMM_QMLUI_ERASE: u8 = 120;
const COMM_QMLUI_WRITE: u8 = 121;
const COMM_LISP_WRITE_CODE: u8 = 131;
const COMM_LISP_ERASE_CODE: u8 = 132;
const COMM_LISP_SET_RUNNING: u8 = 133;
const COMM_FW_VERSION: u8 = 0;

#[derive(Debug)]
struct VescSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    has_qml_app: bool,
}

impl VescSession {
    fn query_has_qml_app(&mut self, runtime: &Runtime) -> Result<bool, PackageInstallError> {
        let request = encode_packet(&[COMM_FW_VERSION]);
        runtime
            .block_on(
                self.peripheral
                    .write(&self.rx_char, &request, WriteType::WithoutResponse),
            )
            .map_err(|_| {
                PackageInstallError::Device("failed to query the firmware version".to_owned())
            })?;

        let response = self.recv_packet()?;
        Ok(parse_has_qml_app(&response))
    }

    fn recv_packet(&mut self) -> Result<Vec<u8>, PackageInstallError> {
        loop {
            if let Some(packet) = self.decoder.pop_ready() {
                return Ok(packet);
            }

            let bytes = self.responses.recv_timeout(RESPONSE_TIMEOUT).map_err(|_| {
                PackageInstallError::Device("timed out waiting for a BLE reply".to_owned())
            })?;

            let packets = self.decoder.push(&bytes).map_err(|_| {
                PackageInstallError::Device("failed to decode a BLE reply".to_owned())
            })?;
            if let Some(packet) = packets.into_iter().next() {
                return Ok(packet);
            }
        }
    }
}

#[derive(Debug)]
pub struct BtlePackageInstallTransport {
    runtime: Runtime,
    session: RefCell<Option<VescSession>>,
}

impl BtlePackageInstallTransport {
    pub fn new() -> Result<Self, PackageInstallError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .map_err(|_| {
                PackageInstallError::Device("failed to start the BLE runtime".to_owned())
            })?;

        Ok(Self {
            runtime,
            session: RefCell::new(None),
        })
    }

    pub fn open(&self) -> Result<(), PackageInstallError> {
        self.open_session(LoopbackTarget::default())
    }

    fn open_session(&self, target: LoopbackTarget) -> Result<(), PackageInstallError> {
        let mut session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        session.has_qml_app = session.query_has_qml_app(&self.runtime)?;
        *self.session.borrow_mut() = Some(session);
        Ok(())
    }

    fn with_session<R>(
        &self,
        f: impl FnOnce(&mut VescSession) -> Result<R, PackageInstallError>,
    ) -> Result<R, PackageInstallError> {
        let mut session = self.session.borrow_mut();
        let session = session.as_mut().ok_or_else(|| {
            PackageInstallError::Device("BLE transport has not been opened".to_owned())
        })?;
        f(session)
    }

    fn write_command(&self, command: u8, payload: &[u8]) -> Result<Vec<u8>, PackageInstallError> {
        let mut data = Vec::with_capacity(payload.len() + 1);
        data.push(command);
        data.extend_from_slice(payload);
        let packet = encode_packet(&data);

        self.with_session(|session| {
            self.runtime
                .block_on(session.peripheral.write(
                    &session.rx_char,
                    &packet,
                    WriteType::WithoutResponse,
                ))
                .map_err(|_| {
                    PackageInstallError::Device("failed to write a BLE command".to_owned())
                })?;
            session.recv_packet()
        })
    }

    fn expect_ok(&self, command: u8, payload: &[u8]) -> Result<(), PackageInstallError> {
        let response = self.write_command(command, payload)?;
        if response.first().copied().unwrap_or(0) != 0 {
            Ok(())
        } else {
            Err(PackageInstallError::Device(
                "device rejected the package write".to_owned(),
            ))
        }
    }
}

impl PackageInstallTransport for BtlePackageInstallTransport {
    fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
        self.with_session(|session| Ok(session.has_qml_app))
    }

    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.expect_ok(COMM_QMLUI_ERASE, &(bytes as i32).to_be_bytes())
    }

    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError> {
        let mut payload = Vec::with_capacity(2 + 4 + 2 + qml.len());
        let fullscreen_flag = if fullscreen { 2_u16 } else { 1_u16 };
        payload.extend_from_slice(&fullscreen_flag.to_be_bytes());
        payload.extend_from_slice(&(qml.len() as u32).to_be_bytes());
        let mut crc_input = Vec::with_capacity(2 + qml.len());
        crc_input.extend_from_slice(&fullscreen_flag.to_be_bytes());
        crc_input.extend_from_slice(qml);
        payload.extend_from_slice(&crate::vesc_uart::crc16(&crc_input).to_be_bytes());
        payload.extend_from_slice(qml);

        for (offset, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
            let mut command_payload = Vec::with_capacity(4 + chunk.len());
            command_payload.extend_from_slice(&((offset * CHUNK_SIZE) as u32).to_be_bytes());
            command_payload.extend_from_slice(chunk);
            self.expect_ok(COMM_QMLUI_WRITE, &command_payload)?;
        }

        Ok(())
    }

    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.expect_ok(COMM_LISP_ERASE_CODE, &(bytes as i32).to_be_bytes())
    }

    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError> {
        let mut payload = Vec::with_capacity(4 + 2 + lisp.len());
        payload.extend_from_slice(&((lisp.len().saturating_sub(2)) as u32).to_be_bytes());
        payload.extend_from_slice(&crate::vesc_uart::crc16(lisp).to_be_bytes());
        payload.extend_from_slice(lisp);

        for (offset, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
            let mut command_payload = Vec::with_capacity(4 + chunk.len());
            command_payload.extend_from_slice(&((offset * CHUNK_SIZE) as u32).to_be_bytes());
            command_payload.extend_from_slice(chunk);
            self.expect_ok(COMM_LISP_WRITE_CODE, &command_payload)?;
        }

        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
        self.expect_ok(COMM_LISP_SET_RUNNING, &[u8::from(running)])
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        Ok(())
    }
}

async fn open_session(target: LoopbackTarget) -> Result<VescSession, PackageInstallError> {
    let manager = Manager::new()
        .await
        .map_err(|_| PackageInstallError::Device("failed to initialize Bluetooth".to_owned()))?;
    let adapters = manager.adapters().await.map_err(|_| {
        PackageInstallError::Device("failed to enumerate Bluetooth adapters".to_owned())
    })?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or_else(|| PackageInstallError::Device("no Bluetooth adapter found".to_owned()))?;

    adapter
        .start_scan(ScanFilter {
            services: vec![VESC_BLE_UART_SERVICE_UUID],
        })
        .await
        .map_err(|_| PackageInstallError::Device("failed to start BLE scan".to_owned()))?;

    let peripheral = time::timeout(SCAN_TIMEOUT, find_matching_peripheral(&adapter, &target))
        .await
        .map_err(|_| {
            PackageInstallError::Device("scan timed out while opening the BLE transport".to_owned())
        })??;

    time::timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| PackageInstallError::Device("failed to connect to the BLE device".to_owned()))?
        .map_err(|_| {
            PackageInstallError::Device("failed to connect to the BLE device".to_owned())
        })?;
    time::timeout(CONNECT_TIMEOUT, peripheral.discover_services())
        .await
        .map_err(|_| PackageInstallError::Device("missing BLE package service".to_owned()))?
        .map_err(|_| PackageInstallError::Device("missing BLE package service".to_owned()))?;

    let characteristics = peripheral.characteristics();
    let rx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_RX_UUID)
        .cloned()
        .ok_or_else(|| PackageInstallError::Device("missing BLE RX characteristic".to_owned()))?;
    let tx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_TX_UUID)
        .cloned()
        .ok_or_else(|| PackageInstallError::Device("missing BLE TX characteristic".to_owned()))?;

    peripheral
        .subscribe(&tx_char)
        .await
        .map_err(|_| PackageInstallError::Device("missing BLE TX characteristic".to_owned()))?;

    let (responses_tx, responses_rx) = mpsc::channel();
    let notification_peripheral = peripheral.clone();
    let notification_uuid = tx_char.uuid;

    tokio::spawn(async move {
        let Ok(mut notifications) = notification_peripheral.notifications().await else {
            return;
        };

        while let Some(notification) = notifications.next().await {
            if notification.uuid == notification_uuid
                && responses_tx.send(notification.value).is_err()
            {
                break;
            }
        }
    });

    Ok(VescSession {
        peripheral,
        rx_char,
        responses: responses_rx,
        decoder: PacketDecoder::new(),
        has_qml_app: false,
    })
}

async fn find_matching_peripheral(
    adapter: &btleplug::platform::Adapter,
    target: &LoopbackTarget,
) -> Result<Peripheral, PackageInstallError> {
    loop {
        let peripherals = adapter.peripherals().await.map_err(|_| {
            PackageInstallError::Device("failed to inspect BLE peripherals".to_owned())
        })?;

        for peripheral in peripherals {
            let properties = match peripheral.properties().await {
                Ok(Some(properties)) => properties,
                _ => continue,
            };

            if properties.services.contains(&VESC_BLE_UART_SERVICE_UUID)
                || properties
                    .local_name
                    .as_deref()
                    .map(|name| {
                        name.eq_ignore_ascii_case(target.device_name_hint())
                            || name.eq_ignore_ascii_case(target.service_name_hint())
                    })
                    .unwrap_or(false)
            {
                return Ok(peripheral);
            }
        }

        time::sleep(Duration::from_millis(250)).await;
    }
}

fn parse_has_qml_app(response: &[u8]) -> bool {
    let mut cursor = response;
    if cursor.first().copied() != Some(COMM_FW_VERSION) {
        return false;
    }
    cursor = &cursor[1..];
    if cursor.len() < 2 {
        return false;
    }
    cursor = &cursor[2..];

    let Some(nul) = cursor.iter().position(|byte| *byte == 0) else {
        return false;
    };
    cursor = &cursor[nul + 1..];

    if cursor.len() < 12 + 5 + 2 {
        return false;
    }
    cursor = &cursor[12..];
    cursor = &cursor[5..];

    cursor.get(1).copied().unwrap_or(0) > 0
}

#[cfg(test)]
mod tests {
    use super::parse_has_qml_app;

    #[test]
    fn parses_fw_version_replies_with_qml_app_support() {
        let mut response = Vec::new();
        response.push(0);
        response.extend_from_slice(&[75, 15]);
        response.extend_from_slice(b"VESC\0");
        response.extend_from_slice(&[0_u8; 12]);
        response.extend_from_slice(&[0, 0, 1, 0, 0, 0, 1]);

        assert!(parse_has_qml_app(&response));
    }

    #[test]
    fn parses_fw_version_replies_without_qml_app_support() {
        let mut response = Vec::new();
        response.push(0);
        response.extend_from_slice(&[75, 15]);
        response.extend_from_slice(b"VESC\0");
        response.extend_from_slice(&[0_u8; 12]);
        response.extend_from_slice(&[0, 0, 1, 0, 0, 0, 0]);

        assert!(!parse_has_qml_app(&response));
    }
}
