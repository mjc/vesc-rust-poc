use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use flate2::read::ZlibDecoder;
use futures_util::StreamExt;
use tokio::runtime::{Builder, Runtime};
use tokio::time;

use crate::ble_discovery::{DiscoveryError, find_matching_peripheral, vesc_tool_scan_filter};
use crate::loopback::LoopbackTarget;
use crate::loopback::LoopbackTransportError;
use crate::loopback_debug::describe_vesc_packet;
use crate::package_install::{PackageInstallError, PackageInstallTransport};
use crate::vesc_uart::{PacketDecoder, encode_packet};

const VESC_BLE_UART_RX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const FW_VERSION_TIMEOUT: Duration = Duration::from_secs(8);
const FW_VERSION_OPEN_ATTEMPTS: usize = 3;
const FW_VERSION_OPEN_RETRY_DELAY: Duration = Duration::from_millis(750);
const RECOVERY_SET_RUNNING_TIMEOUT: Duration = Duration::from_secs(15);
const POST_LISP_UPLOAD_SETTLE: Duration = Duration::from_secs(2);
// Source: ~/projects/vesc_tool/codeloader.cpp:1023-1024 installVescPackage()
// sleeps 500 ms, then calls VescInterface::reloadFirmware().
const POST_PACKAGE_INSTALL_SETTLE: Duration = Duration::from_millis(500);
// Source: ~/projects/vesc_tool/codeloader.cpp:711-731 CodeLoader::qmlErase()
// uses timeoutTimer.start(6000) after one qmlUiErase() send.
const QML_ERASE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(6);
// Source: ~/projects/vesc_tool/codeloader.cpp:81-101 CodeLoader::lispErase()
// uses timeoutTimer.start(8000) after one lispEraseCode() send.
const LISP_ERASE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
// Source: ~/projects/vesc_tool/codeloader.cpp:402-408 and 759-765
// lispUpload()/qmlUpload() wait 1000 ms per chunk write acknowledgement.
const WRITE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(1);
const QML_READ_TIMEOUT: Duration = Duration::from_millis(1500);
const QML_READ_INITIAL_LEN: u32 = 10;
const CHUNK_SIZE: usize = 384;
// Source: ~/projects/vesc_tool/bleuart.cpp:134-147 splits BLE writes into
// 20-byte WriteWithoutResponse chunks.
const BLE_WRITE_CHUNK_SIZE: usize = 20;
// Source: ~/projects/vesc_tool/codeloader.cpp:423-432 and 780-789
// writeChunk() retries chunk writes with int tries = 5.
const WRITE_RETRIES: usize = 5;
const QML_UPLOAD_LIMIT: usize = 1024 * 120;
const LISP_UPLOAD_LIMIT_ESP32: usize = 1024 * 512 - 6;
const LISP_UPLOAD_LIMIT_VESC: usize = 1024 * 128 - 6;

const COMM_GET_QML_UI_APP: u8 = 118;
const COMM_QMLUI_ERASE: u8 = 120;
const COMM_QMLUI_WRITE: u8 = 121;
const COMM_LISP_READ_CODE: u8 = 130;
const COMM_LISP_WRITE_CODE: u8 = 131;
const COMM_LISP_ERASE_CODE: u8 = 132;
const COMM_LISP_SET_RUNNING: u8 = 133;
const COMM_FW_VERSION: u8 = 0;
const COMM_CUSTOM_APP_DATA: u8 = 36;

fn command_name(command: u8) -> &'static str {
    match command {
        COMM_FW_VERSION => "COMM_FW_VERSION",
        COMM_CUSTOM_APP_DATA => "COMM_CUSTOM_APP_DATA",
        COMM_GET_QML_UI_APP => "COMM_GET_QML_UI_APP",
        COMM_QMLUI_ERASE => "COMM_QMLUI_ERASE",
        COMM_QMLUI_WRITE => "COMM_QMLUI_WRITE",
        COMM_LISP_READ_CODE => "COMM_LISP_READ_CODE",
        COMM_LISP_WRITE_CODE => "COMM_LISP_WRITE_CODE",
        COMM_LISP_ERASE_CODE => "COMM_LISP_ERASE_CODE",
        COMM_LISP_SET_RUNNING => "COMM_LISP_SET_RUNNING",
        _ => "UNKNOWN",
    }
}

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

/// Result of reading the installed QML app payload from firmware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QmlAppRead {
    pub(crate) has_qml_app: bool,
    pub(crate) compressed: Vec<u8>,
    pub(crate) decompressed: Option<String>,
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

    fn query_fw_info_with_retries(
        &mut self,
        runtime: &Runtime,
        attempts: usize,
        retry_delay: Duration,
    ) -> Result<FwVersionInfo, PackageInstallError> {
        let mut last_error = None;
        for attempt in 1..=attempts {
            self.clear_packet_state();
            match self.query_fw_info(runtime) {
                Ok(info) => return Ok(info),
                Err(error) => {
                    last_error = Some(error);
                    if attempt != attempts {
                        thread::sleep(retry_delay);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            PackageInstallError::Device("timed out waiting for firmware preflight".to_owned())
        }))
    }

    pub(crate) fn clear_packet_state(&mut self) {
        clear_response_state(&mut self.pending, &mut self.decoder, &self.responses);
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
                self.push_pending_packet(packet, expected_command);
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
                self.push_pending_packet(packet, expected_command);
            }
            if let Some(packet) = self.take_pending(expected_command) {
                return Ok(packet);
            }
        }
    }

    fn push_pending_packet(&mut self, packet: Vec<u8>, expected_command: u8) {
        eprintln!(
            "package-transport: pending while waiting for {} ({}): {}",
            command_name(expected_command),
            expected_command,
            describe_vesc_packet(&packet)
        );
        self.pending.push_back(packet);
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

fn clear_response_state(
    pending: &mut VecDeque<Vec<u8>>,
    decoder: &mut PacketDecoder,
    responses: &Receiver<Vec<u8>>,
) {
    pending.clear();
    decoder.clear();
    drain_response_channel(responses);
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

    /// Opens a BLE UART session without querying firmware metadata.
    pub fn open_without_preflight(
        &self,
        target: LoopbackTarget,
    ) -> Result<(), PackageInstallError> {
        let session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        *self.session.borrow_mut() = Some(session);
        Ok(())
    }

    /// Best-effort short-timeout stop used when normal preflight is unavailable.
    pub fn stop_running_recovery(&self) -> Result<(), PackageInstallError> {
        self.send_with_attempts(
            COMM_LISP_SET_RUNNING,
            &[0],
            RECOVERY_SET_RUNNING_TIMEOUT,
            1,
            |response| parse_simple_ack(response, COMM_LISP_SET_RUNNING),
        )
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

    pub(crate) fn read_lisp_code(
        &self,
        offset: u32,
        len: u32,
    ) -> Result<LispCodeRead, PackageInstallError> {
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&len.to_be_bytes());
        payload.extend_from_slice(&offset.to_be_bytes());
        let response = self.write_command(COMM_LISP_READ_CODE, &payload, FW_VERSION_TIMEOUT)?;
        parse_lisp_code_read(&response)
    }

    pub(crate) fn read_qml_app(&self) -> Result<QmlAppRead, PackageInstallError> {
        let has_qml_app = self.with_session(|session| {
            session.clear_packet_state();
            let info = session.query_fw_info_with_timeout(&self.runtime, FW_VERSION_TIMEOUT)?;
            session.fw_info = info;
            Ok(info.has_qml_app)
        })?;

        if !has_qml_app {
            return Ok(QmlAppRead {
                has_qml_app,
                compressed: Vec::new(),
                decompressed: None,
            });
        }

        let compressed = self.read_qml_ui(COMM_GET_QML_UI_APP)?;
        let decompressed = decompress_qml_readback(&compressed)?;
        Ok(QmlAppRead {
            has_qml_app,
            compressed,
            decompressed: Some(decompressed),
        })
    }

    fn read_qml_ui(&self, command: u8) -> Result<Vec<u8>, PackageInstallError> {
        let first = self.read_qml_ui_chunk(command, QML_READ_INITIAL_LEN, 0)?;
        let total_len = first.total_len as usize;
        if total_len > QML_UPLOAD_LIMIT {
            return Err(PackageInstallError::Device(
                "reported QML app is too large".to_owned(),
            ));
        }

        let mut data = Vec::with_capacity(total_len);
        append_qml_chunk(&mut data, &first)?;

        while data.len() < total_len {
            let remaining = total_len - data.len();
            let len = remaining.min(CHUNK_SIZE) as u32;
            let chunk = self.read_qml_ui_chunk(command, len, data.len() as u32)?;
            if chunk.total_len as usize != total_len {
                return Err(malformed_reply("QML readback length changed"));
            }
            append_qml_chunk(&mut data, &chunk)?;
        }

        Ok(data)
    }

    fn read_qml_ui_chunk(
        &self,
        command: u8,
        len: u32,
        offset: u32,
    ) -> Result<QmlUiRead, PackageInstallError> {
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&len.to_be_bytes());
        payload.extend_from_slice(&offset.to_be_bytes());
        let response = self.write_command(command, &payload, QML_READ_TIMEOUT)?;
        parse_qml_ui_read(&response, command)
    }

    fn open_session(&self, target: LoopbackTarget) -> Result<(), PackageInstallError> {
        let mut session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        session.fw_info = session.query_fw_info_with_retries(
            &self.runtime,
            FW_VERSION_OPEN_ATTEMPTS,
            FW_VERSION_OPEN_RETRY_DELAY,
        )?;
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
            session.clear_packet_state();
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
        self.send_with_attempts(command, payload, timeout, WRITE_RETRIES, |response| {
            response_is_ok(response)
        })
    }

    fn send_with_attempts(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
        attempts: usize,
        mut response_is_ok: impl FnMut(&[u8]) -> Result<bool, PackageInstallError>,
    ) -> Result<(), PackageInstallError> {
        debug_assert!(attempts > 0);

        for attempt in 0..attempts {
            eprintln!(
                "package-transport: send {} ({}) attempt {}/{} payload={} timeout={:?}",
                command_name(command),
                command,
                attempt + 1,
                attempts,
                payload.len(),
                timeout
            );
            match self.write_command(command, payload, timeout) {
                Ok(response) => match response_is_ok(&response) {
                    Ok(true) => {
                        eprintln!(
                            "package-transport: ack {} ({}) attempt {}/{} response={}",
                            command_name(command),
                            command,
                            attempt + 1,
                            attempts,
                            response.len()
                        );
                        return Ok(());
                    }
                    Ok(false) if attempt + 1 < attempts => {
                        eprintln!(
                            "package-transport: rejected {} ({}) attempt {}/{}; retrying",
                            command_name(command),
                            command,
                            attempt + 1,
                            attempts
                        );
                        continue;
                    }
                    Ok(false) => {
                        eprintln!(
                            "package-transport: rejected {} ({}) attempt {}/{}",
                            command_name(command),
                            command,
                            attempt + 1,
                            attempts
                        );
                        return Err(PackageInstallError::Device(
                            "device rejected the package write".to_owned(),
                        ));
                    }
                    Err(error) if attempt + 1 < attempts => {
                        eprintln!(
                            "package-transport: invalid {} ({}) response attempt {}/{}: {error}; retrying",
                            command_name(command),
                            command,
                            attempt + 1,
                            attempts
                        );
                        continue;
                    }
                    Err(error) => {
                        eprintln!(
                            "package-transport: invalid {} ({}) response attempt {}/{}: {error}",
                            command_name(command),
                            command,
                            attempt + 1,
                            attempts
                        );
                        return Err(error);
                    }
                },
                Err(error) if attempt + 1 < attempts => {
                    eprintln!(
                        "package-transport: error {} ({}) attempt {}/{}: {error}; retrying",
                        command_name(command),
                        command,
                        attempt + 1,
                        attempts
                    );
                    continue;
                }
                Err(error) => {
                    eprintln!(
                        "package-transport: error {} ({}) attempt {}/{}: {error}",
                        command_name(command),
                        command,
                        attempt + 1,
                        attempts
                    );
                    return Err(error);
                }
            }
        }

        unreachable!("retry loop always returns");
    }

    fn expect_single_ok(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), PackageInstallError> {
        // Source: ~/projects/vesc_tool/codeloader.cpp:101-103 and 731-733
        // send erase once and wait once; only chunk writes retry.
        let response = self.write_command(command, payload, timeout)?;
        match parse_simple_ack(&response, command)? {
            true => Ok(()),
            false => Err(PackageInstallError::Device(
                "device rejected the package write".to_owned(),
            )),
        }
    }

    fn expect_write_ok(
        &self,
        command: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), PackageInstallError> {
        let expected_offset = payload
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_be_bytes)
            .ok_or_else(|| malformed_reply("write payload missing offset"))?;
        self.send_with_retries(command, payload, timeout, |response| {
            parse_write_ack(response, command, expected_offset)
        })
    }

    fn write_without_reply(&self, command: u8, payload: &[u8]) -> Result<(), PackageInstallError> {
        // Source: ~/projects/vesc_tool/codeloader.cpp:1014-1016 and
        // ~/projects/vesc_tool/commands.cpp:2234-2240 send lispSetRunning(1)
        // without waiting for lispRunningResRx.
        let packet = build_command_packet(command, payload);
        self.with_session(|session| {
            session.clear_packet_state();
            self.runtime
                .block_on(write_ble_uart_packet(
                    &session.peripheral,
                    &session.rx_char,
                    &packet,
                ))
                .map_err(|_| {
                    PackageInstallError::Device("failed to write a BLE command".to_owned())
                })
        })
    }
}

impl PackageInstallTransport for BtlePackageInstallTransport {
    fn has_qml_app(&self) -> Result<bool, PackageInstallError> {
        self.with_session(|session| Ok(session.fw_info.has_qml_app))
    }

    fn erase_qml(&self, bytes: usize) -> Result<(), PackageInstallError> {
        self.expect_single_ok(
            COMM_QMLUI_ERASE,
            &(bytes as i32).to_be_bytes(),
            QML_ERASE_RESPONSE_TIMEOUT,
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
        self.expect_single_ok(
            COMM_LISP_ERASE_CODE,
            &(bytes as i32).to_be_bytes(),
            LISP_ERASE_RESPONSE_TIMEOUT,
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

        thread::sleep(POST_LISP_UPLOAD_SETTLE);
        self.with_session(|session| {
            session.clear_packet_state();
            Ok(())
        })?;

        Ok(())
    }

    fn set_running(&self, running: bool) -> Result<(), PackageInstallError> {
        self.write_without_reply(COMM_LISP_SET_RUNNING, &[u8::from(running)])
    }

    fn reload_firmware(&self) -> Result<(), PackageInstallError> {
        // Source: ~/projects/vesc_tool/vescinterface.h:260-263 only marks
        // cached firmware, QML, and config state stale via updateFwRx(false).
        thread::sleep(POST_PACKAGE_INSTALL_SETTLE);
        self.with_session(|session| {
            session.fw_info.has_qml_app = false;
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
    payload.extend_from_slice(&(qml.len() as u32).to_be_bytes());

    let mut crc_input = Vec::with_capacity(2 + qml.len());
    crc_input.extend_from_slice(&fullscreen_flag.to_be_bytes());
    crc_input.extend_from_slice(qml);
    payload.extend_from_slice(&crate::vesc_uart::crc16(&crc_input).to_be_bytes());
    payload.extend_from_slice(&fullscreen_flag.to_be_bytes());
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
    _expected_offset: u32,
) -> Result<bool, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != expected_command {
        return Err(malformed_reply("unexpected BLE reply"));
    }

    let ok = read_u8(&mut cursor)? > 0;
    let _offset = read_u32_be(&mut cursor)?;

    Ok(ok)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LispCodeRead {
    pub(crate) total_len: u32,
    pub(crate) offset: u32,
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QmlUiRead {
    total_len: u32,
    offset: u32,
    data: Vec<u8>,
}

fn parse_qml_ui_read(
    response: &[u8],
    expected_command: u8,
) -> Result<QmlUiRead, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != expected_command {
        return Err(malformed_reply("unexpected BLE reply while reading QML UI"));
    }

    let total_len = read_u32_be(&mut cursor)?;
    let offset = read_u32_be(&mut cursor)?;
    Ok(QmlUiRead {
        total_len,
        offset,
        data: cursor.to_vec(),
    })
}

fn append_qml_chunk(data: &mut Vec<u8>, read: &QmlUiRead) -> Result<(), PackageInstallError> {
    if read.offset as usize != data.len() {
        return Err(malformed_reply("unexpected QML readback offset"));
    }
    if data.len() + read.data.len() > read.total_len as usize {
        return Err(malformed_reply("QML readback chunk exceeds total length"));
    }
    if read.data.is_empty() && data.len() < read.total_len as usize {
        return Err(malformed_reply("empty QML readback chunk"));
    }
    data.extend_from_slice(&read.data);
    Ok(())
}

fn decompress_qml_readback(compressed: &[u8]) -> Result<String, PackageInstallError> {
    let mut cursor = compressed;
    let expected_len = read_u32_be(&mut cursor)? as usize;
    if expected_len > QML_UPLOAD_LIMIT {
        return Err(PackageInstallError::Device(
            "QML readback decompressed length is too large".to_owned(),
        ));
    }
    let mut decoder = ZlibDecoder::new(cursor);
    let mut raw = Vec::with_capacity(expected_len);
    decoder
        .read_to_end(&mut raw)
        .map_err(|error| PackageInstallError::Io(error.to_string()))?;
    if raw.len() != expected_len {
        return Err(malformed_reply("QML readback decompressed length mismatch"));
    }
    String::from_utf8(raw).map_err(|_| malformed_reply("QML readback is not UTF-8"))
}

fn parse_lisp_code_read(response: &[u8]) -> Result<LispCodeRead, PackageInstallError> {
    let mut cursor = response;
    if read_u8(&mut cursor)? != COMM_LISP_READ_CODE {
        return Err(malformed_reply(
            "unexpected BLE reply while reading Lisp code",
        ));
    }

    let total_len = read_u32_be(&mut cursor)?;
    let offset = read_u32_be(&mut cursor)?;
    Ok(LispCodeRead {
        total_len,
        offset,
        data: cursor.to_vec(),
    })
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
        COMM_FW_VERSION, COMM_GET_QML_UI_APP, COMM_LISP_ERASE_CODE, COMM_LISP_READ_CODE,
        COMM_LISP_SET_RUNNING, COMM_LISP_WRITE_CODE, COMM_QMLUI_ERASE, COMM_QMLUI_WRITE,
        FwVersionInfo, HwType, QML_UPLOAD_LIMIT, QmlUiRead, append_qml_chunk, ble_write_chunks,
        build_command_packet, build_lisp_upload_payload, build_qml_upload_payload,
        clear_response_state, decompress_qml_readback, drain_response_channel,
        parse_fw_version_info, parse_lisp_code_read, parse_qml_ui_read, parse_simple_ack,
        parse_write_ack,
    };
    use crate::vesc_uart::PacketDecoder;
    use std::collections::VecDeque;
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
        // Source: ~/projects/vesc_tool/commands.cpp:932-940 and 1008-1016
        // parses one-byte erase ACKs and write ACKs with ok plus echoed offset.
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
        assert!(
            parse_write_ack(&write_ack, COMM_LISP_WRITE_CODE, 384).expect("matching write ack")
        );
        assert!(parse_write_ack(&write_ack, COMM_QMLUI_ERASE, 384).is_err());

        let wrong_command = [COMM_LISP_WRITE_CODE, 1];
        assert!(parse_simple_ack(&wrong_command, COMM_QMLUI_ERASE).is_err());
        assert!(parse_simple_ack(&wrong_command, COMM_LISP_ERASE_CODE).is_err());
        assert!(parse_simple_ack(&[COMM_QMLUI_ERASE], COMM_QMLUI_ERASE).is_err());
    }

    #[test]
    fn write_ack_ignores_device_echoed_offset_like_vesc_tool() {
        let expected_offset = 0;

        // Source: ~/projects/vesc_tool/codeloader.cpp:409-412 and 766-769
        // both upload callbacks explicitly ignore the echoed write offset.
        for echoed_offset in [0, 384, u32::MAX] {
            let mut write_ack = Vec::from([COMM_LISP_WRITE_CODE, 1]);
            write_ack.extend_from_slice(&echoed_offset.to_be_bytes());

            assert!(
                parse_write_ack(&write_ack, COMM_LISP_WRITE_CODE, expected_offset).expect(
                    "VESC Tool treats ok write ACKs as success regardless of echoed offset"
                )
            );
        }
    }

    #[test]
    fn parses_lisp_code_read_replies() {
        let mut response = Vec::new();
        response.push(COMM_LISP_READ_CODE);
        response.extend_from_slice(&1024_u32.to_be_bytes());
        response.extend_from_slice(&384_u32.to_be_bytes());
        response.extend_from_slice(b"(print \"hello\")");

        let read = parse_lisp_code_read(&response).expect("readback reply");

        assert_eq!(read.total_len, 1024);
        assert_eq!(read.offset, 384);
        assert_eq!(read.data, b"(print \"hello\")");
        assert!(parse_lisp_code_read(&[COMM_LISP_READ_CODE, 0]).is_err());
    }

    #[test]
    fn parses_qml_app_read_replies() {
        let mut response = Vec::new();
        response.push(COMM_GET_QML_UI_APP);
        response.extend_from_slice(&1024_u32.to_be_bytes());
        response.extend_from_slice(&384_u32.to_be_bytes());
        response.extend_from_slice(b"qml");

        let read = parse_qml_ui_read(&response, COMM_GET_QML_UI_APP).expect("qml reply");

        assert_eq!(read.total_len, 1024);
        assert_eq!(read.offset, 384);
        assert_eq!(read.data, b"qml");
        assert!(parse_qml_ui_read(&[COMM_GET_QML_UI_APP, 0], COMM_GET_QML_UI_APP).is_err());
    }

    #[test]
    fn append_qml_chunk_rejects_data_past_reported_total() {
        let mut data = vec![1, 2];
        let read = QmlUiRead {
            total_len: 3,
            offset: 2,
            data: vec![3, 4],
        };

        assert!(append_qml_chunk(&mut data, &read).is_err());
        assert_eq!(data, vec![1, 2]);
    }

    #[test]
    fn decompress_qml_readback_rejects_oversized_lengths_before_allocating() {
        let mut compressed = ((QML_UPLOAD_LIMIT + 1) as u32).to_be_bytes().to_vec();
        compressed.extend_from_slice(b"not zlib");

        let error = decompress_qml_readback(&compressed).expect_err("oversized readback");

        assert_eq!(
            error.to_string(),
            "device error: QML readback decompressed length is too large"
        );
    }

    #[test]
    fn rejects_oversized_qml_uploads() {
        let qml = vec![0_u8; 1024 * 120];
        // Source: ~/projects/vesc_tool/codeloader.cpp:804-807
        // qmlUpload() rejects payloads larger than 1024 * 120 after headers.
        assert!(build_qml_upload_payload(&qml, false).is_err());
    }

    #[test]
    fn qml_upload_payload_matches_vesc_tool_header_order() {
        let qml = [0xAA, 0xBB, 0xCC];
        let payload = build_qml_upload_payload(&qml, true).expect("qml payload");
        let mut crc_input = Vec::from(2_u16.to_be_bytes());
        crc_input.extend_from_slice(&qml);

        // Source: ~/projects/vesc_tool/codeloader.cpp:794-802
        // qmlUpload() writes len, crc, fullscreen flag, then script bytes.
        assert_eq!(&payload[0..4], &(qml.len() as u32).to_be_bytes());
        assert_eq!(
            &payload[4..6],
            &crate::vesc_uart::crc16(&crc_input).to_be_bytes()
        );
        assert_eq!(&payload[6..8], &2_u16.to_be_bytes());
        assert_eq!(&payload[8..], &qml);
    }

    #[test]
    fn lisp_upload_limits_depend_on_hardware_type() {
        let vesc_lisp = vec![0_u8; 1024 * 128];
        // Source: ~/projects/vesc_tool/codeloader.cpp:389-397
        // lispUpload() uses 128 KiB - 6 bytes for VESC and 512 KiB - 6 for ESP32.
        assert!(build_lisp_upload_payload(&vesc_lisp, HwType::Vesc).is_err());

        let custom_lisp = vec![0_u8; 1024 * 128];
        assert!(build_lisp_upload_payload(&custom_lisp, HwType::CustomModule).is_ok());
    }

    #[test]
    fn lisp_upload_payload_matches_flash_helper_header_contract() {
        let mut lisp_data = Vec::new();
        lisp_data.extend_from_slice(&0_u16.to_be_bytes());
        lisp_data.extend_from_slice(b"(print \"hello\")\0");
        lisp_data.extend_from_slice(&0_i16.to_be_bytes());

        let payload = build_lisp_upload_payload(&lisp_data, HwType::Vesc).expect("lisp payload");

        // Source: ~/projects/vesc_tool/codeloader.cpp:383-387
        // lispUpload() writes len as vb.size() - 2, crc over vb, then vb bytes.
        assert_eq!(
            u32::from_be_bytes(payload[0..4].try_into().expect("length header")),
            (lisp_data.len() - 2) as u32
        );
        assert_eq!(
            u16::from_be_bytes(payload[4..6].try_into().expect("crc header")),
            crate::vesc_uart::crc16(&lisp_data)
        );
        assert_eq!(&payload[6..8], &0_u16.to_be_bytes());
        assert_eq!(&payload[8..], &lisp_data[2..]);
    }

    #[test]
    fn erase_command_packets_match_vesc_tool() {
        for bytes in [16_i32, 4096_i32] {
            let payload = bytes.to_be_bytes();
            let expected_tail = payload.to_vec();
            for command in [COMM_QMLUI_ERASE, COMM_LISP_ERASE_CODE] {
                // Source: ~/projects/vesc_tool/commands.cpp:2122-2127 and
                // ~/projects/vesc_tool/commands.cpp:2226-2231. Both erase
                // commands encode command byte plus big-endian int32 size.
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

        // Source: ~/projects/vesc_tool/bleuart.cpp:134-147
        // BleUart::writeData() sends repeated 20-byte chunks then the remainder.
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
    fn clear_response_state_drops_stale_write_acks_before_next_command() {
        let mut pending = VecDeque::from([vec![COMM_QMLUI_WRITE, 1, 0, 0, 0, 0]]);
        let mut decoder = PacketDecoder::new();
        let stale_packet = build_command_packet(COMM_QMLUI_WRITE, &[1, 0, 0, 1, 128]);
        decoder.push(&stale_packet).expect("valid stale packet");
        decoder
            .push(&stale_packet[..3])
            .expect("partial stale packet");
        let (tx, rx) = mpsc::channel();
        tx.send(stale_packet.clone())
            .expect("queued stale notification");
        drop(tx);

        clear_response_state(&mut pending, &mut decoder, &rx);

        assert!(pending.is_empty());
        assert!(decoder.pop_ready().is_none());
        assert!(
            decoder
                .push(&stale_packet[3..])
                .expect("old tail")
                .is_empty()
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lisp_set_running_packet_matches_vesc_tool() {
        // Source: ~/projects/vesc_tool/commands.cpp:2234-2240
        // lispSetRunning() encodes command byte plus int8 running flag.
        let packet = build_command_packet(COMM_LISP_SET_RUNNING, &[1]);
        let decoded = PacketDecoder::new()
            .push(&packet)
            .expect("valid packet")
            .pop()
            .expect("complete packet");

        assert_eq!(decoded, vec![COMM_LISP_SET_RUNNING, 1]);
    }

    #[test]
    fn erase_waits_match_vesc_tool_package_installer() {
        // Source: ~/projects/vesc_tool/codeloader.cpp:711-731
        // qmlErase() waits up to 6000 ms for one erase response.
        assert_eq!(
            super::QML_ERASE_RESPONSE_TIMEOUT,
            std::time::Duration::from_secs(6)
        );
        // Source: ~/projects/vesc_tool/codeloader.cpp:81-101
        // lispErase() waits up to 8000 ms for one erase response.
        assert_eq!(
            super::LISP_ERASE_RESPONSE_TIMEOUT,
            std::time::Duration::from_secs(8)
        );
    }

    #[test]
    fn write_retry_count_matches_vesc_tool_chunk_uploads() {
        // Source: ~/projects/vesc_tool/codeloader.cpp:423-432 and 780-789
        // both writeChunk lambdas use int tries = 5.
        assert_eq!(super::WRITE_RETRIES, 5);
    }

    #[test]
    fn package_install_refresh_wait_matches_vesc_tool() {
        // Source: ~/projects/vesc_tool/codeloader.cpp:1023-1024
        // installVescPackage() sleeps 500 ms before reloadFirmware().
        assert_eq!(
            super::POST_PACKAGE_INSTALL_SETTLE,
            std::time::Duration::from_millis(500)
        );
    }
}
