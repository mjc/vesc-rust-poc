use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RUNTIME_ITEMS,
    RefloatAllDataPayloads, RefloatAllDataRequest, RefloatAllDataResponse, RefloatAppDataCommand,
    RefloatChargingState, RefloatDarkRideState, RefloatMode, RefloatRealtimeDataItem,
    RefloatRunState, RefloatWheelSlipState,
};
use vescpkg_rs::prelude::{SystemTimestamp, TimestampTicks};

// Refloat v1.2.1 `cmd_info` writes this version-2 response shape at
// `third_party/refloat/src/main.c:2070-2139`.
const REFLOAT_INFO_RESPONSE_V2_LEN: usize = 60;
// Refloat v1.2.1 `cmd_realtime_data_ids` writes the counted ID-list packet at
// `third_party/refloat/src/main.c:1876-1901`.
const REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN: usize = 405;
// Refloat v1.2.1 `send_realtime_data` declares its fixed buffer at
// `third_party/refloat/src/main.c:1267-1269`.
const REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN: usize = 72;
// Refloat v1.2.1 `cmd_realtime_data` declares its runtime-sized packet at
// `third_party/refloat/src/main.c:1904-1906`.
const REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY: usize = 77;
const REFLOAT_PACKAGE_NAME: &[u8] = b"Refloat";
const REFLOAT_VERSION_SUFFIX: &[u8] = b"";
const REFLOAT_GIT_HASH: u32 = 0x0ef6_e99d;
const REFLOAT_SYSTEM_TICK_RATE_HZ: u32 = 10_000;

// Refloat C builds this exact packet in `third_party/refloat/src/main.c:1876-1901`, using the ID
// order from `third_party/refloat/src/rt_data.h:38-66` and counted-string framing from
// `third_party/refloat/src/conf/buffer.c:147-155`. QML reads the same two string lists in
// `ui.qml.in:926-934`.
// Keep the materialized bytes in the loaded extension image so hardware never
// has to dereference string-literal storage.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_realtime_data_ids")
)]
#[used]
static REFLOAT_REALTIME_DATA_IDS_RESPONSE_BYTES: [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN] =
    build_refloat_realtime_data_ids_response();

/// Variable-length Refloat `COMMAND_REALTIME_DATA` response bytes from
/// `third_party/refloat/src/main.c:1904-1960`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RefloatRealtimeDataResponse {
    bytes: [u8; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY],
    len: usize,
}

impl RefloatRealtimeDataResponse {
    /// Return the encoded response bytes actually sent on the app-data wire.
    pub(super) fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }
}

/// Fixed-size Refloat app-data response bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RefloatAppDataResponse {
    /// Version/package-info response from `third_party/refloat/src/main.c:2070-2139`.
    InfoV2([u8; REFLOAT_INFO_RESPONSE_V2_LEN]),
    /// Legacy `COMMAND_GET_RTDATA` response from `third_party/refloat/src/main.c:1267-1310`.
    GetRealtimeData([u8; REFLOAT_GET_REALTIME_DATA_RESPONSE_LEN]),
    /// Realtime-data ID list response from `third_party/refloat/src/main.c:1876-1901`.
    RealtimeDataIds([u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN]),
    /// Realtime-data sample response from `third_party/refloat/src/main.c:1904-1960`.
    RealtimeData(RefloatRealtimeDataResponse),
    /// Compact all-data response from `third_party/refloat/src/main.c:1313-1399`.
    AllData(RefloatAllDataResponse),
}

impl RefloatAppDataResponse {
    /// Return the encoded response bytes.
    pub(super) fn as_bytes(&self) -> &[u8] {
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
/// `third_party/refloat/src/main.c:2143-2301`.
#[inline(never)]
pub(super) fn process_refloat_app_data(
    payloads: &RefloatAllDataPayloads,
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
    // `third_party/refloat/src/main.c:2070-2139`; QML allocates the four-byte request and sets
    // version 2 at `ui.qml.in:693-697`.
    let flags = match request_payload {
        [2, flags, ..] => *flags,
        _ => 0,
    };
    let mut bytes = [0; REFLOAT_INFO_RESPONSE_V2_LEN];
    let mut index = 0;
    refloat_response_push_u8(&mut bytes, &mut index, REFLOAT_APP_DATA_PACKAGE_ID.get());
    refloat_response_push_u8(&mut bytes, &mut index, RefloatAppDataCommand::Info.id());
    refloat_response_push_u8(&mut bytes, &mut index, 2);
    refloat_response_push_u8(&mut bytes, &mut index, flags);
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_PACKAGE_NAME);
    refloat_response_push_u8(&mut bytes, &mut index, 1);
    refloat_response_push_u8(&mut bytes, &mut index, 2);
    refloat_response_push_u8(&mut bytes, &mut index, 1);
    append_fixed_ascii::<20>(&mut bytes, &mut index, REFLOAT_VERSION_SUFFIX);
    refloat_response_push_bytes(&mut bytes, &mut index, &REFLOAT_GIT_HASH.to_be_bytes());
    refloat_response_push_bytes(
        &mut bytes,
        &mut index,
        &REFLOAT_SYSTEM_TICK_RATE_HZ.to_be_bytes(),
    );
    // Upstream derives capabilities from data-recorder and LED config at
    // `third_party/refloat/src/main.c:2121-2132`; this Rust runtime has not ported either
    // capability yet, so the honest advertised capability mask is zero.
    refloat_response_push_bytes(&mut bytes, &mut index, &0u32.to_be_bytes());
    // Upstream currently sends zero `extra_flags` at `third_party/refloat/src/main.c:2134-2135`.
    refloat_response_push_u8(&mut bytes, &mut index, 0);
    bytes
}

fn append_fixed_ascii<const LEN: usize>(bytes: &mut [u8], index: &mut usize, value: &[u8]) {
    let start = *index;
    for (offset, byte) in value.iter().copied().take(LEN).enumerate() {
        if let Some(slot) = bytes.get_mut(start.saturating_add(offset)) {
            *slot = byte;
        }
    }
    *index = start.saturating_add(LEN);
}

#[inline(never)]
fn encode_refloat_realtime_data_ids_response() -> [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN] {
    REFLOAT_REALTIME_DATA_IDS_RESPONSE_BYTES
}

// Same packet as `cmd_realtime_data_ids` in `third_party/refloat/src/main.c:1876-1901`, built as
// bytes so the ARM image does not rely on target string-literal addresses.
const fn build_refloat_realtime_data_ids_response() -> [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN]
{
    let mut bytes = [0; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN];
    let mut index = 0;

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 101);
    index = refloat_realtime_ids_push_u8(&mut bytes, index, 32);

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 16);
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.speed");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.erpm");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.dir_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.filt_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.duty_cycle");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.batt_voltage");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.batt_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.mosfet_temp");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"motor.motor_temp");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.pitch");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.balance_pitch");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"imu.roll");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"footpad.adc1");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"footpad.adc2");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"remote.input");

    index = refloat_realtime_ids_push_u8(&mut bytes, index, 10);
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"brake_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"torque_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"turn_tilt.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"remote.setpoint");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"balance_current");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.accel_diff");
    index = refloat_realtime_ids_push_id(&mut bytes, index, b"atr.speed_boost");
    let _index = refloat_realtime_ids_push_id(&mut bytes, index, b"booster.current");

    bytes
}

const fn refloat_realtime_ids_push_id<const N: usize>(
    bytes: &mut [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: &[u8; N],
) -> usize {
    let mut next = refloat_realtime_ids_push_u8(bytes, index, N as u8);
    let mut offset = 0;
    while offset < N {
        next = refloat_realtime_ids_push_u8(bytes, next, value[offset]);
        offset += 1;
    }
    next
}

const fn refloat_realtime_ids_push_u8(
    bytes: &mut [u8; REFLOAT_REALTIME_DATA_IDS_RESPONSE_LEN],
    index: usize,
    value: u8,
) -> usize {
    bytes[index] = value;
    index + 1
}

fn refloat_response_push_bytes(bytes: &mut [u8], index: &mut usize, values: &[u8]) {
    values
        .iter()
        .copied()
        .for_each(|byte| refloat_response_push_u8(bytes, index, byte));
}

fn refloat_response_push_u8(bytes: &mut [u8], index: &mut usize, value: u8) {
    if let Some(slot) = bytes.get_mut(*index) {
        *slot = value;
    }
    *index = index.saturating_add(1);
}

#[inline(never)]
fn encode_refloat_get_realtime_data_response(
    payloads: &RefloatAllDataPayloads,
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
    // `send_realtime_data` at `third_party/refloat/src/main.c:2162-2164`; `send_realtime_data`
    // writes this legacy 72-byte payload at `third_party/refloat/src/main.c:1267-1310`.
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
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc1_volts());
    refloat_realtime_push_float32_auto(&mut bytes, &mut ind, footpad.adc2_volts());

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
    // `d->motor.dir_current` at `third_party/refloat/src/main.c:1298-1306`. The current Rust
    // app-data state does not yet contain those separate runtime fields, so
    // this is explicitly a containment fallback until the shared `Data`
    // runtime is ported from `third_party/refloat/src/main.c:2419-2461`.
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

#[inline(never)]
pub(super) fn encode_refloat_realtime_data_response(
    payloads: &RefloatAllDataPayloads,
    system_timestamp: SystemTimestamp,
) -> RefloatRealtimeDataResponse {
    let mut bytes = [0; REFLOAT_REALTIME_DATA_RESPONSE_CAPACITY];
    let mut ind = 0;
    let base = payloads.base();
    let ride_state = base.status().ride_state();
    let running = matches!(ride_state.run_state(), RefloatRunState::Running);
    let charging = matches!(ride_state.charging(), RefloatChargingState::Charging);

    // Upstream `cmd_realtime_data` writes the realtime packet in
    // `third_party/refloat/src/main.c:1904-1960`; QML consumes it at `ui.qml.in:853-925`.
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
    // control-loop/runtime state (`third_party/refloat/src/main.c:1927-1930`, `third_party/refloat/src/main.c:1956-1958`).
    refloat_realtime_push_u8(&mut bytes, &mut ind, 0);
    // Upstream writes `d->time.now` at `third_party/refloat/src/main.c:1931`; VESC timestamps are
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

fn realtime_value(payloads: &RefloatAllDataPayloads, item: RefloatRealtimeDataItem) -> f32 {
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
        RefloatRealtimeDataItem::FootpadAdc1 => base.footpad().adc1_volts(),
        RefloatRealtimeDataItem::FootpadAdc2 => base.footpad().adc2_volts(),
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
    // `third_party/refloat/src/conf/buffer.c:143-145`, which writes `to_float16` big-endian.
    refloat_realtime_push_u16(buffer, ind, refloat_float16_auto_bits(value));
}

fn refloat_realtime_push_float32_auto(buffer: &mut [u8], ind: &mut usize, value: f32) {
    // Refloat forwards through `buffer_append_float32_auto` at
    // `third_party/refloat/src/conf/buffer.c:118-140`, zeroing denormal/subnormal values before
    // writing big-endian IEEE-754 bits.
    let value = if value.abs() < 1.5e-38 { 0.0 } else { value };
    refloat_realtime_push_u32(buffer, ind, value.to_bits());
}

fn refloat_float16_auto_bits(value: f32) -> u16 {
    // Refloat's `to_float16` is defined at `third_party/refloat/src/conf/buffer.c:33-43`.
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
    if let Some(slot) = buffer.get_mut(*ind) {
        *slot = value;
    }
    *ind = ind.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::super::test_support::sample_all_data_payloads;
    use super::process_refloat_app_data;
    use crate::domain::{
        REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataPayloads, RefloatAppDataCommand,
    };

    #[test]
    fn app_data_processes_all_data_requests_from_payload_snapshot() {
        let response = process_refloat_app_data(
            &sample_all_data_payloads(),
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
                &sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::GetAllData.id(),
                ]
            ),
            None
        );
        assert_eq!(
            process_refloat_app_data(
                &sample_all_data_payloads(),
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
            &sample_all_data_payloads(),
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
        // `cmd_info` replies with the v2 metadata layout at `third_party/refloat/src/main.c:2108-2135`.
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
        fn take_id_list<'a>(bytes: &'a [u8], index: &mut usize) -> std::vec::Vec<&'a str> {
            let count = bytes
                .get(*index)
                .copied()
                .map(usize::from)
                .expect("ID count byte");
            *index = index.saturating_add(1);

            (0..count)
                .map(|_| {
                    let len = bytes
                        .get(*index)
                        .copied()
                        .map(usize::from)
                        .expect("ID length byte");
                    *index = index.saturating_add(1);
                    let end = index.saturating_add(len);
                    let id = bytes.get(*index..end).expect("ID bytes");
                    *index = end;
                    core::str::from_utf8(id).expect("ID UTF-8")
                })
                .collect()
        }

        let response = process_refloat_app_data(
            &sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeDataIds.id(),
            ],
        )
        .expect("realtime data IDs request should produce a response");
        let bytes = response.as_bytes();

        // QML asks for IDs at `ui.qml.in:704-705`;
        // upstream `cmd_realtime_data_ids` writes the counted string sets at
        // `third_party/refloat/src/main.c:1876-1901`, using IDs from `third_party/refloat/src/rt_data.h:38-66`.
        assert_eq!(bytes.len(), 405);
        assert_eq!(bytes.get(..2), Some(&[101, 32][..]));
        let mut index = 2;
        assert_eq!(
            take_id_list(bytes, &mut index).as_slice(),
            &[
                "motor.speed",
                "motor.erpm",
                "motor.current",
                "motor.dir_current",
                "motor.filt_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "motor.batt_current",
                "motor.mosfet_temp",
                "motor.motor_temp",
                "imu.pitch",
                "imu.balance_pitch",
                "imu.roll",
                "footpad.adc1",
                "footpad.adc2",
                "remote.input",
            ]
        );
        assert_eq!(
            take_id_list(bytes, &mut index).as_slice(),
            &[
                "setpoint",
                "atr.setpoint",
                "brake_tilt.setpoint",
                "torque_tilt.setpoint",
                "turn_tilt.setpoint",
                "remote.setpoint",
                "balance_current",
                "atr.accel_diff",
                "atr.speed_boost",
                "booster.current",
            ]
        );
        assert_eq!(index, bytes.len());
    }

    #[test]
    fn app_data_processes_legacy_get_rtdata_like_refloat() {
        let response = process_refloat_app_data(
            &sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetRealtimeData.id(),
            ],
        )
        .expect("legacy realtime-data request should produce a response");
        let bytes = response.as_bytes();

        // Upstream dispatches `COMMAND_GET_RTDATA` at `third_party/refloat/src/main.c:2162-2164`;
        // `send_realtime_data` writes this 72-byte response at
        // `third_party/refloat/src/main.c:1267-1310`.
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
            &RefloatAllDataPayloads::source_startup(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        )
        .expect("realtime data request should produce a response");
        let bytes = response.as_bytes();

        // QML reads `c_REALTIME_DATA` at `ui.qml.in:853-925`; upstream
        // `cmd_realtime_data` writes this non-running packet shape at
        // `third_party/refloat/src/main.c:1904-1960`.
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
}
