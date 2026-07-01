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

use crate::ble_discovery::{DiscoveryError, find_matching_peripheral, vesc_tool_scan_filter};
use crate::loopback::LoopbackTarget;
use crate::loopback::LoopbackTransportError;
use crate::package_install::{PackageInstallError, PackageInstallTransport};
use crate::vesc_uart::{PacketDecoder, encode_packet};

const VESC_BLE_UART_RX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const FW_VERSION_TIMEOUT: Duration = Duration::from_secs(8);
const POST_LISP_RESTART_QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const LISP_SET_RUNNING_TIMEOUT: Duration = Duration::from_secs(60);
const POST_LISP_LOADER_SETTLE: Duration = Duration::from_secs(3);
const ERASE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(6);
const WRITE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(1);
const CHUNK_SIZE: usize = 384;
const BLE_WRITE_CHUNK_SIZE: usize = 20;
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
const COMM_CUSTOM_APP_DATA: u8 = 36;

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
pub(crate) struct VescSession {
    pub(crate) peripheral: Peripheral,
    pub(crate) rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
    decoder: PacketDecoder,
    pending: VecDeque<Vec<u8>>,
    fw_info: FwVersionInfo,
}

impl VescSession {
    pub(crate) fn confirm_fw_version(
        &mut self,
        runtime: &Runtime,
    ) -> Result<(), PackageInstallError> {
        self.query_fw_info(runtime).map(|_| ())
    }

    fn query_fw_info(&mut self, runtime: &Runtime) -> Result<FwVersionInfo, PackageInstallError> {
        self.query_fw_info_with_timeout(runtime, FW_VERSION_TIMEOUT)
    }

    pub(crate) fn clear_packet_state(&mut self) {
        self.pending.clear();
        while self.decoder.pop_ready().is_some() {}
        drain_response_channel(&self.responses);
    }

    pub(crate) fn receive_custom_app_data(
        &mut self,
        timeout: Duration,
    ) -> Result<Vec<u8>, PackageInstallError> {
        let packet = self.recv_packet(COMM_CUSTOM_APP_DATA, timeout)?;
        Ok(packet[1..].to_vec())
    }

    fn query_fw_info_with_timeout(
        &mut self,
        runtime: &Runtime,
        timeout: Duration,
    ) -> Result<FwVersionInfo, PackageInstallError> {
        let request = encode_packet(&[COMM_FW_VERSION]);
        runtime.block_on(write_ble_uart_packet(
            &self.peripheral,
            &self.rx_char,
            &request,
        ))?;

        let response = self.recv_packet(COMM_FW_VERSION, timeout)?;
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

fn drain_response_channel(responses: &Receiver<Vec<u8>>) {
    while responses.try_recv().is_ok() {}
}

/// BLE UART transport used by package install and erase flows.
#[derive(Debug)]
pub struct BtlePackageInstallTransport {
    runtime: Runtime,
    session: RefCell<Option<VescSession>>,
}

impl BtlePackageInstallTransport {
    /// Creates a BLE package install transport with its own single-worker runtime.
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

    /// Opens a package install session to `target`.
    pub fn open(&self, target: LoopbackTarget) -> Result<(), PackageInstallError> {
        self.open_session(target)
    }

    /// Disconnects the active BLE session, if one is open.
    pub fn close(&self) {
        let mut session = self.session.borrow_mut();
        if let Some(session) = session.take() {
            let peripheral = session.peripheral;
            self.runtime.block_on(async move {
                let _ = peripheral.disconnect().await;
            });
        }
    }

    pub(crate) fn with_loopback_session<R>(
        &self,
        f: impl FnOnce(&Runtime, &mut VescSession) -> Result<R, LoopbackTransportError>,
    ) -> Result<R, LoopbackTransportError> {
        let mut session = self.session.borrow_mut();
        let session = session.as_mut().ok_or_else(|| {
            LoopbackTransportError::Device("BLE transport has not been opened".to_owned())
        })?;
        f(&self.runtime, session)
    }

    pub(crate) fn with_app_data_session<R>(
        &self,
        f: impl FnOnce(&Runtime, &mut VescSession) -> Result<R, PackageInstallError>,
    ) -> Result<R, PackageInstallError> {
        let mut session = self.session.borrow_mut();
        let session = session.as_mut().ok_or_else(|| {
            PackageInstallError::Device("BLE transport has not been opened".to_owned())
        })?;
        f(&self.runtime, session)
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
        let packet = build_command_packet(command, payload);

        self.with_session(|session| {
            self.runtime
                .block_on(write_ble_uart_packet(
                    &session.peripheral,
                    &session.rx_char,
                    &packet,
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
    ) -> Result<(), PackageInstallError> {
        self.send_with_retries(command, payload, timeout, |response| {
            parse_write_ack(response, command)
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
            self.expect_write_ok(COMM_QMLUI_WRITE, &command_payload, WRITE_RESPONSE_TIMEOUT)?;
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
            )?;
        }

        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
        self.expect_ok(
            COMM_LISP_SET_RUNNING,
            &[u8::from(running)],
            LISP_SET_RUNNING_TIMEOUT,
        )
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        // `set_running(true)` waits for the restart ack, but the loader still runs
        // incrementally on the eval thread (`load-native-lib`, event handler, etc.).
        thread::sleep(POST_LISP_LOADER_SETTLE);
        self.with_session(|session| {
            session.query_fw_info_with_timeout(&self.runtime, POST_LISP_RESTART_QUERY_TIMEOUT)?;
            thread::sleep(Duration::from_secs(1));
            session.query_fw_info_with_timeout(&self.runtime, POST_LISP_RESTART_QUERY_TIMEOUT)?;
            Ok(())
        })
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

    let (responses_tx, responses_rx) = mpsc::channel();
    let notification_uuid = tx_char.uuid;
    let mut notifications = peripheral
        .notifications()
        .await
        .map_err(|_| PackageInstallError::Device("missing BLE TX characteristic".to_owned()))?;

    peripheral
        .subscribe(&tx_char)
        .await
        .map_err(|_| PackageInstallError::Device("missing BLE TX characteristic".to_owned()))?;

    tokio::spawn(async move {
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

pub(crate) async fn write_ble_uart_packet(
    peripheral: &Peripheral,
    rx_char: &Characteristic,
    packet: &[u8],
) -> Result<(), PackageInstallError> {
    for chunk in ble_write_chunks(packet) {
        peripheral
            .write(rx_char, chunk, WriteType::WithoutResponse)
            .await
            .map_err(|_| PackageInstallError::Device("failed to write a BLE command".to_owned()))?;
    }
    Ok(())
}

fn ble_write_chunks(packet: &[u8]) -> impl Iterator<Item = &[u8]> {
    packet.chunks(BLE_WRITE_CHUNK_SIZE)
}

pub(crate) fn build_command_packet(command: u8, payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(payload.len() + 1);
    data.push(command);
    data.extend_from_slice(payload);
    encode_packet(&data)
}

fn map_discovery_error(error: DiscoveryError) -> PackageInstallError {
    match error {
        DiscoveryError::InspectFailed => {
            PackageInstallError::Device("failed to inspect BLE peripherals".to_owned())
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

fn parse_write_ack(response: &[u8], expected_command: u8) -> Result<bool, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != expected_command {
        return Err(malformed_reply("unexpected BLE reply"));
    }

    let ok = read_u8(&mut cursor)? > 0;
    let _offset = read_u32_be(&mut cursor)?;

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
        COMM_FW_VERSION, COMM_LISP_ERASE_CODE, COMM_LISP_SET_RUNNING, COMM_LISP_WRITE_CODE,
        COMM_QMLUI_ERASE, FwVersionInfo, HwType, ble_write_chunks, build_command_packet,
        build_lisp_upload_payload, build_qml_upload_payload, drain_response_channel,
        parse_fw_version_info, parse_simple_ack, parse_write_ack,
    };
    use crate::vesc_uart::PacketDecoder;
    use std::sync::mpsc;

    #[test]
    fn parses_fw_version_replies() {
        let mut with_qml = Vec::new();
        with_qml.push(COMM_FW_VERSION);
        with_qml.extend_from_slice(&[75, 15]);
        with_qml.extend_from_slice(b"VESC\0");
        with_qml.extend_from_slice(&[0_u8; 12]);
        with_qml.extend_from_slice(&[0, 0, 1, 0, 0, 0, 1]);

        assert_eq!(
            parse_fw_version_info(&with_qml).expect("info with qml"),
            FwVersionInfo {
                hw_type: HwType::VescBms,
                has_qml_app: true,
            }
        );

        let mut without_qml = Vec::new();
        without_qml.push(COMM_FW_VERSION);
        without_qml.extend_from_slice(&[75, 15]);
        without_qml.extend_from_slice(b"VESC\0");
        without_qml.extend_from_slice(&[0_u8; 12]);
        without_qml.extend_from_slice(&[0, 0, 1, 0, 0, 0, 0]);

        assert_eq!(
            parse_fw_version_info(&without_qml).expect("info without qml"),
            FwVersionInfo {
                hw_type: HwType::VescBms,
                has_qml_app: false,
            }
        );
    }

    #[test]
    fn parse_ack_packets_covers_write_and_erase_replies() {
        assert!(parse_simple_ack(&[COMM_QMLUI_ERASE, 1], COMM_QMLUI_ERASE).expect("qml ack"));
        assert!(
            parse_simple_ack(&[COMM_LISP_ERASE_CODE, 1], COMM_LISP_ERASE_CODE).expect("lisp ack")
        );
        assert!(!parse_simple_ack(&[COMM_QMLUI_ERASE, 0], COMM_QMLUI_ERASE).expect("failed qml"));
        assert!(
            !parse_simple_ack(&[COMM_LISP_ERASE_CODE, 0], COMM_LISP_ERASE_CODE)
                .expect("failed lisp")
        );

        let mut write_ack = Vec::new();
        write_ack.push(COMM_LISP_WRITE_CODE);
        write_ack.push(1);
        write_ack.extend_from_slice(&384_u32.to_be_bytes());
        assert!(parse_write_ack(&write_ack, COMM_LISP_WRITE_CODE).expect("write ack"));
        assert!(parse_write_ack(&write_ack, COMM_QMLUI_ERASE).is_err());

        let wrong_command = [COMM_LISP_WRITE_CODE, 1];
        assert!(parse_simple_ack(&wrong_command, COMM_QMLUI_ERASE).is_err());
        assert!(parse_simple_ack(&wrong_command, COMM_LISP_ERASE_CODE).is_err());
        assert!(parse_simple_ack(&[COMM_QMLUI_ERASE], COMM_QMLUI_ERASE).is_err());
    }

    #[test]
    fn rejects_oversized_qml_uploads() {
        let qml = vec![0_u8; 1024 * 120];
        assert!(build_qml_upload_payload(&qml, false).is_err());
    }

    #[test]
    fn lisp_upload_limits_depend_on_hardware_type() {
        let vesc_lisp = vec![0_u8; 1024 * 128];
        assert!(build_lisp_upload_payload(&vesc_lisp, HwType::Vesc).is_err());

        let custom_lisp = vec![0_u8; 1024 * 128];
        assert!(build_lisp_upload_payload(&custom_lisp, HwType::CustomModule).is_ok());
    }

    #[test]
    fn erase_command_packets_match_vesc_tool() {
        for bytes in [16_i32, 4096_i32] {
            let payload = bytes.to_be_bytes();
            let expected_tail = payload.to_vec();
            for command in [COMM_QMLUI_ERASE, COMM_LISP_ERASE_CODE] {
                let packet = build_command_packet(command, &payload);
                assert!(
                    !packet.is_empty(),
                    "command {command} should produce a framed packet"
                );

                let decoded = PacketDecoder::new()
                    .push(&packet)
                    .expect("valid packet")
                    .pop()
                    .expect("complete packet");
                assert_eq!(decoded.len(), 5);
                assert_eq!(decoded[0], command);
                assert_eq!(decoded[1..], expected_tail);
            }
        }
    }

    #[test]
    fn ble_uart_writes_are_split_into_vesc_tool_sized_chunks() {
        let packet = [0_u8; 41];
        let chunks = ble_write_chunks(&packet)
            .map(<[u8]>::len)
            .collect::<Vec<_>>();

        assert_eq!(chunks, vec![20, 20, 1]);
    }

    #[test]
    fn clear_packet_state_drains_stale_notification_bytes() {
        let (tx, rx) = mpsc::channel();
        tx.send(vec![COMM_FW_VERSION, 1])
            .expect("first stale reply");
        tx.send(vec![COMM_FW_VERSION, 2])
            .expect("second stale reply");
        drop(tx);

        drain_response_channel(&rx);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lisp_set_running_waits_for_device_ack() {
        let packet = build_command_packet(COMM_LISP_SET_RUNNING, &[1]);
        let decoded = PacketDecoder::new()
            .push(&packet)
            .expect("valid packet")
            .pop()
            .expect("complete packet");

        assert_eq!(decoded, vec![COMM_LISP_SET_RUNNING, 1]);
    }
}
