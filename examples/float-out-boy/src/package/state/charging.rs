use super::float_out_boy_command_payload;
use crate::domain::{
    FloatOutBoyAllDataMode4Payload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyAppDataCommand, FloatOutBoyChargingState,
};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::TimestampTicks;
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::timer_older_whole_seconds as float_out_boy_ticks_elapsed;

const CHARGING_WIRE_SCALE: f32 = 10.0;

fn decode_charging_voltage(hi: u8, lo: u8) -> BatteryVoltage {
    BatteryVoltage::new(Voltage::from_volts(
        f32::from(i16::from_be_bytes([hi, lo])) / CHARGING_WIRE_SCALE,
    ))
}

fn decode_charging_current(hi: u8, lo: u8) -> BatteryCurrent {
    BatteryCurrent::new(Current::from_amps(
        f32::from(i16::from_be_bytes([hi, lo])) / CHARGING_WIRE_SCALE,
    ))
}

pub(super) fn handle_packet(
    payloads: FloatOutBoyAllDataPayloads,
    bytes: &[u8],
) -> Option<FloatOutBoyAllDataPayloads> {
    // Float Out Boy v1.2.1 routes COMMAND_CHARGING_STATE at `third_party/float-out-boy/src/main.c:2267-2269`;
    // the command ID is defined in `third_party/float-out-boy/src/charging.h:25`.
    let [
        151,
        charging,
        voltage_hi,
        voltage_lo,
        current_hi,
        current_lo,
        ..,
    ] = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::ChargingState)?
    else {
        return None;
    };

    // C map: `charging_state_request` expects magic 151 plus signed float16
    // voltage/current with scale 10 at `third_party/float-out-boy/src/charging.c:37-63`.
    let (voltage, current) = match *charging {
        0 => (
            BatteryVoltage::new(Voltage::ZERO),
            BatteryCurrent::new(Current::ZERO),
        ),
        _ => (
            decode_charging_voltage(*voltage_hi, *voltage_lo),
            decode_charging_current(*current_hi, *current_lo),
        ),
    };

    let base = payloads.base();
    let status = base.status();
    // C map: the same packet writes `state->charging` before storing
    // voltage/current at `third_party/float-out-boy/src/charging.c:53-63`.
    let ride_state = status.ride_state().with_charging(match *charging {
        // C map: `charging_state_request` writes `state->charging` from the
        // packet byte at `third_party/float-out-boy/src/charging.c:37-63`.
        0 => FloatOutBoyChargingState::NotCharging,
        _ => FloatOutBoyChargingState::Charging,
    });
    Some(
        payloads
            .with_base(base.with_status(FloatOutBoyAllDataStatus::new(
                ride_state,
                status.beep_reason(),
            )))
            .with_mode4_charging(FloatOutBoyAllDataMode4Payload::new(current, voltage)),
    )
}

#[cfg(any(test, target_arch = "arm"))]
pub(super) fn timeout(
    payloads: FloatOutBoyAllDataPayloads,
    now: TimestampTicks,
    last_update: TimestampTicks,
) -> FloatOutBoyAllDataPayloads {
    let base = payloads.base();
    let status = base.status();
    let ride_state = status.ride_state();
    if !matches!(ride_state.charging(), FloatOutBoyChargingState::Charging)
        || !float_out_boy_ticks_elapsed(now, last_update, 5)
    {
        return payloads;
    }

    payloads.with_base(base.with_status(FloatOutBoyAllDataStatus::new(
        ride_state.with_charging(FloatOutBoyChargingState::NotCharging),
        status.beep_reason(),
    )))
}

#[cfg(test)]
mod tests;
