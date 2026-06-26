use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use tokio::runtime::{Builder, Runtime};
use tokio::time;

use crate::ble_discovery::{find_matching_peripheral, vesc_tool_scan_filter, DiscoveryError};
use crate::loopback::LoopbackTarget;
use crate::package_install::{PackageInstallError, PackageInstallTransport};
use crate::vesc_uart::{encode_packet, PacketDecoder};

const VESC_BLE_UART_RX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const FW_VERSION_TIMEOUT: Duration = Duration::from_secs(8);
const ERASE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(6);
const WRITE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(1);
const CHUNK_SIZE: usize = 384;
const WRITE_RETRIES: usize = 5;
const QML_UPLOAD_LIMIT: usize = 1024 * 120;
const LISP_UPLOAD_LIMIT_ESP32: usize = 1024 * 512 - 6;
const LISP_UPLOAD_LIMIT_VESC: usize = 1024 * 128 - 6;

const COMM_QMLUI_ERASE: u8 = 120;
const COMM_QMLUI_WRITE: u8 = 121;
const COMM_LISP_WRITE_CODE: u8 = 131;
const COMM_LISP_ERASE_CODE: u8 = 132;
const COMM_LISP_SET_RUNNING: u8 = 133;
const COMM_FW_VERSION: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HwType {
    Vesc,
    VescBms,
    CustomModule,
    Other(i8),
}

impl HwType {
    fn from_raw(value: i8) -> Self {
        match value {
            0 => Self::Vesc,
            1 => Self::VescBms,
            2 => Self::CustomModule,
            other => Self::Other(other),
        }
    }

    fn is_vesc(self) -> bool {
        matches!(self, Self::Vesc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FwVersionInfo {
    hw_type: HwType,
    has_qml_app: bool,
}

#[derive(Debug)]
struct VescSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    pending: VecDeque<Vec<u8>>,
    fw_info: FwVersionInfo,
}

impl VescSession {
    fn query_fw_info(&mut self, runtime: &Runtime) -> Result<FwVersionInfo, PackageInstallError> {
        let request = encode_packet(&[COMM_FW_VERSION]);
        runtime
            .block_on(
                self.peripheral
                    .write(&self.rx_char, &request, WriteType::WithoutResponse),
            )
            .map_err(|_| {
                PackageInstallError::Device("failed to query the firmware version".to_owned())
            })?;

        let response = self.recv_packet(COMM_FW_VERSION, FW_VERSION_TIMEOUT)?;
        parse_fw_version_info(&response)
    }

    fn recv_packet(
        &mut self,
        expected_command: u8,
        timeout: Duration,
    ) -> Result<Vec<u8>, PackageInstallError> {
        if let Some(packet) = self.take_pending(expected_command) {
            return Ok(packet);
        }

        loop {
            if let Some(packet) = self.decoder.pop_ready() {
                if packet.first().copied() == Some(expected_command) {
                    return Ok(packet);
                }
                self.pending.push_back(packet);
                continue;
            }

            let bytes = self.responses.recv_timeout(timeout).map_err(|_| {
                PackageInstallError::Device("timed out waiting for a BLE reply".to_owned())
            })?;

            let packets = self.decoder.push(&bytes).map_err(|_| {
                PackageInstallError::Device("failed to decode a BLE reply".to_owned())
            })?;
            for packet in packets {
                if packet.first().copied() == Some(expected_command) {
                    return Ok(packet);
                }
                self.pending.push_back(packet);
            }
            if let Some(packet) = self.take_pending(expected_command) {
                return Ok(packet);
            }
        }
    }

    fn take_pending(&mut self, expected_command: u8) -> Option<Vec<u8>> {
        let len = self.pending.len();
        for _ in 0..len {
            let packet = self.pending.pop_front()?;
            if packet.first().copied() == Some(expected_command) {
                return Some(packet);
            }
            self.pending.push_back(packet);
        }

        None
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

    pub fn open(&self, target: LoopbackTarget) -> Result<(), PackageInstallError> {
        self.open_session(target)
    }

    fn open_session(&self, target: LoopbackTarget) -> Result<(), PackageInstallError> {
        let mut session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        session.fw_info = session.query_fw_info(&self.runtime)?;
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

    fn write_command(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, PackageInstallError> {
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
            session.recv_packet(command, timeout)
        })
    }

    fn send_with_retries(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
        mut response_is_ok: impl FnMut(&[u8]) -> Result<bool, PackageInstallError>,
    ) -> Result<(), PackageInstallError> {
        for attempt in 0..WRITE_RETRIES {
            match self.write_command(command, payload, timeout) {
                Ok(response) => match response_is_ok(&response)? {
                    true => return Ok(()),
                    false if attempt + 1 < WRITE_RETRIES => continue,
                    false => {
                        return Err(PackageInstallError::Device(
                            "device rejected the package write".to_owned(),
                        ));
                    }
                },
                Err(_) if attempt + 1 < WRITE_RETRIES => continue,
                Err(error) => return Err(error),
            }
        }

        unreachable!("retry loop always returns");
    }

    fn expect_ok(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), PackageInstallError> {
        self.send_with_retries(command, payload, timeout, |response| {
            parse_simple_ack(response, command)
        })
    }

    fn expect_write_ok(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
        expected_offset: u32,
    ) -> Result<(), PackageInstallError> {
        self.send_with_retries(command, payload, timeout, |response| {
            parse_write_ack(response, command, expected_offset)
        })
    }
}

impl PackageInstallTransport for BtlePackageInstallTransport {
    fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
        self.with_session(|session| Ok(session.fw_info.has_qml_app))
    }

    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.expect_ok(
            COMM_QMLUI_ERASE,
            &(bytes as i32).to_be_bytes(),
            ERASE_RESPONSE_TIMEOUT,
        )
    }

    fn upload_qml(&self, qml: &[u8], fullscreen: bool) -> Result<(), PackageInstallError> {
        let payload = build_qml_upload_payload(qml, fullscreen)?;

        for (offset, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
            let mut command_payload = Vec::with_capacity(4 + chunk.len());
            let offset = (offset * CHUNK_SIZE) as u32;
            command_payload.extend_from_slice(&offset.to_be_bytes());
            command_payload.extend_from_slice(chunk);
            self.expect_write_ok(
                COMM_QMLUI_WRITE,
                &command_payload,
                WRITE_RESPONSE_TIMEOUT,
                offset,
            )?;
        }

        Ok(())
    }

    fn erase_lisp(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.expect_ok(
            COMM_LISP_ERASE_CODE,
            &(bytes as i32).to_be_bytes(),
            ERASE_RESPONSE_TIMEOUT,
        )
    }

    fn upload_lisp(&self, lisp: &[u8]) -> Result<(), PackageInstallError> {
        let hw_type = self.with_session(|session| Ok(session.fw_info.hw_type))?;
        let payload = build_lisp_upload_payload(lisp, hw_type)?;

        for (offset, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
            let mut command_payload = Vec::with_capacity(4 + chunk.len());
            let offset = (offset * CHUNK_SIZE) as u32;
            command_payload.extend_from_slice(&offset.to_be_bytes());
            command_payload.extend_from_slice(chunk);
            self.expect_write_ok(
                COMM_LISP_WRITE_CODE,
                &command_payload,
                WRITE_RESPONSE_TIMEOUT,
                offset,
            )?;
        }

        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
        self.expect_ok(
            COMM_LISP_SET_RUNNING,
            &[u8::from(running)],
            WRITE_RESPONSE_TIMEOUT,
        )
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        thread::sleep(Duration::from_millis(500));
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
        .start_scan(vesc_tool_scan_filter())
        .await
        .map_err(|_| PackageInstallError::Device("failed to start BLE scan".to_owned()))?;

    let peripheral = time::timeout(SCAN_TIMEOUT, find_matching_peripheral(&adapter, &target))
        .await
        .map_err(|_| {
            PackageInstallError::Device("scan timed out while opening the BLE transport".to_owned())
        })?
        .map_err(map_discovery_error)?;

    let _ = adapter.stop_scan().await;

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
        pending: VecDeque::new(),
        fw_info: FwVersionInfo {
            hw_type: HwType::Vesc,
            has_qml_app: false,
        },
    })
}

fn map_discovery_error(error: DiscoveryError) -> PackageInstallError {
    match error {
        DiscoveryError::InspectFailed => {
            PackageInstallError::Device("failed to inspect BLE peripherals".to_owned())
        }
        DiscoveryError::EventStreamFailed => {
            PackageInstallError::Device("failed to open the BLE event stream".to_owned())
        }
    }
}

fn build_qml_upload_payload(qml: &[u8], fullscreen: bool) -> Result<Vec<u8>, PackageInstallError> {
    let mut payload = Vec::with_capacity(2 + 4 + 2 + qml.len());
    let fullscreen_flag = if fullscreen { 2_u16 } else { 1_u16 };
    payload.extend_from_slice(&fullscreen_flag.to_be_bytes());
    payload.extend_from_slice(&(qml.len() as u32).to_be_bytes());

    let mut crc_input = Vec::with_capacity(2 + qml.len());
    crc_input.extend_from_slice(&fullscreen_flag.to_be_bytes());
    crc_input.extend_from_slice(qml);
    payload.extend_from_slice(&crate::vesc_uart::crc16(&crc_input).to_be_bytes());
    payload.extend_from_slice(qml);

    if payload.len() > QML_UPLOAD_LIMIT {
        return Err(PackageInstallError::Device("not enough space".to_owned()));
    }

    Ok(payload)
}

fn build_lisp_upload_payload(lisp: &[u8], hw_type: HwType) -> Result<Vec<u8>, PackageInstallError> {
    let limit = if hw_type.is_vesc() {
        LISP_UPLOAD_LIMIT_VESC
    } else {
        LISP_UPLOAD_LIMIT_ESP32
    };

    let mut payload = Vec::with_capacity(4 + 2 + lisp.len());
    payload.extend_from_slice(&((lisp.len().saturating_sub(2)) as u32).to_be_bytes());
    payload.extend_from_slice(&crate::vesc_uart::crc16(lisp).to_be_bytes());
    payload.extend_from_slice(lisp);

    if payload.len() > limit {
        return Err(PackageInstallError::Device("not enough space".to_owned()));
    }

    Ok(payload)
}

fn parse_fw_version_info(response: &[u8]) -> Result<FwVersionInfo, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != COMM_FW_VERSION {
        return Err(malformed_reply(
            "unexpected BLE reply while reading firmware version",
        ));
    }

    let _major = read_i8(&mut cursor)?;
    let _minor = read_i8(&mut cursor)?;
    let _hw = read_string(&mut cursor)?;
    let _uuid = take_bytes(&mut cursor, 12)?;
    let _is_paired = read_i8(&mut cursor)?;
    let _is_test_fw = read_i8(&mut cursor)?;
    let hw_type = HwType::from_raw(read_i8(&mut cursor)?);
    let _custom_config_num = read_i8(&mut cursor)?;
    let _has_phase_filters = read_i8(&mut cursor)?;
    let _qml_hw = read_i8(&mut cursor)?;
    let qml_app = read_i8(&mut cursor)?;

    Ok(FwVersionInfo {
        hw_type,
        has_qml_app: qml_app > 0,
    })
}

fn parse_simple_ack(response: &[u8], expected_command: u8) -> Result<bool, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != expected_command {
        return Err(malformed_reply("unexpected BLE reply"));
    }

    Ok(read_u8(&mut cursor)? > 0)
}

fn parse_write_ack(
    response: &[u8],
    expected_command: u8,
    expected_offset: u32,
) -> Result<bool, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != expected_command {
        return Err(malformed_reply("unexpected BLE reply"));
    }

    let ok = read_u8(&mut cursor)? > 0;
    let offset = read_u32_be(&mut cursor)?;
    if offset != expected_offset {
        return Err(malformed_reply("unexpected BLE write offset"));
    }

    Ok(ok)
}

fn read_string(cursor: &mut &[u8]) -> Result<String, PackageInstallError> {
    let Some(len) = cursor.iter().position(|byte| *byte == 0) else {
        return Err(malformed_reply("missing NUL terminator"));
    };
    let bytes = take_bytes(cursor, len)?;
    take_bytes(cursor, 1)?;
    String::from_utf8(bytes).map_err(|_| malformed_reply("invalid UTF-8"))
}

fn read_u8(cursor: &mut &[u8]) -> Result<u8, PackageInstallError> {
    Ok(take_bytes(cursor, 1)?[0])
}

fn read_i8(cursor: &mut &[u8]) -> Result<i8, PackageInstallError> {
    Ok(i8::from_be_bytes([read_u8(cursor)?]))
}

fn read_u32_be(cursor: &mut &[u8]) -> Result<u32, PackageInstallError> {
    Ok(u32::from_be_bytes(
        take_bytes(cursor, 4)?.try_into().expect("slice length"),
    ))
}

fn take_bytes(cursor: &mut &[u8], len: usize) -> Result<Vec<u8>, PackageInstallError> {
    if cursor.len() < len {
        return Err(malformed_reply("truncated BLE reply"));
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head.to_vec())
}

fn malformed_reply(reason: &str) -> PackageInstallError {
    PackageInstallError::Device(reason.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        build_lisp_upload_payload, build_qml_upload_payload, parse_fw_version_info,
        parse_simple_ack, parse_write_ack, FwVersionInfo, HwType, COMM_FW_VERSION,
        COMM_LISP_WRITE_CODE, COMM_QMLUI_ERASE,
    };

    #[test]
    fn parses_fw_version_replies_with_qml_app_support() {
        let mut response = Vec::new();
        response.push(COMM_FW_VERSION);
        response.extend_from_slice(&[75, 15]);
        response.extend_from_slice(b"VESC\0");
        response.extend_from_slice(&[0_u8; 12]);
        response.extend_from_slice(&[0, 0, 1, 0, 0, 0, 1]);

        let info = parse_fw_version_info(&response).expect("info");
        assert_eq!(
            info,
            FwVersionInfo {
                hw_type: HwType::VescBms,
                has_qml_app: true,
            }
        );
    }

    #[test]
    fn parses_fw_version_replies_without_qml_app_support() {
        let mut response = Vec::new();
        response.push(COMM_FW_VERSION);
        response.extend_from_slice(&[75, 15]);
        response.extend_from_slice(b"VESC\0");
        response.extend_from_slice(&[0_u8; 12]);
        response.extend_from_slice(&[0, 0, 1, 0, 0, 0, 0]);

        let info = parse_fw_version_info(&response).expect("info");
        assert_eq!(
            info,
            FwVersionInfo {
                hw_type: HwType::VescBms,
                has_qml_app: false,
            }
        );
    }

    #[test]
    fn parses_simple_ack_packets() {
        let response = [COMM_QMLUI_ERASE, 1];
        assert!(parse_simple_ack(&response, COMM_QMLUI_ERASE).expect("ack"));
    }

    #[test]
    fn parses_write_ack_packets_and_validates_offsets() {
        let mut response = Vec::new();
        response.push(COMM_LISP_WRITE_CODE);
        response.push(1);
        response.extend_from_slice(&384_u32.to_be_bytes());

        assert!(parse_write_ack(&response, COMM_LISP_WRITE_CODE, 384).expect("ack"));
    }

    #[test]
    fn rejects_wrong_write_offsets() {
        let mut response = Vec::new();
        response.push(COMM_LISP_WRITE_CODE);
        response.push(1);
        response.extend_from_slice(&128_u32.to_be_bytes());

        assert!(parse_write_ack(&response, COMM_LISP_WRITE_CODE, 384).is_err());
    }

    #[test]
    fn rejects_oversized_qml_uploads() {
        let qml = vec![0_u8; 1024 * 120];
        assert!(build_qml_upload_payload(&qml, false).is_err());
    }

    #[test]
    fn uses_the_vesc_lisp_limit_for_vesc_hardware() {
        let lisp = vec![0_u8; 1024 * 128];
        assert!(build_lisp_upload_payload(&lisp, HwType::Vesc).is_err());
    }

    #[test]
    fn allows_larger_lisp_uploads_for_non_vesc_hardware() {
        let lisp = vec![0_u8; 1024 * 128];
        assert!(build_lisp_upload_payload(&lisp, HwType::CustomModule).is_ok());
    }
}
