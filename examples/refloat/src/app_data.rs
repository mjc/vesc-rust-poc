//! Refloat app-data packet processing.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) anchors:
//! - `src/main.c:2143-2295` handles incoming app-data commands.
//! - `src/main.c:2334-2403` owns custom config get/set/XML and stop cleanup.
//! - `src/main.c:2456-2457` registers custom config and app-data handlers.
//!
//! The Rust state here is still a narrow `RefloatAppDataState`, not upstream's
//! full `Data`; upstream shares `Data *` through `ARG` for app-data, custom
//! config, BMS, threads, and stop cleanup.

use crate::domain::{
    FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID, REFLOAT_REALTIME_DATA_ITEMS,
    REFLOAT_REALTIME_RUNTIME_ITEMS, RefloatAllDataAttitude, RefloatAllDataBasePayload,
    RefloatAllDataMode3Payload, RefloatAllDataMode4Payload, RefloatAllDataMotorPayload,
    RefloatAllDataPayloads, RefloatAllDataRequest, RefloatAllDataResponse, RefloatAllDataStatus,
    RefloatAppDataCommand, RefloatChargingState, RefloatDarkRideState, RefloatFirmwareFaultCode,
    RefloatFocIdCurrent, RefloatMode, RefloatMotorCommand, RefloatRealtimeBalanceCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage, RefloatRealtimeDataItem,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
    RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};
use crate::runtime::RefloatRuntimeThreads;
use core::ffi::c_int;
use vescpkg_rs::prelude::{
    AngleDegrees, BatteryCurrent, BatteryVoltage, Current, MotorCurrent, SystemTimestamp,
    TimestampTicks, Voltage,
};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, CustomConfigBindings, ImuApi, ImuBindings,
    LoopbackLifecycle, MotorControlApi, MotorControlBindings, MotorTelemetryApi,
    MotorTelemetryBindings, ffi,
};

/// Refloat v1.2.1 generated custom-config XML blob.
///
/// Upstream generates this from `src/conf/settings.xml` via `src/Makefile:28-31`
/// and exposes `data_refloatconfig_` through `get_cfg_xml` at
/// `src/main.c:2388-2396`.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_config_xml")
)]
#[used]
static REFLOAT_CONFIG_XML: [u8; 25_723] = *include_bytes!("conf/refloatconfig.dat");

/// Refloat v1.2.1 generated serialized default custom config.
///
/// Upstream `get_cfg(..., is_default=true)` allocates `RefloatConfig`, fills
/// defaults, serializes it, then frees it at `src/main.c:2335-2356`.
/// `src/Makefile:28-31` generates the format from `src/conf/settings.xml`;
/// generated `conf/confparser.h:11-12` defines signature `2427955642` and
/// serialized length `276`, while generated `conf/confparser.c:8-178` and
/// `conf/confparser.c:363-531` serialize the default values.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_default_config")
)]
#[used]
static REFLOAT_DEFAULT_CONFIG: [u8; 276] = *include_bytes!("conf/default_config.dat");
const REFLOAT_CONFIG_SIGNATURE_BYTES: [u8; 4] = [0x90, 0xb7, 0xa9, 0xba];
// Upstream serializes `kp` as the first float16 config value after the
// signature; `src/conf/settings.xml:28-54` uses scale 10.
const REFLOAT_CONFIG_KP_OFFSET: usize = 4;
// Upstream defines `hertz` in `src/conf/settings.xml:223-246`, serializes it
// after the first seven `SerOrder` float16 entries at
// `src/conf/settings.xml:3916-3923`, and reads it as a big-endian uint16 via
// `src/conf/buffer.c:188-191`.
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_CONFIG_HERTZ_OFFSET: usize = 18;
// Upstream defines `fault_is_dual_switch` in `src/conf/settings.xml:454-467`;
// its `<ser>fault_is_dual_switch</ser>` entry at
// `src/conf/settings.xml:3935` follows the first seven float16 values,
// hertz, four fault float16 values, footbeep, five uint16 fault fields, and
// lands at byte 39 in the 276-byte generated config image.
const REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET: usize = 39;
// Upstream defines `enable_quickstop` in `src/conf/settings.xml:482-493`;
// its `<ser>enable_quickstop</ser>` entry at `src/conf/settings.xml:3937`
// lands two bools after `fault_is_dual_switch`.
const REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET: usize = 41;
// Upstream defines `disabled` in `src/conf/settings.xml:3890-3902`; its
// `<ser>disabled</ser>` entry at `src/conf/settings.xml:4064` lands at byte
// 243 in the 276-byte generated config image.
const REFLOAT_CONFIG_DISABLED_OFFSET: usize = 243;
// Upstream defines `meta.is_default` in `src/conf/settings.xml:3903-3914`; its
// `<ser>meta.is_default</ser>` entry at `src/conf/settings.xml:4083` lands at
// the final byte in the generated config image.
const REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET: usize = 275;
// Refloat v1.2.1 `cmd_info` writes this version-2 response shape at
// `src/main.c:2070-2139`.
const REFLOAT_INFO_RESPONSE_V2_LEN: usize = 60;
// Refloat v1.2.1 `cmd_realtime_data_ids` writes the counted ID-list packet at
// `src/main.c:1876-1901`.
const REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN: usize = 405;
// Refloat v1.2.1 `send_realtime_data` declares its fixed buffer at
// `src/main.c:1267-1269`.
const REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// Refloat v1.2.1 `cmd_realtime_data` declares its runtime-sized packet at
// `src/main.c:1904-1906`.
const REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY: usize = 77;
const REFLOAT_PACKAGE_NAME: &[u8] = b"Refloat";
const REFLOAT_VERSION_SUFFIX: &[u8] = b"";
const REFLOAT_GIT_HASH: u32 = 0x0ef6_e99d;
const REFLOAT_SYSTEM_TICK_RATE_HZ: u32 = 10_000;

/// Variable-length Refloat `COMMAND_REALTIME_DATA` response bytes from
/// `src/main.c:1904-1960`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRealtimeDataResponse {
    bytes: [u8; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY],
    len: usize,
}

impl RefloatRealtimeDataResponse {
    /// Return the encoded response bytes actually sent on the app-data wire.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// Fixed-size Refloat app-data response bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefloatAppDataResponse {
    /// Version/package-info response from `src/main.c:2070-2139`.
    InfoV2([u8; REFLOAT_INFO_RESPONSE_V2_LEN]),
    /// Legacy `COMMAND_GET_RTDATA` response from `src/main.c:1267-1310`.
    GetRealtimeData([u8; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN]),
    /// Realtime-data ID list response from `src/main.c:1876-1901`.
    RealtimeDataIds([u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN]),
    /// Realtime-data sample response from `src/main.c:1904-1960`.
    RealtimeData(RefloatRealtimeDataResponse),
    /// Compact all-data response from `src/main.c:1313-1399`.
    AllData(RefloatAllDataResponse),
}

impl RefloatAppDataResponse {
    /// Return the encoded response bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::InfoV2(bytes) => bytes,
            Self::GetRealtimeData(bytes) => bytes,
            Self::RealtimeDataIds(bytes) => bytes,
            Self::RealtimeData(response) => response.as_bytes(),
            Self::AllData(response) => response.as_bytes(),
        }
    }
}

/// Process one Refloat app-data packet from a typed all-data payload snapshot.
///
/// Upstream dispatches the command byte in `on_command_received` at
/// `src/main.c:2143-2301`.
pub fn process_refloat_app_data(
    payloads: RefloatAllDataPayloads,
    bytes: &[u8],
) -> Option<RefloatAppDataResponse> {
    let [package_id, command_id, payload @ ..] = bytes else {
        return None;
    };
    if *package_id != REFLOAT_APP_DATA_PACKAGE_ID.get() {
        return None;
    }
    match RefloatAppDataCommand::try_from_id(*command_id).ok()? {
        RefloatAppDataCommand::Info => Some(RefloatAppDataResponse::InfoV2(
            encode_refloat_info_response_v2(payload),
        )),
        RefloatAppDataCommand::GetRealtimeData => Some(RefloatAppDataResponse::GetRealtimeData(
            encode_refloat_get_realtime_data_response(payloads),
        )),
        RefloatAppDataCommand::RealtimeData => Some(RefloatAppDataResponse::RealtimeData(
            encode_refloat_realtime_data_response(
                payloads,
                SystemTimestamp::new(TimestampTicks::from_ticks(0)),
            ),
        )),
        RefloatAppDataCommand::RealtimeDataIds => Some(RefloatAppDataResponse::RealtimeDataIds(
            encode_refloat_realtime_data_ids_response(),
        )),
        RefloatAppDataCommand::GetAllData => Some(RefloatAppDataResponse::AllData(
            payloads.encode_response(RefloatAllDataRequest::parse(bytes).ok()?),
        )),
        _ => None,
    }
}

fn encode_refloat_info_response_v2(request_payload: &[u8]) -> [u8; REFLOAT_INFO_RESPONSE_V2_LEN] {
    // Upstream `cmd_info` responds to QML's version-2 request at
    // `src/main.c:2070-2139`; QML allocates the four-byte request and sets
    // version 2 at `ui.qml.in:693-697`.
    let flags = match request_payload {
        [2, flags, ..] => *flags,
        _ => 0,
    };
    let mut bytes = [0; REFLOAT_INFO_RESPONSE_V2_LEN];
    let mut index = 0;
    bytes[index] = REFLOAT_APP_DATA_PACKAGE_ID.get();
    index += 1;
    bytes[index] = RefloatAppDataCommand::Info.id();
    index += 1;
    bytes[index] = 2;
    index += 1;
    bytes[index] = flags;
    index += 1;
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_PACKAGE_NAME);
    bytes[index] = 1;
    index += 1;
    bytes[index] = 2;
    index += 1;
    bytes[index] = 1;
    index += 1;
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_VERSION_SUFFIX);
    bytes[index..index + 4].copy_from_slice(&REFLOAT_GIT_HASH.to_be_bytes());
    index += 4;
    bytes[index..index + 4].copy_from_slice(&REFLOAT_SYSTEM_TICK_RATE_HZ.to_be_bytes());
    index += 4;
    // Upstream derives capabilities from data-recorder and LED config at
    // `src/main.c:2121-2132`; this Rust runtime has not ported either
    // capability yet, so the honest advertised capability mask is zero.
    bytes[index..index + 4].copy_from_slice(&0u32.to_be_bytes());
    index += 4;
    // Upstream currently sends zero `extra_flags` at `src/main.c:2134-2135`.
    bytes[index] = 0;
    bytes
}

fn append_fixed_ascii<const LEN: usize>(bytes: &mut [u8], index: &mut usize, value: &[u8]) {
    let len = value.len().min(LEN);
    bytes[*index..*index + len].copy_from_slice(&value[..len]);
    *index += LEN;
}

fn encode_refloat_realtime_data_ids_response() -> [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN] {
    let mut bytes = [0; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN];
    let mut index = 0;
    bytes[index] = REFLOAT_APP_DATA_PACKAGE_ID.get();
    index += 1;
    bytes[index] = RefloatAppDataCommand::RealtimeDataIds.id();
    index += 1;
    // Upstream sends two counted string-ID sets from `cmd_realtime_data_ids`
    // at `src/main.c:1876-1901`; QML consumes them at `ui.qml.in:927-934`.
    append_realtime_item_ids(&mut bytes, &mut index, &REFLOAT_REALTIME_DATA_ITEMS);
    append_realtime_item_ids(&mut bytes, &mut index, &REFLOAT_REALTIME_RUNTIME_ITEMS);
    bytes
}

fn append_realtime_item_ids<const N: usize>(
    bytes: &mut [u8],
    index: &mut usize,
    items: &[RefloatRealtimeDataItem; N],
) {
    bytes[*index] = N as u8;
    *index += 1;
    for id in items.iter().map(|item| item.id().as_bytes()) {
        bytes[*index] = id.len() as u8;
        *index += 1;
        bytes[*index..*index + id.len()].copy_from_slice(id);
        *index += id.len();
    }
}

fn encode_refloat_get_realtime_data_response(
    payloads: RefloatAllDataPayloads,
) -> [u8; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN] {
    let mut bytes = [0; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let footpad = base.footpad();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let motor = base.motor();

    // Upstream `on_command_received` dispatches `COMMAND_GET_RTDATA` to
    // `send_realtime_data` at `src/main.c:2162-2164`; `send_realtime_data`
    // writes this legacy 72-byte payload at `src/main.c:1267-1310`.
    refloat_realtime_push_u8(&mut bytes, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        RefloatAppDataCommand::GetRealtimeData.id(),
    );

    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        base.balance_current().current().current().as_amps(),
    );
    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        attitude.balance_pitch().angle().as_radians(),
    );
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, attitude.roll().angle().as_radians());

    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (ride_state.float_state_compat() & 0x0f) + (ride_state.setpoint_adjustment_compat() << 4),
    );
    let switch_state = footpad.state().switch_compat()
        | u8::from(matches!(ride_state.mode(), RefloatMode::HandTest)) << 3;
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        (switch_state & 0x0f) + (base.status().beep_reason().id() << 4),
    );
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc1().ratio().as_ratio());
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc2().ratio().as_ratio());

    [
        setpoints.board(),
        setpoints.atr(),
        setpoints.brake_tilt(),
        setpoints.torque_tilt(),
        setpoints.turn_tilt(),
        setpoints.remote(),
    ]
    .into_iter()
    .map(|setpoint| setpoint.angle().as_degrees())
    .for_each(|value| refloat_realtime_push_float32_auto(&mut bytes, &mut ind, value));

    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, attitude.pitch().angle().as_radians());
    // Upstream reads `d->motor.filt_current`, `d->atr.accel_diff`, and
    // `d->motor.dir_current` at `src/main.c:1298-1306`. The current Rust
    // app-data state does not yet contain those separate runtime fields, so
    // this is explicitly a containment fallback until the shared `Data`
    // runtime is ported from `src/main.c:2419-2461`.
    refloat_realtime_push_float32_auto(
        &mut bytes,
        &mut ind,
        motor.motor_current().current().as_amps(),
    );
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, 0.0);
    if matches!(ride_state.charging(), RefloatChargingState::Charging) {
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    } else {
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            base.booster_current().current().current().as_amps(),
        );
        refloat_realtime_push_float32_auto(
            &mut bytes,
            &mut ind,
            motor.motor_current().current().as_amps(),
        );
    }
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, 0.0);

    bytes
}

fn encode_refloat_realtime_data_response(
    payloads: RefloatAllDataPayloads,
    system_timestamp: SystemTimestamp,
) -> RefloatRealtimeDataResponse {
    let mut bytes = [0; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = matches!(ride_state.run_state(), RefloatRunState::Running);
    let charging = matches!(ride_state.charging(), RefloatChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
    refloat_realtime_push_u8(&mut bytes, &mut ind, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        RefloatAppDataCommand::RealtimeData.id(),
    );

    let mut mask = 0x04;
    if running {
        mask |= 0x01;
    }
    if charging {
        mask |= 0x02;
    }
    refloat_realtime_push_u8(&mut bytes, &mut ind, mask);

    // The data recorder and alert tracker are still part of the unported
    // control-loop/runtime state (`src/main.c:1927-1930`, `src/main.c:1956-1958`).
    refloat_realtime_push_u8(&mut bytes, &mut ind, 0);
    // Upstream writes `d->time.now` at `src/main.c:1931`; VESC timestamps are
    // represented as 100 us system ticks.
    refloat_realtime_push_u32(&mut bytes, &mut ind, system_timestamp.ticks().as_ticks());

    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        ride_state.mode().id() << 4 | ride_state.run_state().id(),
    );
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        base.footpad().state().id() << 6
            | u8::from(matches!(
                ride_state.charging(),
                RefloatChargingState::Charging
            )) << 5
            | u8::from(matches!(
                ride_state.darkride(),
                RefloatDarkRideState::Active
            )) << 1
            | u8::from(matches!(
                ride_state.wheelslip(),
                RefloatWheelSlipState::Detected
            )),
    );
    refloat_realtime_push_u8(
        &mut bytes,
        &mut ind,
        ride_state.setpoint_adjustment().id() << 4 | ride_state.stop_condition().id(),
    );
    refloat_realtime_push_u8(&mut bytes, &mut ind, base.status().beep_reason().id());

    REFLOAT_REALTIME_DATA_ITEMS.into_iter().for_each(|item| {
        refloat_realtime_push_float16_auto(&mut bytes, &mut ind, realtime_value(payloads, item))
    });
    if running {
        REFLOAT_REALTIME_RUNTIME_ITEMS.into_iter().for_each(|item| {
            refloat_realtime_push_float16_auto(
                &mut bytes,
                &mut ind,
                realtime_value(payloads, item),
            );
        });
    }
    if charging {
        refloat_realtime_push_float16_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().current().current().current().as_amps(),
        );
        refloat_realtime_push_float16_auto(
            &mut bytes,
            &mut ind,
            payloads.mode4().voltage().voltage().voltage().as_volts(),
        );
    }

    refloat_realtime_push_u32(&mut bytes, &mut ind, 0);
    refloat_realtime_push_u32(&mut bytes, &mut ind, 0);
    refloat_realtime_push_u8(&mut bytes, &mut ind, 0);

    RefloatRealtimeDataResponse { bytes, len: ind }
}

fn realtime_value(payloads: RefloatAllDataPayloads, item: RefloatRealtimeDataItem) -> f32 {
    let base = payloads.base();
    let motor = base.motor();
    let attitude = base.attitude();
    let setpoints = base.setpoints();
    let temperatures = payloads.mode2().temperatures();

    match item {
        RefloatRealtimeDataItem::MotorSpeed => motor.vehicle_speed().speed().as_meters_per_second(),
        RefloatRealtimeDataItem::MotorErpm => {
            motor.electrical_speed().rpm().as_revolutions_per_minute()
        }
        RefloatRealtimeDataItem::MotorCurrent => motor.motor_current().current().as_amps(),
        RefloatRealtimeDataItem::MotorDirectionalCurrent => {
            motor.motor_current().current().as_amps()
        }
        RefloatRealtimeDataItem::MotorFilteredCurrent => motor.motor_current().current().as_amps(),
        RefloatRealtimeDataItem::MotorDutyCycle => motor.duty_cycle().ratio().as_ratio(),
        RefloatRealtimeDataItem::MotorBatteryVoltage => {
            motor.battery_voltage().voltage().as_volts()
        }
        RefloatRealtimeDataItem::MotorBatteryCurrent => motor.battery_current().current().as_amps(),
        RefloatRealtimeDataItem::MotorMosfetTemperature => {
            temperatures.mosfet().temperature().as_degrees_celsius()
        }
        RefloatRealtimeDataItem::MotorTemperature => {
            temperatures.motor().temperature().as_degrees_celsius()
        }
        RefloatRealtimeDataItem::ImuPitch => {
            refloat_radians_to_degrees(attitude.pitch().angle().as_radians())
        }
        RefloatRealtimeDataItem::ImuBalancePitch => {
            refloat_radians_to_degrees(attitude.balance_pitch().angle().as_radians())
        }
        RefloatRealtimeDataItem::ImuRoll => {
            refloat_radians_to_degrees(attitude.roll().angle().as_radians())
        }
        RefloatRealtimeDataItem::FootpadAdc1 => base.footpad().adc1().ratio().as_ratio(),
        RefloatRealtimeDataItem::FootpadAdc2 => base.footpad().adc2().ratio().as_ratio(),
        RefloatRealtimeDataItem::RemoteInput => 0.0,
        RefloatRealtimeDataItem::Setpoint => setpoints.board().angle().as_degrees(),
        RefloatRealtimeDataItem::AtrSetpoint => setpoints.atr().angle().as_degrees(),
        RefloatRealtimeDataItem::BrakeTiltSetpoint => setpoints.brake_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::TorqueTiltSetpoint => setpoints.torque_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::TurnTiltSetpoint => setpoints.turn_tilt().angle().as_degrees(),
        RefloatRealtimeDataItem::RemoteSetpoint => setpoints.remote().angle().as_degrees(),
        RefloatRealtimeDataItem::BalanceCurrent => {
            base.balance_current().current().current().as_amps()
        }
        RefloatRealtimeDataItem::AtrAccelDiff => 0.0,
        RefloatRealtimeDataItem::AtrSpeedBoost => 0.0,
        RefloatRealtimeDataItem::BoosterCurrent => {
            base.booster_current().current().current().as_amps()
        }
    }
}

fn refloat_radians_to_degrees(radians: f32) -> f32 {
    radians * 57.295_78
}

fn refloat_realtime_push_float16_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float16_auto` at
    // `src/conf/buffer.c:143-145`, which writes `to_float16` big-endian.
    refloat_realtime_push_u16(buffer, ind, refloat_float16_auto_bits(value));
}

fn refloat_realtime_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float32_auto` at
    // `src/conf/buffer.c:118-140`, zeroing denormal/subnormal values before
    // writing big-endian IEEE-754 bits.
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    refloat_realtime_push_u32(buffer, ind, value.to_bits());
}

fn refloat_float16_auto_bits(value: f32) -> u16 {
    // Refloat's `to_float16` is defined at `src/conf/buffer.c:33-43`.
    let b = value.to_bits().wrapping_add(0x0000_1000);
    let e = (b & 0x7f80_0000) >> 23;
    let m = b & 0x007f_ffff;
    let normalized = if e > 112 {
        (((e - 112) << 10) & 0x7c00) | (m >> 13)
    } else {
        0
    };
    let denormalized = if e < 113 && e > 101 {
        (((0x007f_f000 + m) >> (125 - e)) + 1) >> 1
    } else {
        0
    };
    let saturated = if e > 143 { 0x7fff } else { 0 };
    (((b & 0x8000_0000) >> 16) | normalized | denormalized | saturated) as u16
}

fn refloat_realtime_push_u32(buffer: &mut [u8], ind: &mut usize, value: u32) {
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| refloat_realtime_push_u8(buffer, ind, byte));
}

fn refloat_realtime_push_u16(buffer: &mut [u8], ind: &mut usize, value: u16) {
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| refloat_realtime_push_u8(buffer, ind, byte));
}

fn refloat_realtime_push_u8(buffer: &mut [u8], ind: &mut usize, value: u8) {
    buffer[*ind] = value;
    *ind += 1;
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn handle_refloat_app_data_packet<
    B: AppDataBindings,
    M: MotorTelemetryBindings,
    I: ImuBindings,
>(
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
    data: *mut u8,
    len: u32,
) -> bool {
    let Some(data) = core::ptr::NonNull::new(data) else {
        return false;
    };
    let Ok(len) = usize::try_from(len) else {
        return false;
    };
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr().cast_const(), len) };
    state.handle_packet_with_runtime(lifecycle, telemetry, imu, bytes)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn prog_addr() -> u32 {
    let address: u32;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {prog_ptr}",
            address = out(reg) address,
            prog_ptr = sym crate::init::prog_ptr,
            options(nomem, nostack, preserves_flags),
        );
    }
    address
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_app_data_handler() -> ffi::AppDataHandler {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {handler}",
            address = out(reg) address,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
        core::mem::transmute::<usize, ffi::AppDataHandler>(address | 1)
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn refloat_state_from_arg() -> Option<&'static mut RefloatAppDataState> {
    let arg_slot = unsafe { ffi::raw::vesc_get_arg(prog_addr()) };
    if arg_slot.is_null() {
        return None;
    }
    let arg_slot = unsafe { arg_slot.as_mut()? };
    let state = (*arg_slot).cast::<RefloatAppDataState>();
    if state.is_null() {
        return None;
    }
    unsafe { state.as_mut() }
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// Upstream registers `on_command_received` in `src/main.c:2457`; the handler
/// dispatches command IDs in `src/main.c:2143-2295`.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn refloat_handle_app_data(data: *mut u8, len: u32) {
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
    let imu = ImuApi::new(vescpkg_rs::RealImuBindings);
    let _ =
        unsafe { handle_refloat_app_data_packet(state, &lifecycle, &telemetry, &imu, data, len) };
}

/// Install source-startup Refloat state without registering callbacks.
///
/// Upstream allocates `Data`, runs `data_init`, and stores `stop`/`Data *` in
/// loader metadata at `src/main.c:2419-2432`; callback/LispBM registration
/// follows at `src/main.c:2455-2459`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `state` must
/// remain valid until firmware stops the package.
#[cfg(test)]
pub(crate) unsafe fn install_refloat_startup_state_with<B: AppDataBindings>(
    info: *mut ffi::LibInfo,
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    *state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());
    unsafe { lifecycle.install_refloat_state(info, state, handler) }
}

/// Install source-startup Refloat state and callback registrations.
///
/// Upstream stores loader metadata at `src/main.c:2431-2432` before registering
/// custom config/app-data callbacks at `src/main.c:2456-2457`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `state` and
/// `handler` must remain valid until firmware clears/replaces the handler and
/// stops the package.
#[cfg(test)]
pub(crate) unsafe fn install_refloat_startup_app_data_with<
    B: AppDataBindings + CustomConfigBindings,
>(
    info: *mut ffi::LibInfo,
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    if !unsafe { install_refloat_startup_state_with(info, state, lifecycle, handler) } {
        return false;
    }
    unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_ok()
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn clear_refloat_app_data_loader_info(info: *mut ffi::LibInfo) {
    if let Some(info) = unsafe { info.as_mut() } {
        info.arg = core::ptr::null_mut();
        info.stop_fun = None;
    }
}

/// Allocate and install source-startup Refloat state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `src/main.c:2419`, runs
/// `data_init` at `src/main.c:2424`, and stores the same pointer in
/// `info->arg` at `src/main.c:2432`. This Rust path still allocates a narrow
/// `RefloatAppDataState`, but keeps the same loader metadata order before the
/// registration tail at `src/main.c:2455-2459`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `handler` must
/// remain valid until firmware stops the package.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) unsafe fn allocate_refloat_startup_state_with<
    A: vescpkg_rs::AllocBindings,
    B: AppDataBindings,
>(
    info: *mut ffi::LibInfo,
    allocator: &vescpkg_rs::FirmwareAllocator<'_, A>,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    let Ok(mut allocation) = allocator.allocate_for::<RefloatAppDataState>(1) else {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    };
    let state = allocation.as_mut_ptr();
    unsafe {
        state.write(RefloatAppDataState::new(
            RefloatAllDataPayloads::source_startup(),
        ));
    }
    let state = unsafe { &mut *state };

    if !unsafe { lifecycle.install_refloat_state(info, state, handler) } {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    }

    let _ = allocation.into_raw();
    true
}

/// Allocate source-startup Refloat state and register app-data callbacks.
///
/// Upstream performs state setup at `src/main.c:2419-2432`, starts runtime
/// threads at `src/main.c:2439-2449`, then registers custom config/app-data
/// callbacks at `src/main.c:2456-2457` after IMU setup. This compatibility
/// helper only keeps state-before-callback order for tests.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `handler` must
/// remain valid until firmware clears/replaces the handler and stops the package.
#[cfg(test)]
pub(crate) unsafe fn allocate_refloat_startup_app_data_with<
    A: vescpkg_rs::AllocBindings,
    B: AppDataBindings + CustomConfigBindings,
>(
    info: *mut ffi::LibInfo,
    allocator: &vescpkg_rs::FirmwareAllocator<'_, A>,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    if !unsafe { allocate_refloat_startup_state_with(info, allocator, lifecycle, handler) } {
        return false;
    }

    if unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_err() {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    }

    true
}

/// Allocate and install Refloat startup state using firmware memory.
///
/// This matches the loader metadata step from upstream `src/main.c:2419-2432`;
/// callback/LispBM registration is a separate step at `src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data_state(info: *mut ffi::LibInfo) -> bool {
    let alloc_bindings = vescpkg_rs::RealBindings;
    let allocator = vescpkg_rs::FirmwareAllocator::new(&alloc_bindings);
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let handler = runtime_refloat_app_data_handler();
    unsafe { allocate_refloat_startup_state_with(info, &allocator, &lifecycle, handler) }
}

/// Register Refloat custom config and app-data callbacks.
///
/// Upstream registers these callbacks at `src/main.c:2456-2457`, after runtime
/// thread startup at `src/main.c:2439-2449` and IMU setup at
/// `src/main.c:2455`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_app_data_callbacks(info: *mut ffi::LibInfo) -> bool {
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let handler = runtime_refloat_app_data_handler();
    unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_ok()
}

/// Allocate startup state and register Refloat app-data callbacks.
///
/// Kept as the old combined entrypoint for callers that do not need the
/// upstream split between `src/main.c:2431-2432` and `src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data(info: *mut ffi::LibInfo) -> bool {
    install_refloat_app_data_state(info) && register_refloat_app_data_callbacks(info)
}

/// Register Refloat custom-config callbacks with VESC Tool.
///
/// Upstream registers `get_cfg`, `set_cfg`, and `get_cfg_xml` at
/// `src/main.c:2456`; those callbacks are implemented at `src/main.c:2334-2396`.
/// The Rust port does not yet generate or serialize upstream `RefloatConfig`, so
/// these callbacks report no config payload instead of pretending to be the full
/// confparser path.
pub fn register_refloat_custom_config<B: CustomConfigBindings>(bindings: &B) -> bool {
    unsafe {
        bindings.register_custom_config(refloat_get_cfg, refloat_set_cfg, refloat_get_cfg_xml)
    }
}

unsafe extern "C" fn refloat_get_cfg(buffer: *mut u8, is_default: bool) -> c_int {
    let state = unsafe { runtime_refloat_config_state() };
    refloat_get_cfg_with_state(buffer, is_default, state)
}

fn refloat_get_cfg_with_state(
    buffer: *mut u8,
    is_default: bool,
    state: Option<&RefloatAppDataState>,
) -> c_int {
    if !is_default {
        // Upstream serializes `d->float_conf` at `src/main.c:2347-2350`;
        // `data_init` first populates it from EEPROM or generated defaults at
        // `src/main.c:1160-1185`. The Rust state stores the serialized image
        // until the typed `RefloatConfig` parser/deserializer is ported.
        let Some(state) = state else {
            return 0;
        };
        return copy_refloat_config(buffer, state.serialized_config());
    }

    // Upstream default path is `src/main.c:2339-2350`: allocate config, call
    // `confparser_set_defaults_refloatconfig`, then
    // `confparser_serialize_refloatconfig`.
    copy_refloat_config(buffer, &REFLOAT_DEFAULT_CONFIG)
}

fn copy_refloat_config(buffer: *mut u8, config: &[u8; 276]) -> c_int {
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return 0;
    };

    unsafe { core::ptr::copy_nonoverlapping(config.as_ptr(), buffer.as_ptr(), config.len()) };
    config.len() as c_int
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn runtime_refloat_config_state() -> Option<&'static RefloatAppDataState> {
    let state = unsafe { refloat_state_from_arg()? };
    Some(&*state)
}

#[cfg(any(test, not(target_arch = "arm")))]
unsafe fn runtime_refloat_config_state() -> Option<&'static RefloatAppDataState> {
    None
}

unsafe extern "C" fn refloat_set_cfg(buffer: *mut u8) -> bool {
    let state = unsafe { runtime_refloat_config_state_mut() };
    refloat_set_cfg_with_state(buffer, state)
}

fn refloat_set_cfg_with_state(buffer: *mut u8, state: Option<&mut RefloatAppDataState>) -> bool {
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return false;
    };
    let Some(state) = state else {
        return false;
    };
    let config = unsafe {
        core::slice::from_raw_parts(buffer.as_ptr().cast_const(), REFLOAT_DEFAULT_CONFIG.len())
    };
    // Upstream `set_cfg` gates special modes, deserializes, persists, and
    // reconfigures at `src/main.c:2360-2386`; generated
    // `conf/confparser.c:187-190` rejects bad signatures before field reads.
    // This byte-image step is intentionally only the deserialization/storage
    // part; EEPROM write and `configure(d)` remain separate parity work.
    state.store_serialized_config(config)
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatAppDataState> {
    unsafe { refloat_state_from_arg() }
}

#[cfg(any(test, not(target_arch = "arm")))]
unsafe fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatAppDataState> {
    None
}

unsafe extern "C" fn refloat_get_cfg_xml(buffer: *mut *mut u8) -> c_int {
    let xml = runtime_refloat_config_xml();
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        *buffer = xml.cast_mut();
    }
    // Upstream returns `data_refloatconfig_ + PROG_ADDR` and
    // `DATA_REFLOATCONFIG__SIZE` at `src/main.c:2388-2396`.
    REFLOAT_CONFIG_XML.len() as c_int
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_xml() -> *const u8 {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {xml}",
            address = out(reg) address,
            xml = sym REFLOAT_CONFIG_XML,
            options(nomem, nostack, preserves_flags),
        );
    }
    address as *const u8
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_xml() -> *const u8 {
    REFLOAT_CONFIG_XML.as_ptr()
}

/// Refloat motor-control request state.
///
/// Upstream `MotorControl` stores `current_requested` and `requested_current`
/// at `src/motor_control.h:27-30`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatMotorControl {
    disabled: bool,
    requested_current: Option<RefloatMotorCommand>,
}

impl RefloatMotorControl {
    const fn new() -> Self {
        Self {
            disabled: false,
            requested_current: None,
        }
    }

    fn request_current(&mut self, current: MotorCurrent) {
        // Upstream `motor_control_request_current` sets the request flag and
        // stores the requested current at `src/motor_control.c:44-47`.
        self.requested_current = Some(RefloatMotorCommand::new(current));
    }

    fn apply_requested_current<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
    ) -> bool {
        let Some(command) = self.requested_current else {
            return false;
        };

        motor.timeout_reset();
        motor.set_current_off_delay(0.05);
        motor.set_current(command.requested_current());
        self.requested_current = None;
        true
    }

    fn apply<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
        run_state: RefloatRunState,
    ) -> bool {
        if matches!(run_state, RefloatRunState::Disabled) {
            if !self.disabled {
                motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
                self.disabled = true;
                return true;
            }
            return false;
        }

        self.disabled = false;
        self.apply_requested_current(motor)
    }
}

/// Refloat package app-data state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAppDataState {
    all_data_payloads: RefloatAllDataPayloads,
    serialized_config: [u8; 276],
    runtime_threads: RefloatRuntimeThreads,
    motor_control: RefloatMotorControl,
}

impl RefloatAppDataState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        Self {
            all_data_payloads,
            // Upstream `data_init` reads EEPROM and falls back to generated
            // defaults at `src/main.c:1160-1185`; full EEPROM parity remains a
            // later source-backed slice.
            serialized_config: REFLOAT_DEFAULT_CONFIG,
            // Upstream stores these in `Data` after spawning at
            // `src/main.c:2439-2445`; this Rust state only tracks the handles
            // until the full `Data` layout is ported.
            runtime_threads: RefloatRuntimeThreads::empty(),
            motor_control: RefloatMotorControl::new(),
        }
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    /// Return the runtime thread handles currently owned by this package state.
    pub const fn runtime_threads(self) -> RefloatRuntimeThreads {
        self.runtime_threads
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    /// Apply and clear a pending motor-current request.
    pub fn apply_requested_motor_current<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
    ) -> bool {
        self.motor_control.apply_requested_current(motor)
    }

    /// Apply motor control for the current run state.
    pub fn apply_motor_control<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
        run_state: RefloatRunState,
    ) -> bool {
        self.motor_control.apply(motor, run_state)
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn set_runtime_threads(&mut self, runtime_threads: RefloatRuntimeThreads) {
        self.runtime_threads = runtime_threads;
    }

    fn serialized_config(&self) -> &[u8; 276] {
        &self.serialized_config
    }

    fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Ok(config) = <&[u8; 276]>::try_from(config) else {
            return false;
        };
        if config[..4] != REFLOAT_CONFIG_SIGNATURE_BYTES {
            return false;
        }

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        let mut config = *config;
        // Upstream clears `d->float_conf.disabled` while running at
        // `src/main.c:2369-2372`; `disabled` is serialized from
        // `src/conf/settings.xml:3890-3902` at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            config[REFLOAT_CONFIG_DISABLED_OFFSET] = 0;
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `src/main.c:2375-2377`; `meta.is_default` is serialized from
        // `src/conf/settings.xml:3903-3914` at byte 275.
        config[REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 0;
        self.serialized_config = config;
        true
    }

    fn refresh_config_runtime_state(&mut self) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let disabled = self.serialized_config[REFLOAT_CONFIG_DISABLED_OFFSET] != 0;
        let run_state = match (ride_state.run_state(), disabled) {
            // Refloat applies `float_conf.disabled` from `configure(d)` at
            // `src/main.c:184-190`; `state_set_disabled` keeps RUNNING alive
            // and toggles DISABLED/STARTUP at `src/state.c:41-47`.
            (RefloatRunState::Running, true) => RefloatRunState::Running,
            (RefloatRunState::Disabled, false) => RefloatRunState::Startup,
            (_, true) => RefloatRunState::Disabled,
            (run_state, false) => run_state,
        };
        if run_state == ride_state.run_state() {
            return;
        }

        let ride_state = RefloatRideState::new(
            run_state,
            ride_state.mode(),
            ride_state.setpoint_adjustment(),
            ride_state.stop_condition(),
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(ride_state.wheelslip())
        .with_darkride(ride_state.darkride());
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        let hertz = u16::from_be_bytes([
            self.serialized_config[REFLOAT_CONFIG_HERTZ_OFFSET],
            self.serialized_config[REFLOAT_CONFIG_HERTZ_OFFSET + 1],
        ]);
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `src/main.c:190-191`, then `refloat_thd` sleeps that value at
        // `src/main.c:1080`.
        1_000_000 / u32::from(hertz)
    }

    /// Recover typed app-data state from VESC loader metadata.
    ///
    /// # Safety
    ///
    /// `info.arg` must either be null or contain a valid pointer to a live
    /// `RefloatAppDataState`.
    pub unsafe fn from_info_arg(info: &mut ffi::LibInfo) -> Option<&mut Self> {
        let ptr = core::ptr::NonNull::new(info.arg.cast::<Self>())?;
        Some(unsafe { ptr.as_ptr().as_mut()? })
    }

    /// Handle one app-data packet through the supplied lifecycle transport.
    pub fn handle_packet<B: AppDataBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }
        lifecycle.send_response(self.all_data_payloads, bytes)
    }

    /// Handle one app-data packet after refreshing the source-backed runtime slices.
    ///
    /// Upstream refreshes IMU angles in `src/imu.c:35-40` and gates
    /// `STATE_STARTUP` -> `STATE_READY` on `imu_startup_done()` in
    /// `src/main.c:833-838`. This does not model the full control loop.
    pub fn handle_packet_with_runtime<
        B: AppDataBindings,
        M: MotorTelemetryBindings,
        I: ImuBindings,
    >(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        bytes: &[u8],
    ) -> bool {
        self.refresh_runtime_state(telemetry, imu);
        self.handle_packet_with_telemetry(lifecycle, telemetry, bytes)
    }

    /// Refresh the source-backed runtime slices that Refloat updates near the
    /// top of `refloat_thd`.
    ///
    /// Upstream applies `configure(d)` before runtime work at
    /// `src/main.c:184-191`, updates IMU at `src/main.c:775`, motor data at
    /// `src/main.c:796`, and performs the `STATE_STARTUP` -> `STATE_READY`
    /// gate at `src/main.c:833-838`.
    pub(crate) fn refresh_runtime_state<M: MotorTelemetryBindings, I: ImuBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_imu_runtime_state(imu);
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    pub fn handle_packet_with_telemetry<B: AppDataBindings, M: MotorTelemetryBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }

        if matches!(
            bytes,
            [
                package_id,
                command_id,
                ..
            ] if *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get()
                && matches!(
                    RefloatAppDataCommand::try_from_id(*command_id),
                    Ok(RefloatAppDataCommand::Info | RefloatAppDataCommand::RealtimeDataIds)
                )
        ) {
            return lifecycle.send_response(self.all_data_payloads, bytes);
        }

        if matches!(
            bytes,
            [package_id, command_id, ..]
                if *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get()
                    && matches!(
                        RefloatAppDataCommand::try_from_id(*command_id),
                        Ok(RefloatAppDataCommand::RealtimeData)
                    )
        ) {
            let payloads = self
                .all_data_payloads
                .with_base_battery_voltage(BatteryVoltage::new(
                    telemetry.input_voltage_filtered().voltage(),
                ))
                .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                    telemetry.mosfet_temperature(),
                    telemetry.motor_temperature(),
                ));
            // Refloat's main loop updates `d->time.now` before app-data reads it
            // in `cmd_realtime_data` at `src/main.c:1931`.
            let system_timestamp = SystemTimestamp::new(TimestampTicks::from_ticks(
                lifecycle.bindings().system_time_ticks(),
            ));
            let response = encode_refloat_realtime_data_response(payloads, system_timestamp);
            return lifecycle.send_response_bytes(response.as_bytes());
        }

        let Ok(request) = RefloatAllDataRequest::parse(bytes) else {
            return false;
        };
        let fault = telemetry.firmware_fault();
        if !fault.is_none() {
            let Some(fault_code) = fault.compat_code() else {
                return false;
            };
            let response = RefloatAllDataResponse::fault(
                RefloatFirmwareFaultCode::from_compat_code(fault_code),
            );
            return lifecycle.send_response_bytes(response.as_bytes());
        }
        let mode = request.mode();
        let payloads = self
            .all_data_payloads
            .with_base_battery_voltage(BatteryVoltage::new(
                telemetry.input_voltage_filtered().voltage(),
            ));
        let payloads = if mode.includes_mode2() {
            self.runtime_all_data_payloads(payloads, telemetry, mode.includes_mode3())
        } else {
            payloads
        };
        lifecycle.send_all_data_response(payloads, request)
    }

    fn refresh_motor_runtime_state<M: MotorTelemetryBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
    ) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let motor = base.motor();
        // Refloat v1.2.1 updates motor fields in `motor_data_update` at
        // `src/motor_data.c:108-145`. Battery current uses the same first-order
        // smoothing expression from `src/motor_data.c:140`; this app-data
        // refresh is still a runtime proxy until the real source main loop runs.
        let previous_battery_current = motor.battery_current().current().as_amps();
        let next_battery_current = telemetry.battery_current().current().as_amps();
        let motor = RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(telemetry.input_voltage_filtered().voltage()),
            telemetry.electrical_speed(),
            telemetry.vehicle_speed(),
            telemetry.motor_current(),
            BatteryCurrent::new(Current::from_amps(
                previous_battery_current + 0.01 * (next_battery_current - previous_battery_current),
            )),
            telemetry.duty_cycle_now(),
            // Upstream compact all-data reads optional `VESC_IF->foc_get_id` at
            // `src/main.c:1364-1368` and writes 222 when the slot is absent.
            telemetry.foc_id_current().map_or(
                RefloatFocIdCurrent::unavailable(),
                RefloatFocIdCurrent::measured,
            ),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            motor,
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn refresh_imu_runtime_state<I: ImuBindings>(&mut self, imu: &ImuApi<I>) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let resets_runtime_vars =
            matches!(ride_state.run_state(), RefloatRunState::Startup) && imu.startup_done();
        let run_state = match (ride_state.run_state(), imu.startup_done()) {
            (RefloatRunState::Startup, true) => RefloatRunState::Ready,
            (run_state, _) => run_state,
        };
        let flywheel_both_footpads_fault = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Running,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let reverse_stop_no_footpads_fault = matches!(
            (
                run_state,
                ride_state.setpoint_adjustment(),
                base.footpad().state()
            ),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop,
                FootpadSensorState::None
            )
        );
        let reverse_stop_pitch_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && imu.pitch().angle().as_radians().abs()
            > 18.0_f32.to_radians();
        let motor_erpm = base
            .motor()
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute();
        let pitch = imu.pitch().angle().as_radians();
        let quickstop_fault = matches!(
            (run_state, base.footpad().state(), ride_state.mode()),
            (
                RefloatRunState::Running,
                FootpadSensorState::None,
                mode
            ) if !matches!(mode, RefloatMode::Flywheel)
        ) && self.serialized_config[REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET]
            != 0
            && motor_erpm.abs() < 200.0
            && pitch.abs() > 14.0_f32.to_radians()
            && base.setpoints().remote().angle().as_degrees().abs() < 30.0
            && (pitch >= 0.0) == (motor_erpm >= 0.0);
        let single_footpad = matches!(
            base.footpad().state(),
            FootpadSensorState::Left | FootpadSensorState::Right
        );
        let dual_switch = self.serialized_config[REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET] != 0;
        let can_engage = matches!(ride_state.charging(), RefloatChargingState::NotCharging)
            && (matches!(base.footpad().state(), FootpadSensorState::Both)
                || single_footpad && dual_switch
                || matches!(ride_state.mode(), RefloatMode::Flywheel));
        let ready_flywheel_stop = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Ready,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let darkride_can_engage_fault = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Active)
        ) && can_engage;
        let state_stop_fault = flywheel_both_footpads_fault
            || reverse_stop_no_footpads_fault
            || reverse_stop_pitch_fault
            || quickstop_fault
            || darkride_can_engage_fault;
        let ready_engage = matches!(run_state, RefloatRunState::Ready)
            && !ready_flywheel_stop
            && can_engage
            && base.attitude().balance_pitch().angle().as_radians().abs()
                < core::f32::consts::FRAC_PI_4
            && imu.roll().angle().as_radians().abs() < core::f32::consts::FRAC_PI_4;
        let stop_condition = if flywheel_both_footpads_fault {
            // Upstream `check_faults(d)` stops RUNNING FLYWHEEL when both
            // footpads are engaged at `src/main.c:491-493`; `state_stop`
            // moves to READY and stores STOP_SWITCH_HALF at `src/state.c:29-33`.
            RefloatStopCondition::SwitchHalf
        } else if reverse_stop_no_footpads_fault {
            // Upstream `check_faults(d)` immediately stops reverse-stop mode
            // when the footpad is fully open at `src/main.c:418-422`.
            RefloatStopCondition::SwitchFull
        } else if reverse_stop_pitch_fault {
            // Upstream `check_faults(d)` immediately stops reverse-stop mode
            // when `fabsf(d->imu.pitch) > 18` at `src/main.c:423-426`.
            RefloatStopCondition::ReverseStop
        } else if quickstop_fault {
            // Upstream `check_faults(d)` quick-stops no-footpad low-speed
            // pitch-runaway cases at `src/main.c:419-423`.
            RefloatStopCondition::QuickStop
        } else if darkride_can_engage_fault {
            // Upstream darkride `check_faults(d)` allows turning it off by
            // engaging foot sensors at `src/main.c:387-390`.
            RefloatStopCondition::SwitchHalf
        } else if ready_engage {
            RefloatStopCondition::None
        } else {
            ride_state.stop_condition()
        };
        let run_state = if state_stop_fault {
            RefloatRunState::Ready
        } else if ready_engage {
            // Upstream READY engages when startup pitch/roll tolerances and
            // `can_engage(d)` pass at `src/main.c:1033-1036`; `state_engage`
            // moves to RUNNING and clears the stop condition at `src/state.c:36-39`.
            RefloatRunState::Running
        } else {
            run_state
        };
        let setpoint_adjustment = if ready_engage {
            RefloatSetpointAdjustment::Centering
        } else {
            ride_state.setpoint_adjustment()
        };
        let wheelslip = if state_stop_fault {
            RefloatWheelSlipState::None
        } else {
            ride_state.wheelslip()
        };
        let ride_state = RefloatRideState::new(
            run_state,
            if ready_flywheel_stop {
                // Upstream READY stops FLYWHEEL on abort/both-footpad before
                // start conditions at `src/main.c:957-963`; `flywheel_stop`
                // returns to NORMAL mode at `src/main.c:1869-1873`.
                RefloatMode::Normal
            } else {
                ride_state.mode()
            },
            setpoint_adjustment,
            stop_condition,
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(wheelslip)
        .with_darkride(ride_state.darkride());
        let (mut balance_current, setpoints) = if resets_runtime_vars {
            // Upstream `STATE_STARTUP` calls `reset_runtime_vars(d)` before
            // `STATE_READY` at `src/main.c:833-837`; reset clears
            // `balance_current` at `src/main.c:246` and seeds runtime
            // setpoints from `d->imu.balance_pitch` at `src/main.c:249-255`.
            let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
                base.attitude().balance_pitch().angle().as_radians() * 180.0
                    / core::f32::consts::PI,
            ));
            (
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                RefloatRealtimeRuntimeSetpoints::new(
                    setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
                ),
            )
        } else {
            (base.balance_current(), base.setpoints())
        };
        if matches!(run_state, RefloatRunState::Running) {
            let kp = refloat_read_scaled_i16(
                [
                    self.serialized_config[REFLOAT_CONFIG_KP_OFFSET],
                    self.serialized_config[REFLOAT_CONFIG_KP_OFFSET + 1],
                ],
                10.0,
            );
            let balance_pitch_degrees = base.attitude().balance_pitch().angle().as_radians()
                * 180.0
                / core::f32::consts::PI;
            let setpoint_error = setpoints.board().angle().as_degrees() - balance_pitch_degrees;
            let angle_p_current = setpoint_error * kp;
            let smoothed_current =
                balance_current.current().current().as_amps() * 0.8 + angle_p_current * 0.2;
            // Upstream `pid_update` computes angle P at `src/pid.c:40` and
            // scales it by `kp` at `src/pid.c:69`; RUNNING then smooths
            // `balance_current` at `src/main.c:949-954`.
            balance_current = RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(
                Current::from_amps(smoothed_current),
            ));
            self.request_motor_current(balance_current.current());
        }
        let base = RefloatAllDataBasePayload::new(
            balance_current,
            RefloatAllDataAttitude::new(base.attitude().balance_pitch(), imu.roll(), imu.pitch()),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn handle_charging_state_packet(&mut self, bytes: &[u8]) -> bool {
        // Refloat v1.2.1 routes COMMAND_CHARGING_STATE at `src/main.c:2267-2269`;
        // the command ID is defined in `src/charging.h:25`.
        let [package_id, command_id, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != crate::domain::REFLOAT_APP_DATA_PACKAGE_ID.get()
            || RefloatAppDataCommand::try_from_id(*command_id)
                != Ok(RefloatAppDataCommand::ChargingState)
            || payload.len() < 6
            || payload[0] != 151
        {
            return false;
        }

        let (voltage, current) = if payload[1] > 0 {
            (
                refloat_read_scaled_i16([payload[2], payload[3]], 10.0),
                refloat_read_scaled_i16([payload[4], payload[5]], 10.0),
            )
        } else {
            (0.0, 0.0)
        };
        self.all_data_payloads =
            self.all_data_payloads
                .with_mode4_charging(RefloatAllDataMode4Payload::new(
                    RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(
                        current,
                    ))),
                    RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(
                        voltage,
                    ))),
                ));
        true
    }

    fn runtime_all_data_payloads<M: MotorTelemetryBindings>(
        self,
        payloads: RefloatAllDataPayloads,
        telemetry: &MotorTelemetryApi<M>,
        include_mode3: bool,
    ) -> RefloatAllDataPayloads {
        let payloads = payloads
            .with_mode2_distance_abs(telemetry.distance_abs())
            .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads.with_mode3_ride_totals(RefloatAllDataMode3Payload::new(
                telemetry.odometer(),
                telemetry.amp_hours_discharged(),
                telemetry.amp_hours_charged(),
                telemetry.watt_hours_discharged(),
                telemetry.watt_hours_charged(),
                telemetry.battery_level(),
            ))
        } else {
            payloads
        }
    }
}

fn refloat_read_scaled_i16(bytes: [u8; 2], scale: f32) -> f32 {
    i16::from_be_bytes(bytes) as f32 / scale
}

/// Refloat app-data lifecycle wiring.
pub struct RefloatAppDataLifecycle<B> {
    lifecycle: LoopbackLifecycle<B>,
}

impl<B: AppDataBindings> RefloatAppDataLifecycle<B> {
    /// Build Refloat app-data lifecycle wiring from firmware bindings.
    pub fn new(bindings: B) -> Self {
        Self {
            lifecycle: LoopbackLifecycle::new(bindings),
        }
    }

    /// Return the wrapped firmware bindings.
    pub fn bindings(&self) -> &B {
        self.lifecycle.bindings()
    }

    /// Install Refloat stop cleanup and app-data handler.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. The supplied
    /// handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install(
        &self,
        info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        unsafe {
            let _ = self.lifecycle.install(info, stop_refloat_app_data, handler);
        }
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Install Refloat stop cleanup and package-owned state without callbacks.
    ///
    /// Upstream stores `stop` and `Data *` in loader metadata at
    /// `src/main.c:2431-2432`, before registering custom config/app-data/LispBM
    /// callbacks at `src/main.c:2455-2459`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` must
    /// remain valid until the firmware stops the package. The supplied handler is
    /// not registered here; it is only passed through the SDK lifecycle install
    /// shape whose current implementation records the stop hook.
    pub unsafe fn install_refloat_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> bool {
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::from_mut(state).cast();
        }
        unsafe { self.lifecycle.install(info, stop_refloat_app_data, handler) }
    }

    /// Install Refloat state, stop cleanup, and app-data handler.
    ///
    /// Upstream stores `Data *`/`stop` in loader metadata at
    /// `src/main.c:2431-2432`; app-data registration follows later at
    /// `src/main.c:2456`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` and
    /// `handler` must remain valid until firmware clears/replaces the handler
    /// and stops the package.
    pub unsafe fn install_with_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = unsafe { self.install_refloat_state(info, state, handler) };
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Clear Refloat callbacks during package stop.
    ///
    /// Refloat `v1.2.1` clears app-data at `src/main.c:2402` and custom config
    /// at `src/main.c:2403`.
    pub fn stop(&self) -> Result<(), AppDataHandlerRegistrationError>
    where
        B: CustomConfigBindings,
    {
        let app_data_result = self.lifecycle.clear_app_data_handler();
        unsafe {
            let _ = self.lifecycle.bindings().clear_custom_configs();
        }
        app_data_result
    }

    /// Process one Refloat app-data packet and send a response when accepted.
    pub fn send_response(&self, payloads: RefloatAllDataPayloads, bytes: &[u8]) -> bool {
        let Some(response) = process_refloat_app_data(payloads, bytes) else {
            return false;
        };
        self.send_response_bytes(response.as_bytes())
    }

    /// Encode and send one parsed Refloat all-data response.
    pub fn send_all_data_response(
        &self,
        payloads: RefloatAllDataPayloads,
        request: RefloatAllDataRequest,
    ) -> bool {
        let response = payloads.encode_response(request);
        self.send_response_bytes(response.as_bytes())
    }

    fn send_response_bytes(&self, bytes: &[u8]) -> bool {
        unsafe {
            self.lifecycle
                .send_app_data(bytes.as_ptr(), bytes.len() as u32)
        };
        true
    }
}

impl<B: AppDataBindings + CustomConfigBindings> RefloatAppDataLifecycle<B> {
    /// Install Refloat custom config and app-data callbacks.
    ///
    /// Upstream registers custom config before app-data at `src/main.c:2456-2457`,
    /// after loader metadata receives `stop`/`Data *` at `src/main.c:2431-2432`.
    ///
    /// # Safety
    ///
    /// The supplied handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install_refloat_callbacks(
        &self,
        _info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = register_refloat_custom_config(self.bindings());
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Install Refloat state plus custom config and app-data callbacks.
    ///
    /// Upstream stores `Data *` in `info->arg` at `src/main.c:2432` before
    /// registering custom config and app-data at `src/main.c:2456-2457`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` and
    /// `handler` must remain valid until firmware clears/replaces the handler
    /// and stops the package.
    pub unsafe fn install_refloat_callbacks_with_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = unsafe { self.install_refloat_state(info, state, handler) };
        unsafe { self.install_refloat_callbacks(info, handler) }
    }
}

unsafe extern "C" fn stop_refloat_app_data(_arg: *mut core::ffi::c_void) {
    // Upstream stop cleanup in `src/main.c:2398-2412` clears IMU/app-data/custom
    // config callbacks, terminates aux+main threads, destroys LEDs, and frees
    // `Data`. This isolated handler only clears app-data/custom config and frees
    // the narrow Rust app-data allocation if that experimental path was installed.
    #[cfg(not(test))]
    {
        let _ = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings).stop();
    }
    #[cfg(all(not(test), target_arch = "arm"))]
    if let Some(ptr) = core::ptr::NonNull::new(_arg.cast::<RefloatAppDataState>()) {
        let bindings = vescpkg_rs::RealBindings;
        crate::runtime::request_refloat_runtime_thread_termination(unsafe { ptr.as_ref() });
        let _allocation =
            unsafe { vescpkg_rs::FirmwareAllocation::from_raw_parts(ptr, 1, &bindings) };
    }
}

#[cfg(test)]
mod tests {
    use super::{RefloatAppDataLifecycle, RefloatAppDataState};
    use super::{
        allocate_refloat_startup_app_data_with, handle_refloat_app_data_packet,
        install_refloat_startup_app_data_with, process_refloat_app_data,
    };
    use crate::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature,
        RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
        RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataStatus,
        RefloatAppDataCommand, RefloatBeepReason, RefloatChargingState, RefloatDarkRideState,
        RefloatFocIdCurrent, RefloatMode, RefloatRealtimeBalanceCurrent,
        RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent, RefloatRealtimeChargingCurrent,
        RefloatRealtimeChargingVoltage, RefloatRealtimeMotorTemperatures,
        RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints, RefloatRideState,
        RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition, RefloatWheelSlipState,
    };
    use core::cell::Cell;
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::{
        FakeImuBindings, FakeMotorControlBindings, FakeMotorTelemetryBindings,
    };
    use vescpkg_rs::{AllocBindings, AppDataBindings, FirmwareAllocator, ffi};

    #[test]
    fn app_data_processes_all_data_requests_from_payload_snapshot() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        )
        .expect("all-data request should produce a response");

        assert_eq!(response.as_bytes().len(), 58);
        assert_eq!(&response.as_bytes()[..3], &[101, 10, 4]);
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::GetAllData.id(),
                ]
            ),
            None
        );
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::PrintInfo.id(),
                    4,
                ]
            ),
            None
        );
    }

    #[test]
    fn app_data_processes_info_v2_request_like_refloat_qml() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::Info.id(),
                2,
                0,
            ],
        )
        .expect("info request should produce a response");
        let bytes = response.as_bytes();

        // QML sends COMMAND_INFO version 2 at `ui.qml.in:693-697`; upstream
        // `cmd_info` replies with the v2 metadata layout at `src/main.c:2108-2135`.
        assert_eq!(bytes.len(), 60);
        assert_eq!(&bytes[..4], &[101, 0, 2, 0]);
        assert_eq!(&bytes[4..11], b"Refloat");
        assert_eq!(&bytes[24..27], &[1, 2, 1]);
        assert_eq!(
            u32::from_be_bytes([bytes[47], bytes[48], bytes[49], bytes[50]]),
            0x0ef6_e99d
        );
        assert_eq!(
            u32::from_be_bytes([bytes[51], bytes[52], bytes[53], bytes[54]]),
            10_000
        );
        assert_eq!(
            u32::from_be_bytes([bytes[55], bytes[56], bytes[57], bytes[58]]),
            0
        );
        assert_eq!(bytes[59], 0);
    }

    #[test]
    fn app_data_processes_realtime_data_ids_like_refloat_qml() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeDataIds.id(),
            ],
        )
        .expect("realtime data IDs request should produce a response");
        let bytes = response.as_bytes();

        // QML asks for IDs at `ui.qml.in:704-705`; upstream
        // `cmd_realtime_data_ids` writes the two counted ID sets at
        // `src/main.c:1876-1901`.
        assert_eq!(bytes.len(), 405);
        assert_eq!(&bytes[..3], &[101, 32, 16]);
        assert_eq!(bytes[3], b"motor.speed".len() as u8);
        assert_eq!(&bytes[4..15], b"motor.speed");
        assert_eq!(bytes[243], 10);
        assert_eq!(bytes[244], b"setpoint".len() as u8);
        assert_eq!(&bytes[245..253], b"setpoint");
    }

    #[test]
    fn app_data_processes_legacy_get_rtdata_like_refloat() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetRealtimeData.id(),
            ],
        )
        .expect("legacy realtime-data request should produce a response");
        let bytes = response.as_bytes();

        // Upstream dispatches `COMMAND_GET_RTDATA` at `src/main.c:2162-2164`;
        // `send_realtime_data` writes this 72-byte response at
        // `src/main.c:1267-1310`.
        assert_eq!(bytes.len(), 72);
        assert_eq!(&bytes[..2], &[101, 1]);
        assert_f32_be(bytes, 2, 9.0);
        assert_f32_be(bytes, 6, 1.2);
        assert_f32_be(bytes, 10, -0.5);
        assert_eq!(bytes[14], 0x21);
        assert_eq!(bytes[15], 0x12);
        assert_f32_be(bytes, 16, 0.60);
        assert_f32_be(bytes, 20, 0.40);
        assert_f32_be(bytes, 24, 1.0);
        assert_f32_be(bytes, 32, -1.0);
        assert_f32_be(bytes, 44, 3.0);
        assert_f32_be(bytes, 48, 2.3);
        assert_f32_be(bytes, 52, 5.0);
        assert_f32_be(bytes, 56, 0.0);
        assert_f32_be(bytes, 60, 4.0);
        assert_f32_be(bytes, 64, 5.0);
        assert_f32_be(bytes, 68, 0.0);
    }

    #[test]
    fn app_data_processes_non_running_realtime_data_like_refloat_qml() {
        let response = process_refloat_app_data(
            RefloatAllDataPayloads::source_startup(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        )
        .expect("realtime data request should produce a response");
        let bytes = response.as_bytes();

        // QML reads `c_REALTIME_DATA` at `ui.qml.in:853-925`; upstream
        // `cmd_realtime_data` writes this non-running packet shape at
        // `src/main.c:1904-1960`.
        assert_eq!(bytes.len(), 53);
        assert_eq!(&bytes[..2], &[101, 31]);
        assert_eq!(bytes[2], 0x04);
        assert_eq!(bytes[3], 0);
        assert_eq!(&bytes[4..8], &[0, 0, 0, 0]);
        assert_eq!(bytes[8], 1);
        assert_eq!(bytes[9], 0);
        assert_eq!(bytes[10], 0);
        assert_eq!(bytes[11], 0);
        assert!(bytes[12..44].iter().all(|byte| *byte == 0));
        assert_eq!(&bytes[44..48], &[0, 0, 0, 0]);
        assert_eq!(&bytes[48..52], &[0, 0, 0, 0]);
        assert_eq!(bytes[52], 0);
    }

    #[track_caller]
    fn assert_f32_be(bytes: &[u8], offset: usize, expected: f32) {
        assert_eq!(
            u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            expected.to_bits(),
        );
    }

    #[test]
    fn lifecycle_installs_app_data_handler_and_stop_cleanup() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(unsafe { lifecycle.install(&mut info, handler) }, Ok(()));
        assert!(info.stop_fun.is_some());
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            handler as *const () as usize
        );

        assert_eq!(lifecycle.stop(), Ok(()));
        assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
        assert_eq!(lifecycle.bindings().last_handler.get(), 0);
        // Refloat v1.2.1 stop clears app-data at `src/main.c:2402` and
        // custom config at `src/main.c:2403`.
        assert_eq!(lifecycle.bindings().custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn lifecycle_sends_refloat_app_data_responses_through_bindings() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());

        assert!(lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 4]);

        assert!(lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::Info.id(),
                2,
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 2);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 60);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 0, 2]);

        assert!(!lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::PrintInfo.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 2);
    }

    #[test]
    fn app_data_state_handles_packets_through_lifecycle_send_boundary() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet(
            &lifecycle,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(state.all_data_payloads(), sample_all_data_payloads());
    }

    #[test]
    fn app_data_state_refreshes_mode2_distance_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_distance_bits.get(),
            12.5_f32.to_bits()
        );
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 1);
    }

    #[test]
    fn app_data_state_refreshes_mode2_temperatures_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_temperatures(
                MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
                MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
            ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_temperature_bytes.get(),
            [74, 97]
        );
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_mode3_ride_totals_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_ride_totals(
            OdometerMeters::from_meters(123_456),
            AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
            AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
            WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
            WattHoursCharged::new(Energy::from_watt_hours(18.5)),
            BatteryLevel::new(Ratio::from_ratio_const(0.72)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                3,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 54);
        assert_eq!(
            lifecycle.bindings().last_sent_mode3_ride_total_bytes.get(),
            [0, 1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144]
        );
        assert_eq!(telemetry.bindings().odometer_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 1);
    }

    #[test]
    fn app_data_state_sends_fault_response_before_refreshing_mode_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_firmware_fault(FirmwareFaultCode::from_compat_code(5)),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 4);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 69]);
        assert_eq!(telemetry.bindings().firmware_fault_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_updates_mode4_charging_fields_from_charging_state_command() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::ChargingState.id(),
                151,
                1,
                1,
                244,
                0,
                123,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 0);

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(
            lifecycle.bindings().last_sent_mode4_charging_bytes.get(),
            [0, 123, 1, 244]
        );
    }

    #[test]
    fn app_data_state_does_not_refresh_distance_for_base_all_data() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_base_battery_voltage_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_input_voltage_filtered(InputVoltage::new(Voltage::from_volts(84.2))),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_base_motor_voltage_bytes
                .get(),
            [3, 74]
        );
        assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_realtime_voltage_and_temperatures_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::with_input_voltage_and_temperatures(
                InputVoltage::new(Voltage::from_volts(84.2)),
                MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
                MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
            ),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        // Refloat writes realtime values as float16 at `src/main.c:1943-1954`
        // using `buffer_append_float16_auto` from `src/conf/buffer.c:143-145`.
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 53);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 31, 4]);
        assert_eq!(
            lifecycle.bindings().last_sent_realtime_voltage_bytes.get(),
            [85, 67]
        );
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_realtime_temperature_bytes
                .get(),
            [80, 160, 82, 16]
        );
        assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_realtime_timestamp_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(0x0102_0304),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Refloat v1.2.1 writes `d->time.now` into realtime packets at
        // `src/main.c:1931`; VESC system ticks are 100 us ticks.
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_realtime_timestamp_bytes
                .get(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn app_data_runtime_refreshes_startup_ready_gate_and_imu_attitude_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.25)),
                    ImuPitch::new(AngleRadians::from_radians(-0.125)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let payloads = state.all_data_payloads();
        assert_eq!(
            payloads.base().status().ride_state().run_state(),
            RefloatRunState::Ready
        );
        assert_eq!(payloads.base().attitude().roll().angle().as_radians(), 0.25);
        assert_eq!(
            payloads.base().attitude().pitch().angle().as_radians(),
            -0.125
        );
    }

    #[test]
    fn app_data_startup_ready_resets_runtime_vars_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.25)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Startup,
            RefloatMode::Normal,
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        assert_eq!(
            base.status().ride_state().run_state(),
            RefloatRunState::Ready
        );
        // Refloat calls `reset_runtime_vars(d)` before READY at
        // `src/main.c:833-837`; reset clears `balance_current` at
        // `src/main.c:246` and seeds all runtime setpoints from
        // `d->imu.balance_pitch` at `src/main.c:249-255`.
        assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
        let expected_startup_setpoint = 1.2 * 180.0 / core::f32::consts::PI;
        assert_eq!(
            base.setpoints().board().angle().as_degrees(),
            expected_startup_setpoint
        );
        assert_eq!(
            base.setpoints().atr().angle().as_degrees(),
            expected_startup_setpoint
        );
        assert_eq!(
            base.setpoints().brake_tilt().angle().as_degrees(),
            expected_startup_setpoint
        );
        assert_eq!(
            base.setpoints().torque_tilt().angle().as_degrees(),
            expected_startup_setpoint
        );
        assert_eq!(
            base.setpoints().turn_tilt().angle().as_degrees(),
            expected_startup_setpoint
        );
        assert_eq!(
            base.setpoints().remote().angle().as_degrees(),
            expected_startup_setpoint
        );
    }

    #[test]
    fn app_data_running_flywheel_both_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops RUNNING FLYWHEEL when both footpads
        // are engaged at `src/main.c:491-493`; `state_stop` moves to READY
        // and stores the stop condition at `src/state.c:29-33`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_running_flywheel_stop_clears_wheelslip_like_refloat_state_stop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads = sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
        );
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_wheelslip(RefloatWheelSlipState::Detected);
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `state_stop` clears wheelslip at `src/state.c:29-33`.
        assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::None);
    }

    #[test]
    fn app_data_running_reverse_stop_no_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` immediately stops reverse-stop mode when
        // the footpad is fully open at `src/main.c:418-422`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchFull
        );
    }

    #[test]
    fn app_data_running_quickstop_no_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(15.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let motor = RefloatAllDataMotorPayload::new(
            base.motor().battery_voltage(),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
            base.motor().vehicle_speed(),
            base.motor().motor_current(),
            base.motor().battery_current(),
            base.motor().duty_cycle(),
            base.motor().foc_id_current(),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            setpoints,
            base.booster_current(),
            motor,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET] = 1;
        state.store_serialized_config(&config);

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` quick-stops no-footpad low-speed
        // pitch-runaway cases at `src/main.c:419-423`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::QuickStop);
    }

    #[test]
    fn app_data_running_reverse_stop_high_pitch_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(19.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` immediately stops reverse-stop mode when
        // `fabsf(d->imu.pitch) > 18` at `src/main.c:423-426`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::ReverseStop
        );
    }

    #[test]
    fn app_data_running_darkride_footpads_stop_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                base.balance_current(),
                base.attitude(),
                RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
                base.footpad(),
                base.setpoints(),
                base.booster_current(),
                base.motor(),
            ),
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream darkride `check_faults(d)` allows turning it off by
        // engaging foot sensors at `src/main.c:387-390`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_ready_normal_both_footpads_engages_like_refloat_start_conditions() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.1)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY engages when startup pitch/roll tolerances and
        // `can_engage(d)` pass at `src/main.c:1033-1036`; `state_engage`
        // moves to RUNNING and sets SAT_CENTERING at `src/state.c:36-39`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
    }

    #[test]
    fn app_data_ready_normal_charging_does_not_engage_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let charging_state = base
            .status()
            .ride_state()
            .with_charging(RefloatChargingState::Charging);
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.1)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(charging_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` rejects charging state before checking
        // footpads at `src/main.c:328-331`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.charging(), RefloatChargingState::Charging);
    }

    #[test]
    fn app_data_ready_flywheel_without_footpads_engages_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Flywheel);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.1)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` keeps FLYWHEEL mode engaged after footpad
        // checks at `src/main.c:346-349`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_ready_flywheel_both_footpads_stops_flywheel_like_refloat_ready_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Flywheel,
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY handles FLYWHEEL abort/both-footpad before start
        // conditions at `src/main.c:957-963`; `flywheel_stop` returns to
        // NORMAL mode at `src/main.c:1869-1873`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.mode(), RefloatMode::Normal);
    }

    #[test]
    fn app_data_ready_single_footpad_engages_when_dual_switch_config_is_set() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.1)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *include_bytes!("conf/default_config.dat");
        config[super::REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` allows a single footpad when
        // `fault_is_dual_switch` is enabled at `src/main.c:338-342`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_ready_single_footpad_default_config_does_not_engage_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.1)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` keeps a single footpad gated unless
        // `fault_is_dual_switch` or simple start is enabled at `src/main.c:338-342`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::None
        );
    }

    #[test]
    fn app_data_runtime_applies_disabled_config_before_startup_ready_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_DISABLED_OFFSET] = 1;
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));
        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Upstream `configure(d)` applies `disabled` before the control-loop
        // startup gate at `src/main.c:184-190`; `state_set_disabled` forces
        // `STATE_DISABLED` at `src/state.c:41-47`, so `src/main.c:833-838`
        // cannot promote STARTUP to READY in this configuration.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            RefloatRunState::Disabled,
        );
    }

    #[test]
    fn app_data_configured_loop_time_uses_refloat_hertz_config() {
        let mut incoming = *include_bytes!("conf/default_config.dat");
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert_eq!(state.configured_loop_time_us(), 1201);

        incoming[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
            .copy_from_slice(&500u16.to_be_bytes());
        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        // Upstream generated serialization places `hertz` after the first
        // seven float16 config fields; `configure(d)` then uses it as
        // `1e6 / d->float_conf.hertz` at `src/main.c:190-191`.
        assert_eq!(state.configured_loop_time_us(), 2000);
    }

    #[test]
    fn app_data_motor_control_applies_requested_current_like_refloat_motor_control() {
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        state.request_motor_current(MotorCurrent::new(Current::from_amps(6.25)));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `motor_control_apply` resets timeout, keeps current control
        // on for 50ms, sends the requested current, then clears the request at
        // `src/motor_control.c:92-99` and `src/motor_control.c:121-122`.
        assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
        assert_eq!(motor.bindings().set_current_off_delay_calls.get(), 1);
        assert_eq!(motor.bindings().current_off_delay_seconds(), 0.05);
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().current().current().as_amps(), 6.25);
        assert!(!state.apply_requested_motor_current(&motor));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
    }

    #[test]
    fn app_data_running_runtime_requests_balance_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING computes `d->balance_current` and then requests it
        // via `motor_control_request_current` at `src/main.c:949-956`.
        assert_eq!(motor.bindings().current().current().as_amps(), 3.8);
    }

    #[test]
    fn app_data_running_computes_angle_p_balance_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `pid_update` computes angle P at `src/pid.c:40` and scales
        // it by `kp` at `src/pid.c:69`; RUNNING then smooths balance current
        // as `old * 0.8 + new_current * 0.2` at `src/main.c:932-954`.
        assert_eq!(motor.bindings().current().current().as_amps(), 12.0);
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .balance_current()
                .current()
                .current()
                .as_amps(),
            12.0
        );
    }

    #[test]
    fn app_data_motor_control_sets_zero_once_while_disabled_like_refloat() {
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.apply_motor_control(&motor, RefloatRunState::Disabled));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().current().current().as_amps(), 0.0);

        assert!(!state.apply_motor_control(&motor, RefloatRunState::Disabled));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);

        assert!(!state.apply_motor_control(&motor, RefloatRunState::Ready));
        assert!(state.apply_motor_control(&motor, RefloatRunState::Disabled));
        assert_eq!(motor.bindings().set_current_calls.get(), 2);
    }

    #[test]
    fn app_data_runtime_refreshes_motor_payload_like_refloat_motor_data_update() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
                MotorCurrent::new(Current::from_amps(12.25)),
                BatteryCurrent::new(Current::from_amps(4.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
            ));
        let imu = ImuApi::new(FakeImuBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));

        let motor = state.all_data_payloads().base().motor();
        assert_eq!(
            motor.electrical_speed().rpm().as_revolutions_per_minute(),
            1234.0
        );
        assert_eq!(motor.vehicle_speed().speed().as_meters_per_second(), 5.5);
        assert_eq!(motor.motor_current().current().as_amps(), 12.25);
        assert_eq!(motor.battery_current().current().as_amps(), 0.04);
        assert_eq!(motor.duty_cycle().ratio().as_ratio(), 0.375);
    }

    #[test]
    fn app_data_runtime_refreshes_foc_id_current_like_refloat_all_data() {
        // Refloat v1.2.1 encodes `fabsf(VESC_IF->foc_get_id()) * 3` for
        // compact all-data at `src/main.c:1364-1368`.
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_foc_id_current(Some(MotorCurrent::new(Current::from_amps(-4.0)))),
        );
        let imu = ImuApi::new(FakeImuBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));

        let motor = state.all_data_payloads().base().motor();
        assert_eq!(
            motor
                .foc_id_current()
                .as_measured()
                .expect("measured Id current")
                .current()
                .as_amps(),
            -4.0
        );
        assert_eq!(lifecycle.bindings().last_sent_base_foc_id_byte.get(), 12);
    }

    #[test]
    fn lifecycle_installs_typed_refloat_state_for_handler_retrieval() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(
            unsafe { lifecycle.install_with_state(&mut info, &mut state, handler) },
            Ok(())
        );
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            sample_all_data_payloads()
        );
        let mut empty_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        assert!(unsafe { RefloatAppDataState::from_info_arg(&mut empty_info) }.is_none());
    }

    #[test]
    fn lifecycle_installs_refloat_state_before_callbacks_like_refloat_startup() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe { lifecycle.install_refloat_state(&mut info, &mut state, handler) });
        // Upstream sets `info->stop_fun` and `info->arg` at `src/main.c:2431-2432`,
        // before registering custom config/app-data/extensions at `src/main.c:2455-2459`.
        assert_eq!(lifecycle.bindings().handler_calls.get(), 0);
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
        assert!(info.stop_fun.is_some());
        assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
    }

    #[test]
    fn raw_handler_boundary_rejects_null_and_sends_valid_packets() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(!unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            let imu = ImuApi::new(FakeImuBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                core::ptr::null_mut(),
                0,
            )
        });

        let mut request = [101, 10, 0];
        assert!(unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            let imu = ImuApi::new(FakeImuBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                request.as_mut_ptr(),
                request.len() as u32,
            )
        });
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 0]);
    }

    #[test]
    fn startup_app_data_install_seeds_state_and_registers_handler() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe {
            install_refloat_startup_app_data_with(&mut info, &mut state, &lifecycle, handler)
        });
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            state.all_data_payloads(),
            RefloatAllDataPayloads::source_startup()
        );
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    #[test]
    fn startup_app_data_install_uses_firmware_allocated_state() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut backing = MaybeUninit::<RefloatAppDataState>::uninit();
        let alloc_bindings = RecordingAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&alloc_bindings);

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe {
            allocate_refloat_startup_app_data_with(&mut info, &allocator, &lifecycle, handler)
        });
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 1);
        assert_eq!(alloc_bindings.malloc_calls.get(), 1);
        assert_eq!(
            alloc_bindings.last_requested_len.get(),
            core::mem::size_of::<RefloatAppDataState>()
        );
        assert_eq!(alloc_bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("allocated state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    #[test]
    fn custom_config_xml_callback_returns_upstream_settings_blob() {
        let mut buffer = core::ptr::null_mut();

        let len = unsafe { super::refloat_get_cfg_xml(&mut buffer) };

        // Refloat v1.2.1 returns generated `data_refloatconfig_` at
        // `src/main.c:2388-2396`, produced from `src/conf/settings.xml` by
        // `src/Makefile:28-31`.
        assert_eq!(len, 25_723);
        assert!(!buffer.is_null());
        let bytes = unsafe { core::slice::from_raw_parts(buffer.cast_const(), len as usize) };
        assert_eq!(&bytes[..6], &[0x00, 0x05, 0x5c, 0xa1, 0x78, 0xda]);
    }

    #[test]
    fn custom_config_default_callback_returns_upstream_serialized_defaults() {
        let mut buffer = [0u8; 276];

        let len = unsafe { super::refloat_get_cfg(buffer.as_mut_ptr(), true) };

        // Refloat v1.2.1 default `get_cfg` allocates a temporary config,
        // applies generated defaults, and serializes it at `src/main.c:2339-2350`.
        // The generated format comes from `src/Makefile:28-31`;
        // generated `conf/confparser.h:11-12` fixes signature/length, and
        // generated `conf/confparser.c:8-178,363-531` writes these bytes.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("conf/default_config.dat"));
        assert_eq!(&buffer[..4], &[0x90, 0xb7, 0xa9, 0xba]);
    }

    #[test]
    fn custom_config_current_callback_reads_state_serialized_config() {
        let state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut buffer = [0u8; 276];

        let len = super::refloat_get_cfg_with_state(buffer.as_mut_ptr(), false, Some(&state));

        // Upstream current `get_cfg` serializes `d->float_conf` from shared
        // package state at `src/main.c:2347-2350`; `data_init` populates it
        // from EEPROM or generated defaults at `src/main.c:1160-1185`.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("conf/default_config.dat"));
    }

    #[test]
    fn custom_config_set_callback_stores_serialized_config_in_state() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream `set_cfg` deserializes into `d->float_conf` at
        // `src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
        // bad signature before reading the field bytes.
        incoming[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 0;
        assert_eq!(len, 276);
        assert_eq!(current, incoming);
    }

    #[test]
    fn custom_config_set_callback_resets_is_default_flag_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 1;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream clears `d->float_conf.meta.is_default` for every config
        // write at `src/main.c:2375-2377`; generated
        // `conf/confparser.c:179` serializes that flag as the final byte.
        assert_eq!(len, 276);
        assert_eq!(current[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_keeps_package_enabled_while_running_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_DISABLED_OFFSET] = 1;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream refuses to persist `disabled = true` while running at
        // `src/main.c:2369-2372`; `disabled` is serialized at
        // `src/conf/settings.xml:4064`.
        assert_eq!(len, 276);
        assert_eq!(current[super::REFLOAT_CONFIG_DISABLED_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_rejects_special_modes_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::HandTest,
        ));
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(!super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream rejects VESC Tool config writes outside `MODE_NORMAL` at
        // `src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
        assert_eq!(len, 276);
        assert_eq!(current, *include_bytes!("conf/default_config.dat"));
    }

    struct RecordingAppDataBindings {
        handler_calls: Cell<usize>,
        last_handler: Cell<usize>,
        send_calls: Cell<usize>,
        last_sent_len: Cell<u32>,
        last_sent_prefix: Cell<[u8; 3]>,
        last_sent_base_foc_id_byte: Cell<u8>,
        last_sent_base_motor_voltage_bytes: Cell<[u8; 2]>,
        last_sent_realtime_timestamp_bytes: Cell<[u8; 4]>,
        last_sent_realtime_voltage_bytes: Cell<[u8; 2]>,
        last_sent_realtime_temperature_bytes: Cell<[u8; 4]>,
        last_sent_mode2_distance_bits: Cell<u32>,
        last_sent_mode2_temperature_bytes: Cell<[u8; 2]>,
        last_sent_mode3_ride_total_bytes: Cell<[u8; 13]>,
        last_sent_mode4_charging_bytes: Cell<[u8; 4]>,
        custom_config_register_calls: Cell<usize>,
        custom_config_clear_calls: Cell<usize>,
        system_time_ticks: Cell<u32>,
        handler_results: Cell<[bool; 2]>,
    }

    struct RecordingAllocBindings {
        malloc_calls: Cell<usize>,
        free_calls: Cell<usize>,
        next_ptr: Cell<*mut c_void>,
        last_requested_len: Cell<usize>,
    }

    impl RecordingAllocBindings {
        fn new(next_ptr: *mut c_void) -> Self {
            Self {
                malloc_calls: Cell::new(0),
                free_calls: Cell::new(0),
                next_ptr: Cell::new(next_ptr),
                last_requested_len: Cell::new(0),
            }
        }
    }

    impl AllocBindings for RecordingAllocBindings {
        unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
            self.malloc_calls.set(self.malloc_calls.get() + 1);
            self.last_requested_len.set(bytes);
            self.next_ptr.get()
        }

        unsafe fn free(&self, _ptr: *mut c_void) {
            self.free_calls.set(self.free_calls.get() + 1);
        }
    }

    impl RecordingAppDataBindings {
        fn accepting() -> Self {
            Self {
                handler_calls: Cell::new(0),
                last_handler: Cell::new(0),
                send_calls: Cell::new(0),
                last_sent_len: Cell::new(0),
                last_sent_prefix: Cell::new([0; 3]),
                last_sent_base_foc_id_byte: Cell::new(0),
                last_sent_base_motor_voltage_bytes: Cell::new([0; 2]),
                last_sent_realtime_timestamp_bytes: Cell::new([0; 4]),
                last_sent_realtime_voltage_bytes: Cell::new([0; 2]),
                last_sent_realtime_temperature_bytes: Cell::new([0; 4]),
                last_sent_mode2_distance_bits: Cell::new(0),
                last_sent_mode2_temperature_bytes: Cell::new([0; 2]),
                last_sent_mode3_ride_total_bytes: Cell::new([0; 13]),
                last_sent_mode4_charging_bytes: Cell::new([0; 4]),
                custom_config_register_calls: Cell::new(0),
                custom_config_clear_calls: Cell::new(0),
                system_time_ticks: Cell::new(0),
                handler_results: Cell::new([true, true]),
            }
        }

        fn with_system_time_ticks(self, ticks: u32) -> Self {
            self.system_time_ticks.set(ticks);
            self
        }
    }

    impl AppDataBindings for RecordingAppDataBindings {
        unsafe fn set_app_data_handler(&self, handler: ffi::AppDataHandler) -> bool {
            self.handler_calls.set(self.handler_calls.get() + 1);
            self.last_handler.set(handler as *const () as usize);
            let index = self.handler_calls.get().saturating_sub(1).min(1);
            self.handler_results.get()[index]
        }

        unsafe fn clear_app_data_handler(&self) -> bool {
            self.handler_calls.set(self.handler_calls.get() + 1);
            self.last_handler.set(0);
            let index = self.handler_calls.get().saturating_sub(1).min(1);
            self.handler_results.get()[index]
        }

        fn system_time_ticks(&self) -> u32 {
            self.system_time_ticks.get()
        }

        unsafe fn send_app_data(&self, data: *const u8, len: u32) {
            self.send_calls.set(self.send_calls.get() + 1);
            self.last_sent_len.set(len);
            if len >= 3 {
                let bytes = unsafe { core::slice::from_raw_parts(data, len as usize) };
                self.last_sent_prefix.set([bytes[0], bytes[1], bytes[2]]);
                if bytes.len() >= 24 {
                    self.last_sent_base_motor_voltage_bytes
                        .set([bytes[22], bytes[23]]);
                }
                if bytes.len() >= 34 {
                    self.last_sent_base_foc_id_byte.set(bytes[33]);
                }
                if bytes.len() >= 32 && bytes[1] == RefloatAppDataCommand::RealtimeData.id() {
                    self.last_sent_realtime_timestamp_bytes
                        .set([bytes[4], bytes[5], bytes[6], bytes[7]]);
                    self.last_sent_realtime_voltage_bytes
                        .set([bytes[24], bytes[25]]);
                    self.last_sent_realtime_temperature_bytes
                        .set([bytes[28], bytes[29], bytes[30], bytes[31]]);
                }
                if bytes.len() >= 38 {
                    self.last_sent_mode2_distance_bits.set(u32::from_be_bytes([
                        bytes[34], bytes[35], bytes[36], bytes[37],
                    ]));
                }
                if bytes.len() >= 40 {
                    self.last_sent_mode2_temperature_bytes
                        .set([bytes[38], bytes[39]]);
                }
                if bytes.len() >= 54 {
                    self.last_sent_mode3_ride_total_bytes.set([
                        bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46],
                        bytes[47], bytes[48], bytes[49], bytes[50], bytes[51], bytes[52],
                        bytes[53],
                    ]);
                }
                if bytes.len() >= 58 {
                    self.last_sent_mode4_charging_bytes
                        .set([bytes[54], bytes[55], bytes[56], bytes[57]]);
                }
            }
        }
    }

    impl CustomConfigBindings for RecordingAppDataBindings {
        unsafe fn register_custom_config(
            &self,
            _get_cfg: ffi::raw::CustomConfigGet,
            _set_cfg: ffi::raw::CustomConfigSet,
            _get_cfg_xml: ffi::raw::CustomConfigXml,
        ) -> bool {
            // Refloat v1.2.1 registers custom config during init at `src/main.c:2456`.
            self.custom_config_register_calls
                .set(self.custom_config_register_calls.get() + 1);
            true
        }

        unsafe fn clear_custom_configs(&self) -> bool {
            // Refloat v1.2.1 clears custom config during stop at `src/main.c:2403`.
            self.custom_config_clear_calls
                .set(self.custom_config_clear_calls.get() + 1);
            true
        }
    }

    fn sample_all_data_payloads() -> RefloatAllDataPayloads {
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal)
    }

    fn sample_all_data_payloads_with_ride_state(
        run_state: RefloatRunState,
        mode: RefloatMode,
    ) -> RefloatAllDataPayloads {
        let ride_state = RefloatRideState::new(
            run_state,
            mode,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.60)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.40)),
            FootpadSensorState::Both,
        );
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
        );

        RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
                    ImuRoll::new(AngleRadians::from_radians(-0.5)),
                    ImuPitch::new(AngleRadians::from_radians(2.3)),
                ),
                RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
                footpad,
                setpoints,
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
                RefloatAllDataMotorPayload::new(
                    BatteryVoltage::new(Voltage::from_volts(72.0)),
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
                    MotorCurrent::new(Current::from_amps(5.0)),
                    BatteryCurrent::new(Current::from_amps(-2.0)),
                    DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
                    RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
                ),
            ),
            RefloatAllDataMode2Payload::new(
                TripDistance::new(Distance::from_meters(64.0)),
                RefloatRealtimeMotorTemperatures::new(
                    MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
                    MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
                ),
                RefloatAllDataBatteryTemperature::unavailable(),
            ),
            RefloatAllDataMode3Payload::new(
                OdometerMeters::from_meters(123_456),
                AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
                AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
                WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
                WattHoursCharged::new(Energy::from_watt_hours(18.5)),
                BatteryLevel::new(Ratio::from_ratio_const(0.72)),
            ),
            RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
                RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
            ),
        )
    }
}
