//! Verbose BLE loopback diagnostics for hardware bring-up.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use tokio::runtime::{Builder, Runtime};
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
static REFLOAT_ALL_DATA_MODE4_PAYLOAD: [u8; 3] = [101, 10, 4];
const REFLOAT_ALL_DATA_MODE4_REQUEST: CustomAppDataRequest = CustomAppDataRequest {
    step: "all-data",
    payload: &REFLOAT_ALL_DATA_MODE4_PAYLOAD,
};
// QML asks for Refloat realtime IDs with `[101, 32]` at
// `refloat/ui.qml.in:704-705`; the app-data handler dispatches it at
// `refloat/src/main.c:2275-2277`.
static REFLOAT_REALTIME_DATA_IDS_PAYLOAD: [u8; 2] = [101, 32];
const REFLOAT_REALTIME_DATA_IDS_REQUEST: CustomAppDataRequest = CustomAppDataRequest {
    step: "realtime-data-ids",
    payload: &REFLOAT_REALTIME_DATA_IDS_PAYLOAD,
};
// QML samples Refloat realtime data with `[101, 31]` every 100 ms at
// `refloat/ui.qml.in:157-170` and `refloat/ui.qml.in:699-704`; Refloat C
// dispatches the packet at `refloat/src/main.c:2223-2225`.
static REFLOAT_REALTIME_DATA_PAYLOAD: [u8; 2] = [101, 31];
const REFLOAT_REALTIME_DATA_REQUEST: CustomAppDataRequest = CustomAppDataRequest {
    step: "realtime-data",
    payload: &REFLOAT_REALTIME_DATA_PAYLOAD,
};

/// Decoded Refloat realtime-data ID response lists from
/// `refloat/src/main.c:1876-1901`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatRealtimeDataIds {
    /// Items sent in every realtime-data packet.
    pub always: Vec<String>,
    /// Items sent only while the package reports runtime data.
    pub runtime: Vec<String>,
}

/// Decoded Refloat realtime sample as consumed by the QML plot.
///
/// QML appends this row shape at
/// `refloat/ui.qml.in:922-924` after decoding the packet fields at
/// `refloat/ui.qml.in:860-911`.
#[derive(Debug, Clone, PartialEq)]
pub struct RefloatRealtimeSample {
    /// VESC system timestamp in 100 us ticks.
    pub timestamp_ticks: u32,
    /// Raw package mode from QML's `modeAndState >> 4`.
    pub package_mode: u8,
    /// Raw package state from QML's `modeAndState & 0x0f`.
    pub package_state: u8,
    /// Whether QML sees runtime values in this packet.
    pub has_runtime: bool,
    /// Whether QML sees the package in RUNNING state.
    pub running: bool,
    /// Whether QML sees wheelslip active.
    pub wheelslip: bool,
    /// Refloat-compatible setpoint-adjustment id.
    pub setpoint_adjustment: u8,
    /// Refloat-compatible footpad switch id.
    pub footpad_switch: u8,
    /// QML realtime values keyed by the decoded realtime ID strings.
    pub values: Vec<(String, f32)>,
}

impl RefloatRealtimeSample {
    /// Return a realtime value by the QML/Refloat ID string.
    ///
    /// QML builds the value-name table from realtime ID lists at
    /// `refloat/ui.qml.in:929-931`.
    pub fn value(&self, name: &str) -> Option<f32> {
        self.values
            .iter()
            .find_map(|(key, value)| (key == name).then_some(*value))
    }
}

/// Decoded fields from Refloat compact all-data mode 4 from
/// `refloat/src/main.c:1313-1399`.
#[derive(Debug, Clone, PartialEq)]
pub struct RefloatAllDataSnapshot {
    /// Refloat-compatible float state id.
    pub float_state: u8,
    /// Refloat-compatible setpoint-adjustment id.
    pub setpoint_adjustment: u8,
    /// Refloat-compatible footpad switch id.
    pub footpad_switch: u8,
    /// Whether the legacy HANDTEST bit is set in switch state.
    pub handtest: bool,
    /// Balance current in amps.
    pub balance_current_amps: f32,
    /// Balance pitch in radians.
    pub balance_pitch_rad: f32,
    /// Roll in radians.
    pub roll_rad: f32,
    /// Pitch in radians.
    pub pitch_rad: f32,
    /// ADC1 volts.
    pub adc1_volts: f32,
    /// ADC2 volts.
    pub adc2_volts: f32,
    /// Battery voltage.
    pub battery_voltage: f32,
    /// Motor ERPM.
    pub erpm: i16,
    /// Motor current in amps.
    pub motor_current_amps: f32,
    /// Battery current in amps.
    pub battery_current_amps: f32,
}

/// Refloat smoke probe result.
#[derive(Debug, Clone, PartialEq)]
pub struct RefloatProbeReport {
    /// Decoded realtime item IDs used by QML and Float Control.
    pub realtime_ids: RefloatRealtimeDataIds,
    /// Decoded realtime samples from the same QML data-log path.
    pub realtime_samples: Vec<RefloatRealtimeSample>,
    /// Decoded compact all-data fields used for hardware proof.
    pub all_data_snapshot: RefloatAllDataSnapshot,
    /// Raw mode-4 all-data response.
    pub all_data: Vec<u8>,
}

/// Refloat probe collection options.
///
/// Defaults mirror QML's realtime timer at
/// `refloat/ui.qml.in:157-170`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatProbeOptions {
    /// Number of QML realtime-log samples to collect.
    pub realtime_sample_count: usize,
    /// Delay between QML realtime-log samples.
    pub realtime_sample_interval: Duration,
}

impl Default for RefloatProbeOptions {
    fn default() -> Self {
        Self {
            realtime_sample_count: 1,
            realtime_sample_interval: Duration::from_millis(100),
        }
    }
}

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
            Self::ScanSeen { device, matched } => describe_scan_seen(device, *matched),
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
                describe_waiting_for_reply(step, *timeout_secs)
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
            } => describe_timeout_waiting(step, *elapsed_secs, pending),
        }
    }
}

fn describe_scan_seen(device: &str, matched: bool) -> String {
    match matched {
        true => format!("scan match: {device}"),
        false => format!("scan seen: {device}"),
    }
}

fn describe_waiting_for_reply(step: &str, timeout_secs: u64) -> String {
    match step {
        "fw-version-preflight" => {
            format!("step {step}: waiting up to {timeout_secs}s for COMM_FW_VERSION reply")
        }
        _ => format!("step {step}: waiting up to {timeout_secs}s for COMM_CUSTOM_APP_DATA reply"),
    }
}

fn describe_timeout_waiting(step: &str, elapsed_secs: u64, pending: &[String]) -> String {
    match pending {
        [] => format!("step {step}: timed out after {elapsed_secs}s with no VESC packets received"),
        packets => format!(
            "step {step}: timed out after {elapsed_secs}s; pending packets: {}",
            packets.join("; ")
        ),
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

/// Decode Refloat `COMMAND_REALTIME_DATA_IDS`.
///
/// Refloat C writes two counted string lists in
/// `refloat/src/main.c:1876-1901`, with string framing from
/// `refloat/src/conf/buffer.c:147-155` and ID order from
/// `refloat/src/rt_data.h:38-66`. QML reads the same lists in
/// `refloat/ui.qml.in:926-934`.
pub fn decode_refloat_realtime_data_ids_response(
    payload: &[u8],
) -> Result<RefloatRealtimeDataIds, &'static str> {
    let mut index = 0;
    let package_id = take_u8(payload, &mut index)?;
    let command = take_u8(payload, &mut index)?;
    if package_id != 101 || command != 32 {
        return Err("not a Refloat realtime-data IDs response");
    }

    Ok(RefloatRealtimeDataIds {
        always: take_string_list(payload, &mut index)?,
        runtime: take_string_list(payload, &mut index)?,
    })
}

/// Decode Refloat `COMMAND_REALTIME_DATA` exactly like the QML data plot.
///
/// Refloat C writes this packet in
/// `refloat/src/main.c:1904-1960`; QML decodes the packet and appends
/// `rtPlotData` samples at `refloat/ui.qml.in:852-925`.
pub fn decode_refloat_realtime_data_response(
    payload: &[u8],
    ids: &RefloatRealtimeDataIds,
) -> Result<RefloatRealtimeSample, &'static str> {
    let mut index = 0;
    let package_id = take_u8(payload, &mut index)?;
    let command = take_u8(payload, &mut index)?;
    if package_id != 101 || command != 31 {
        return Err("not a Refloat realtime-data response");
    }

    let mask = take_u8(payload, &mut index)?;
    let has_runtime = mask & 0x01 != 0;
    let has_charging = mask & 0x02 != 0;
    let has_alerts = mask & 0x04 != 0;
    let _extra_flags = take_u8(payload, &mut index)?;
    let timestamp_ticks = take_u32(payload, &mut index)?;
    let mode_and_state = take_u8(payload, &mut index)?;
    let sensor_and_flags = take_u8(payload, &mut index)?;
    let setpoint_and_stop = take_u8(payload, &mut index)?;
    let _beep_reason = take_u8(payload, &mut index)?;

    let runtime_ids: &[String] = if has_runtime { &ids.runtime } else { &[] };
    let values = ids
        .always
        .iter()
        .chain(runtime_ids.iter())
        .map(|name| take_float16_auto(payload, &mut index).map(|value| (name.clone(), value)))
        .collect::<Result<Vec<_>, _>>()?;
    if has_charging {
        take_float16_auto(payload, &mut index)?;
        take_float16_auto(payload, &mut index)?;
    }
    if has_alerts {
        let end = index.saturating_add(9);
        payload
            .get(index..end)
            .ok_or("truncated realtime-data alert tail")?;
    }

    // QML assigns `pkgState = modeAndState & 0x0f` at
    // `refloat/ui.qml.in:874-878` and defines RUNNING as state id 3 at
    // `refloat/ui.qml.in:1711-1722`.
    Ok(RefloatRealtimeSample {
        timestamp_ticks,
        package_mode: mode_and_state >> 4,
        package_state: mode_and_state & 0x0f,
        has_runtime,
        running: mode_and_state & 0x0f == 3,
        wheelslip: sensor_and_flags & 0x01 != 0,
        setpoint_adjustment: setpoint_and_stop >> 4,
        footpad_switch: sensor_and_flags >> 6,
        values,
    })
}

/// Decode Refloat compact all-data mode 4.
///
/// Refloat C writes this packet in `refloat/src/main.c:1313-1399`: state
/// and HANDTEST are at `refloat/src/main.c:1333-1341`, ADCs at
/// `refloat/src/main.c:1344-1345`, setpoints at
/// `refloat/src/main.c:1347-1353`, pitch/booster at
/// `refloat/src/main.c:1355-1356`, motor fields at
/// `refloat/src/main.c:1358-1364`, and mode-4 charge fields at
/// `refloat/src/main.c:1391-1395`.
pub fn decode_refloat_all_data_mode4_response(
    payload: &[u8],
) -> Result<RefloatAllDataSnapshot, &'static str> {
    let mut index = 0;
    let package_id = take_u8(payload, &mut index)?;
    let command = take_u8(payload, &mut index)?;
    let mode = take_u8(payload, &mut index)?;
    if package_id != 101 || command != 10 || mode != 4 {
        return Err("not a Refloat all-data mode 4 response");
    }

    let balance_current_amps = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let balance_pitch_rad = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let roll_rad = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let state = take_u8(payload, &mut index)?;
    let switch = take_u8(payload, &mut index)? & 0x0f;
    let adc1_volts = f32::from(take_u8(payload, &mut index)?) / 50.0;
    let adc2_volts = f32::from(take_u8(payload, &mut index)?) / 50.0;
    // Refloat C writes six compact setpoint bytes at
    // `refloat/src/main.c:1347-1353`; they are not needed for pre-bench proof.
    (0..6).try_for_each(|_| take_u8(payload, &mut index).map(drop))?;
    let pitch_rad = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let _booster_current = take_u8(payload, &mut index)?;
    let battery_voltage = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let erpm = take_i16(payload, &mut index)?;
    let _vehicle_speed = take_i16(payload, &mut index)?;
    let motor_current_amps = f32::from(take_i16(payload, &mut index)?) / 10.0;
    let battery_current_amps = f32::from(take_i16(payload, &mut index)?) / 10.0;

    Ok(RefloatAllDataSnapshot {
        float_state: state & 0x0f,
        setpoint_adjustment: state >> 4,
        footpad_switch: switch & 0x07,
        handtest: switch & 0x08 != 0,
        balance_current_amps,
        balance_pitch_rad,
        roll_rad,
        pitch_rad,
        adc1_volts,
        adc2_volts,
        battery_voltage,
        erpm,
        motor_current_amps,
        battery_current_amps,
    })
}

fn take_u8(payload: &[u8], index: &mut usize) -> Result<u8, &'static str> {
    let byte = payload
        .get(*index)
        .copied()
        .ok_or("truncated realtime-data IDs response")?;
    *index = index.saturating_add(1);
    Ok(byte)
}

fn take_i16(payload: &[u8], index: &mut usize) -> Result<i16, &'static str> {
    let start = *index;
    let end = start.saturating_add(2);
    let bytes = payload
        .get(start..end)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .ok_or("truncated Refloat response")?;
    *index = end;
    Ok(i16::from_be_bytes(bytes))
}

fn take_u16(payload: &[u8], index: &mut usize) -> Result<u16, &'static str> {
    let start = *index;
    let end = start.saturating_add(2);
    let bytes = payload
        .get(start..end)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .ok_or("truncated Refloat response")?;
    *index = end;
    Ok(u16::from_be_bytes(bytes))
}

fn take_u32(payload: &[u8], index: &mut usize) -> Result<u32, &'static str> {
    let start = *index;
    let end = start.saturating_add(4);
    let bytes = payload
        .get(start..end)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .ok_or("truncated Refloat response")?;
    *index = end;
    Ok(u32::from_be_bytes(bytes))
}

fn take_float16_auto(payload: &[u8], index: &mut usize) -> Result<f32, &'static str> {
    // QML uses the same half-float expansion at
    // `refloat/ui.qml.in:653-662`; Refloat C writes these values with
    // `buffer_append_float16_auto` at `refloat/src/conf/buffer.c:142-143`.
    let bits = take_u16(payload, index)?;
    Ok(float16_auto_to_f32(bits))
}

fn float16_auto_to_f32(bits: u16) -> f32 {
    let sign = (u32::from(bits & 0x8000)) << 16;
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = u32::from(bits & 0x03ff);
    let float_bits = match (exponent, mantissa) {
        (0, 0) => sign,
        (0, _) => {
            let shift = mantissa.leading_zeros() - 21;
            let normalized_mantissa = (mantissa << (shift + 1)) & 0x03ff;
            let normalized_exponent = 127 - 14 - shift;
            sign | (normalized_exponent << 23) | (normalized_mantissa << 13)
        }
        (0x1f, _) => sign | 0x7f80_0000 | (mantissa << 13),
        _ => sign | ((u32::from(exponent) + 112) << 23) | (mantissa << 13),
    };
    f32::from_bits(float_bits)
}

fn take_string_list(payload: &[u8], index: &mut usize) -> Result<Vec<String>, &'static str> {
    let count = usize::from(take_u8(payload, index)?);
    (0..count).map(|_| take_string(payload, index)).collect()
}

fn take_string(payload: &[u8], index: &mut usize) -> Result<String, &'static str> {
    let len = usize::from(take_u8(payload, index)?);
    let start = *index;
    let end = start.saturating_add(len);
    let bytes = payload
        .get(start..end)
        .ok_or("truncated realtime-data ID string")?;
    *index = end;
    core::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| "invalid realtime-data ID UTF-8")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CustomAppDataRequest {
    step: &'static str,
    payload: &'static [u8],
}

impl CustomAppDataRequest {
    fn send(
        self,
        runtime: &Runtime,
        session: &mut DiagnosticSession,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Vec<u8>, LoopbackTransportError> {
        let wire = build_custom_app_data_packet(self.payload);
        progress(LoopbackProgress::StepStarted { step: self.step });
        progress(LoopbackProgress::Sending {
            step: self.step,
            wire_hex: hex_snippet(&wire, 48),
            bytes: wire.len(),
        });
        runtime.block_on(write_ble_uart_packet(
            &session.peripheral,
            &session.rx_char,
            &wire,
        ))?;

        session.receive_custom_app_data_payload_with_progress(self.step, progress)
    }
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

/// Sends Refloat realtime-ID and all-data app-data probes.
///
/// The first request mirrors QML startup at `refloat/ui.qml.in:704-705`; the
/// second asks C for compact mode-4 data through
/// `refloat/src/main.c:2210-2213`, decoded from the packet layout in
/// `refloat/src/main.c:1313-1399`.
pub fn run_refloat_probe_with_diagnostics(
    target: LoopbackTarget,
    progress: impl FnMut(LoopbackProgress),
) -> Result<RefloatProbeReport, LoopbackTransportError> {
    run_refloat_probe_with_options(target, RefloatProbeOptions::default(), progress)
}

/// Sends Refloat realtime-ID, QML realtime-log, and all-data app-data probes.
///
/// Refloat QML requests realtime IDs once at
/// `refloat/ui.qml.in:703-704`, samples realtime data on its 100 ms timer at
/// `refloat/ui.qml.in:157-170`, and decodes samples at
/// `refloat/ui.qml.in:852-925`.
pub fn run_refloat_probe_with_options(
    target: LoopbackTarget,
    options: RefloatProbeOptions,
    mut progress: impl FnMut(LoopbackProgress),
) -> Result<RefloatProbeReport, LoopbackTransportError> {
    with_diagnostic_session(target, &mut progress, |runtime, session, progress| {
        session.clear_pending();
        let realtime_ids = REFLOAT_REALTIME_DATA_IDS_REQUEST
            .send(runtime, session, progress)
            .and_then(|response| {
                decode_refloat_realtime_data_ids_response(&response).map_err(|error| {
                    LoopbackTransportError::Device(format!(
                        "{error}: response={}",
                        hex_snippet(&response, 96)
                    ))
                })
            })?;
        let realtime_samples = (0..options.realtime_sample_count)
            .map(|sample_index| {
                if sample_index != 0 {
                    std::thread::sleep(options.realtime_sample_interval);
                }
                session.clear_pending();
                REFLOAT_REALTIME_DATA_REQUEST
                    .send(runtime, session, progress)
                    .and_then(|response| {
                        decode_refloat_realtime_data_response(&response, &realtime_ids).map_err(
                            |error| {
                                LoopbackTransportError::Device(format!(
                                    "{error}: response={}",
                                    hex_snippet(&response, 96)
                                ))
                            },
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        session.clear_pending();
        let all_data = REFLOAT_ALL_DATA_MODE4_REQUEST.send(runtime, session, progress)?;
        let all_data_snapshot = decode_refloat_all_data_mode4_response(&all_data)
            .map_err(|error| LoopbackTransportError::Device(error.to_owned()))?;

        Ok(RefloatProbeReport {
            realtime_ids,
            realtime_samples,
            all_data_snapshot,
            all_data,
        })
    })
}

fn with_diagnostic_session<P, R>(
    target: LoopbackTarget,
    progress: &mut P,
    run: impl FnOnce(&Runtime, &mut DiagnosticSession, &mut P) -> Result<R, LoopbackTransportError>,
) -> Result<R, LoopbackTransportError>
where
    P: FnMut(LoopbackProgress),
{
    progress(LoopbackProgress::StartingRuntime);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .map_err(|_| device_error("failed to start the BLE runtime"))?;

    progress(LoopbackProgress::OpeningSession {
        target: describe_loopback_target(&target),
    });
    let mut session = runtime.block_on(open_session_with_progress(target, progress))?;
    progress(LoopbackProgress::SessionOpened);
    std::thread::sleep(POST_INSTALL_SETTLE);

    runtime.block_on(run_fw_version_preflight(&mut session, progress))?;
    session.clear_pending();
    run(&runtime, &mut session, progress)
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
        self.decoder.clear();
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
        let payload = self.receive_custom_app_data_payload_with_progress(step, progress)?;
        let wire_hex = hex_snippet(&payload, 32);
        let command = decode_loopback_command(&payload)?;
        progress(LoopbackProgress::ReplyReceived {
            step,
            wire_hex,
            command,
        });
        Ok(payload)
    }

    fn take_pending_response(&mut self) -> Option<Vec<u8>> {
        self.pending
            .iter()
            .position(|packet| matches!(packet.as_slice(), [COMM_CUSTOM_APP_DATA, ..]))
            .and_then(|index| self.pending.remove(index))
            .and_then(|packet| custom_app_data_payload(&packet))
    }

    fn receive_custom_app_data_payload_with_progress(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Vec<u8>, LoopbackTransportError> {
        self.take_pending_response()
            .map(Ok)
            .unwrap_or_else(|| self.receive_custom_app_data_from_notifications(step, progress))
    }

    fn receive_custom_app_data_from_notifications(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Vec<u8>, LoopbackTransportError> {
        progress(LoopbackProgress::WaitingForReply {
            step,
            timeout_secs: RESPONSE_TIMEOUT.as_secs(),
        });

        let started = std::time::Instant::now();
        std::iter::repeat_with(|| self.next_custom_app_data_payload(step, started, progress))
            .find_map(Result::transpose)
            .expect("repeat_with produces an infinite iterator")
    }

    fn next_custom_app_data_payload(
        &mut self,
        step: &'static str,
        started: std::time::Instant,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Option<Vec<u8>>, LoopbackTransportError> {
        self.take_decoded_custom_app_data(step, progress)
            .map(Some)
            .map(Ok)
            .unwrap_or_else(|| self.receive_custom_app_data_notification(step, started, progress))
    }

    fn receive_custom_app_data_notification(
        &mut self,
        step: &'static str,
        started: std::time::Instant,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Option<Vec<u8>>, LoopbackTransportError> {
        match self.responses.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(bytes) => self.decode_notification(step, bytes, progress),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let pending: Vec<String> = self
                    .pending
                    .iter()
                    .map(|packet| describe_vesc_packet(packet))
                    .collect();
                progress(LoopbackProgress::TimeoutWaiting {
                    step,
                    elapsed_secs: started.elapsed().as_secs(),
                    pending: pending.clone(),
                });
                Err(timeout_error(step, started.elapsed(), &pending))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(device_error("BLE notification stream disconnected"))
            }
        }
    }

    fn decode_notification(
        &mut self,
        step: &'static str,
        bytes: Vec<u8>,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Result<Option<Vec<u8>>, LoopbackTransportError> {
        progress(LoopbackProgress::ReceivedNotification {
            step,
            bytes: bytes.len(),
        });
        let packets = self
            .decoder
            .push(&bytes)
            .map_err(|_| device_error("failed to decode a VESC packet"))?;
        progress(LoopbackProgress::DecodedPackets {
            step,
            count: packets.len(),
        });
        Ok(self.take_custom_app_data_from_packets(step, progress, packets))
    }

    fn take_decoded_custom_app_data(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
    ) -> Option<Vec<u8>> {
        self.decoder
            .pop_ready()
            .and_then(|packet| self.store_or_take_custom_app_data(step, progress, packet))
    }

    fn take_custom_app_data_from_packets(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
        packets: Vec<Vec<u8>>,
    ) -> Option<Vec<u8>> {
        packets
            .into_iter()
            .find_map(|packet| self.store_or_take_custom_app_data(step, progress, packet))
    }

    fn store_or_take_custom_app_data(
        &mut self,
        step: &'static str,
        progress: &mut impl FnMut(LoopbackProgress),
        packet: Vec<u8>,
    ) -> Option<Vec<u8>> {
        match custom_app_data_payload(&packet) {
            Some(payload) => Some(payload),
            None => {
                progress(LoopbackProgress::NonAppDataPacket {
                    step,
                    summary: describe_vesc_packet(&packet),
                });
                self.pending.push_back(packet);
                None
            }
        }
    }
}

fn custom_app_data_payload(packet: &[u8]) -> Option<Vec<u8>> {
    match packet {
        [COMM_CUSTOM_APP_DATA, payload @ ..] => Some(payload.to_vec()),
        _ => None,
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
    use super::{
        COMM_CUSTOM_APP_DATA, LoopbackProgress, REFLOAT_ALL_DATA_MODE4_REQUEST,
        REFLOAT_REALTIME_DATA_IDS_REQUEST, REFLOAT_REALTIME_DATA_REQUEST, RefloatRealtimeDataIds,
        build_custom_app_data_packet, decode_refloat_all_data_mode4_response,
        decode_refloat_realtime_data_ids_response, decode_refloat_realtime_data_response,
        describe_vesc_packet, hex_snippet, timeout_error,
    };
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
    fn refloat_probe_uses_all_data_mode4_request() {
        let wire = build_custom_app_data_packet(REFLOAT_ALL_DATA_MODE4_REQUEST.payload);
        let decoded = crate::vesc_uart::PacketDecoder::new()
            .push(&wire)
            .expect("packet")
            .pop()
            .expect("ready");

        assert_eq!(decoded, [COMM_CUSTOM_APP_DATA, 101, 10, 4]);
    }

    #[test]
    fn refloat_probe_uses_realtime_data_ids_request() {
        let wire = build_custom_app_data_packet(REFLOAT_REALTIME_DATA_IDS_REQUEST.payload);
        let decoded = crate::vesc_uart::PacketDecoder::new()
            .push(&wire)
            .expect("packet")
            .pop()
            .expect("ready");

        assert_eq!(decoded, [COMM_CUSTOM_APP_DATA, 101, 32]);
    }

    #[test]
    fn refloat_probe_uses_qml_realtime_data_request() {
        let wire = build_custom_app_data_packet(REFLOAT_REALTIME_DATA_REQUEST.payload);
        let decoded = crate::vesc_uart::PacketDecoder::new()
            .push(&wire)
            .expect("packet")
            .pop()
            .expect("ready");

        assert_eq!(decoded, [COMM_CUSTOM_APP_DATA, 101, 31]);
    }

    #[test]
    fn refloat_realtime_id_decoder_matches_qml_string_lists() {
        let payload = [
            101, 32, 2, 11, b'm', b'o', b't', b'o', b'r', b'.', b's', b'p', b'e', b'e', b'd', 10,
            b'm', b'o', b't', b'o', b'r', b'.', b'e', b'r', b'p', b'm', 1, 8, b's', b'e', b't',
            b'p', b'o', b'i', b'n', b't',
        ];

        let ids = decode_refloat_realtime_data_ids_response(&payload).expect("ids");

        assert_eq!(ids.always, ["motor.speed", "motor.erpm"]);
        assert_eq!(ids.runtime, ["setpoint"]);
    }

    #[test]
    fn refloat_realtime_data_decoder_matches_qml_log_sample() {
        let ids = RefloatRealtimeDataIds {
            always: vec!["imu.pitch".to_owned(), "motor.current".to_owned()],
            runtime: vec!["balance_current".to_owned(), "setpoint".to_owned()],
        };
        let payload = [
            101, 31, 1, 0, 0, 0, 3, 0xe8, 0x13, 0xc1, 0x20, 0, 0x3c, 0, 0xc0, 0, 0x38, 0, 0x42, 0,
        ];

        let sample = decode_refloat_realtime_data_response(&payload, &ids).expect("rt sample");

        assert_eq!(sample.timestamp_ticks, 1000);
        assert_eq!(sample.package_mode, 1);
        assert_eq!(sample.package_state, 3);
        assert!(sample.has_runtime);
        assert!(sample.running);
        assert!(sample.wheelslip);
        assert_eq!(sample.setpoint_adjustment, 2);
        assert_eq!(sample.footpad_switch, 3);
        assert_eq!(sample.value("imu.pitch"), Some(1.0));
        assert_eq!(sample.value("motor.current"), Some(-2.0));
        assert_eq!(sample.value("balance_current"), Some(0.5));
        assert_eq!(sample.value("setpoint"), Some(3.0));
    }

    #[test]
    fn refloat_all_data_decoder_reports_pre_bench_state_fields() {
        let payload = [
            101, 10, 4, 0, 25, 0, 2, 0xff, 0xfd, 0x2b, 0x0a, 125, 0, 128, 128, 128, 128, 128, 128,
            0, 12, 128, 0x02, 0xd2, 0x04, 0xd2, 0, 0, 0, 55, 0xff, 0xf4, 128, 222,
        ];

        let snapshot = decode_refloat_all_data_mode4_response(&payload).expect("all data");

        assert_eq!(snapshot.float_state, 11);
        assert_eq!(snapshot.setpoint_adjustment, 2);
        assert_eq!(snapshot.footpad_switch, 2);
        assert!(snapshot.handtest);
        assert_eq!(snapshot.balance_current_amps, 2.5);
        assert_eq!(snapshot.pitch_rad, 1.2);
        assert_eq!(snapshot.roll_rad, -0.3);
        assert_eq!(snapshot.adc1_volts, 2.5);
        assert_eq!(snapshot.battery_voltage, 72.2);
        assert_eq!(snapshot.erpm, 1234);
        assert_eq!(snapshot.motor_current_amps, 5.5);
        assert_eq!(snapshot.battery_current_amps, -1.2);
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
