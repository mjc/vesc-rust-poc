//! Cutoff command 33 mask-selected realtime encoding.

use crate::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyPacket, FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeLiveValues, FloatOutBoyRealtimeMask1, FloatOutBoyRealtimeMask2,
    FloatOutBoyRealtimePrecision, FloatOutBoyRealtimeSelectedRequest, realtime_value,
};
use vescpkg_rs::GnssSnapshot;

/// Exact maximum size of the cutoff's currently selectable response fields.
pub const FLOAT_OUT_BOY_REALTIME_SELECTED_RESPONSE_CAPACITY: usize = 188;

/// Fixed-capacity command 33 response.
pub type FloatOutBoyRealtimeSelectedResponse =
    FloatOutBoyPacket<FLOAT_OUT_BOY_REALTIME_SELECTED_RESPONSE_CAPACITY>;

const MASK1_EXTRA_FLAGS: u32 = 1 << 0;
const MASK1_STATE_FLAGS: u32 = 1 << 1;
const MASK1_BATTERY_SOC: u32 = 1 << 14;
const MASK2_ODOMETER: u32 = 1 << 0;
const MASK2_GNSS_LAT: u32 = 1 << 9;
const MASK2_GNSS_LON: u32 = 1 << 10;
const MASK2_GNSS_LAST_UPDATE: u32 = 1 << 14;

const MASK1_ITEMS_BEFORE_SOC: [(u32, FloatOutBoyRealtimeDataItem); 8] = [
    (1 << 6, FloatOutBoyRealtimeDataItem::MotorSpeed),
    (1 << 7, FloatOutBoyRealtimeDataItem::MotorErpm),
    (1 << 8, FloatOutBoyRealtimeDataItem::MotorCurrent),
    (1 << 9, FloatOutBoyRealtimeDataItem::MotorDirectionalCurrent),
    (1 << 10, FloatOutBoyRealtimeDataItem::MotorFilteredCurrent),
    (1 << 11, FloatOutBoyRealtimeDataItem::MotorDutyCycle),
    (1 << 12, FloatOutBoyRealtimeDataItem::MotorBatteryVoltage),
    (1 << 13, FloatOutBoyRealtimeDataItem::MotorBatteryCurrent),
];

const MASK1_ITEMS_AFTER_SOC: [(u32, FloatOutBoyRealtimeDataItem); 16] = [
    (1 << 15, FloatOutBoyRealtimeDataItem::MotorMosfetTemperature),
    (1 << 16, FloatOutBoyRealtimeDataItem::MotorTemperature),
    (1 << 17, FloatOutBoyRealtimeDataItem::ImuPitch),
    (1 << 18, FloatOutBoyRealtimeDataItem::ImuBalancePitch),
    (1 << 19, FloatOutBoyRealtimeDataItem::ImuRoll),
    (1 << 20, FloatOutBoyRealtimeDataItem::FootpadAdc1),
    (1 << 21, FloatOutBoyRealtimeDataItem::FootpadAdc2),
    (1 << 22, FloatOutBoyRealtimeDataItem::RemoteInput),
    (1 << 23, FloatOutBoyRealtimeDataItem::Setpoint),
    (1 << 24, FloatOutBoyRealtimeDataItem::AtrSetpoint),
    (1 << 25, FloatOutBoyRealtimeDataItem::BrakeTiltSetpoint),
    (1 << 26, FloatOutBoyRealtimeDataItem::TorqueTiltSetpoint),
    (1 << 27, FloatOutBoyRealtimeDataItem::TurnTiltSetpoint),
    (1 << 28, FloatOutBoyRealtimeDataItem::RemoteSetpoint),
    (1 << 29, FloatOutBoyRealtimeDataItem::BalanceCurrent),
    (1 << 30, FloatOutBoyRealtimeDataItem::ControlFrequency),
];

/// Encode one cutoff command 33 response.
#[must_use]
pub fn encode_float_out_boy_realtime_selected_response(
    request: FloatOutBoyRealtimeSelectedRequest,
    payloads: &FloatOutBoyAllDataPayloads,
    header: FloatOutBoyRealtimeDataHeader,
    live: FloatOutBoyRealtimeLiveValues,
    gnss: Option<GnssSnapshot>,
) -> FloatOutBoyRealtimeSelectedResponse {
    let mut packet = FloatOutBoyPacket::new();
    let flags = request.control_flags();
    packet.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
    packet.push(FloatOutBoyAppDataCommand::RealtimeDataSelected.id());
    packet.push(flags.wire_value());
    packet.push_u32(request.mask1().wire_value());
    packet.push_u32(request.mask2().wire_value());
    packet.push_u32(header.timestamp().as_ticks());
    append_mask1(
        &mut packet,
        request.mask1(),
        flags.precision(),
        payloads,
        header,
        live,
    );
    append_mask2(
        &mut packet,
        request.mask2(),
        flags.precision(),
        payloads,
        gnss,
    );
    packet
}

fn append_mask1(
    packet: &mut FloatOutBoyRealtimeSelectedResponse,
    mask: FloatOutBoyRealtimeMask1,
    precision: FloatOutBoyRealtimePrecision,
    payloads: &FloatOutBoyAllDataPayloads,
    header: FloatOutBoyRealtimeDataHeader,
    live: FloatOutBoyRealtimeLiveValues,
) {
    if mask.selects(MASK1_EXTRA_FLAGS) {
        packet.push(header.extra_flags_compat());
    }
    if mask.selects(MASK1_STATE_FLAGS) {
        packet.push_u32(header.state_flags_compat());
    }
    for (bit, item) in MASK1_ITEMS_BEFORE_SOC {
        push_selected(
            packet,
            mask.selects(bit),
            precision,
            realtime_value(payloads, item, live),
        );
    }
    push_selected(
        packet,
        mask.selects(MASK1_BATTERY_SOC),
        precision,
        payloads.mode3().battery_level().as_fraction(),
    );
    for (bit, item) in MASK1_ITEMS_AFTER_SOC {
        push_selected(
            packet,
            mask.selects(bit),
            precision,
            realtime_value(payloads, item, live),
        );
    }
}

fn append_mask2(
    packet: &mut FloatOutBoyRealtimeSelectedResponse,
    mask: FloatOutBoyRealtimeMask2,
    precision: FloatOutBoyRealtimePrecision,
    payloads: &FloatOutBoyAllDataPayloads,
    gnss: Option<GnssSnapshot>,
) {
    if mask.selects(MASK2_ODOMETER) {
        packet.push_u32(crate::truncating_u64_to_u32(
            payloads.mode3().odometer().as_meters(),
        ));
    }
    let mode2 = payloads.mode2();
    let mode3 = payloads.mode3();
    let mode4 = payloads.mode4();
    for (bit, value) in [
        (1 << 1, mode2.distance_abs().distance().as_meters()),
        (1 << 2, mode4.voltage().voltage().as_volts()),
        (1 << 3, mode4.current().current().as_amps()),
        (1 << 4, mode3.discharged_charge().charge().as_amp_hours()),
        (1 << 5, mode3.charged_charge().charge().as_amp_hours()),
        (1 << 6, mode3.discharged_energy().energy().as_watt_hours()),
        (1 << 7, mode3.charged_energy().energy().as_watt_hours()),
        (
            1 << 8,
            payloads
                .base()
                .motor()
                .foc_id_current()
                .map_or(0.0, |current| current.current().as_amps()),
        ),
    ] {
        push_selected(packet, mask.selects(bit), precision, value);
    }
    let Some(gnss) = gnss else { return };
    if mask.selects(MASK2_GNSS_LAT) {
        packet.extend(&gnss.latitude().latitude().as_degrees().to_be_bytes());
    }
    if mask.selects(MASK2_GNSS_LON) {
        packet.extend(&gnss.longitude().longitude().as_degrees().to_be_bytes());
    }
    for (bit, value) in [
        (1 << 11, gnss.altitude().altitude().as_meters()),
        (1 << 12, gnss.speed().speed().as_kilometers_per_hour()),
        (1 << 13, gnss.hdop().as_unitless()),
    ] {
        push_selected(packet, mask.selects(bit), precision, value);
    }
    if mask.selects(MASK2_GNSS_LAST_UPDATE) {
        packet.push_u32(gnss.last_update().as_ticks());
    }
}

fn push_selected(
    packet: &mut FloatOutBoyRealtimeSelectedResponse,
    selected: bool,
    precision: FloatOutBoyRealtimePrecision,
    value: f32,
) {
    if !selected {
        return;
    }
    match precision {
        FloatOutBoyRealtimePrecision::Float16 => packet.push_float16_auto(value),
        FloatOutBoyRealtimePrecision::Float32 => packet.push_float32_auto(value),
    }
}
