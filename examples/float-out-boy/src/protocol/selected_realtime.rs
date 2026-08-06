//! Float Out Boy cutoff command 33 mask-selected realtime encoding.

use super::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_DATA_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
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
    for (_, item) in (6..14)
        .map(|shift| 1_u32.wrapping_shl(shift))
        .zip(FLOAT_OUT_BOY_REALTIME_DATA_ITEMS[2..10].iter().copied())
        .filter(|(bit, _)| mask.selects(*bit))
    {
        precision.push(packet, realtime_value(payloads, item, live));
    }
    if mask.selects(MASK1_BATTERY_SOC) {
        precision.push(packet, payloads.battery_level().as_fraction());
    }
    let after_soc = FLOAT_OUT_BOY_REALTIME_DATA_ITEMS[10..]
        .iter()
        .chain(FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS[..7].iter())
        .copied()
        .chain([FloatOutBoyRealtimeDataItem::ControlFrequency]);
    let bits = (15..31).map(|shift| 1_u32.wrapping_shl(shift));
    for (_, item) in bits.zip(after_soc).filter(|(bit, _)| mask.selects(*bit)) {
        precision.push(packet, realtime_value(payloads, item, live));
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
        packet.push_u32(super::truncating_u64_to_u32(
            payloads.odometer().as_meters(),
        ));
    }
    let bits = (1..9).map(|shift| 1_u32.wrapping_shl(shift));
    for (_, value) in bits
        .zip([
            payloads.distance_abs().distance().as_meters(),
            payloads.charging_voltage().voltage().as_volts(),
            payloads.charging_current().current().as_amps(),
            payloads.discharged_charge().charge().as_amp_hours(),
            payloads.charged_charge().charge().as_amp_hours(),
            payloads.discharged_energy().energy().as_watt_hours(),
            payloads.charged_energy().energy().as_watt_hours(),
            payloads
                .foc_id_current()
                .map_or(0.0, |current| current.current().as_amps()),
        ])
        .filter(|(bit, _)| mask.selects(*bit))
    {
        precision.push(packet, value);
    }
    let Some(gnss) = gnss else { return };
    if mask.selects(MASK2_GNSS_LAT) {
        packet.extend(&gnss.latitude().latitude().as_degrees().to_be_bytes());
    }
    if mask.selects(MASK2_GNSS_LON) {
        packet.extend(&gnss.longitude().longitude().as_degrees().to_be_bytes());
    }
    let bits = (11..14).map(|shift| 1_u32.wrapping_shl(shift));
    for (_, value) in bits
        .zip([
            gnss.altitude().altitude().as_meters(),
            gnss.speed().speed().as_kilometers_per_hour(),
            gnss.hdop().as_unitless(),
        ])
        .filter(|(bit, _)| mask.selects(*bit))
    {
        precision.push(packet, value);
    }
    if mask.selects(MASK2_GNSS_LAST_UPDATE) {
        packet.push_u32(gnss.last_update().as_ticks());
    }
}
