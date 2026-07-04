use super::refloat_command_payload;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataMode4Payload, RefloatAllDataPayloads,
    RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
};
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct ChargingScalar(f32);

impl ChargingScalar {
    #[inline(never)]
    fn from_wire(hi: u8, lo: u8) -> Self {
        Self(f32::from(i16::from_be_bytes([hi, lo])) / 10.0)
    }

    const fn voltage(self) -> BatteryVoltage {
        BatteryVoltage::new(Voltage::from_volts(self.0))
    }

    const fn current(self) -> BatteryCurrent {
        BatteryCurrent::new(Current::from_amps(self.0))
    }
}

pub(super) fn handle_packet(
    payloads: RefloatAllDataPayloads,
    bytes: &[u8],
) -> Option<RefloatAllDataPayloads> {
    // Refloat v1.2.1 routes COMMAND_CHARGING_STATE at `third_party/refloat/src/main.c:2267-2269`;
    // the command ID is defined in `third_party/refloat/src/charging.h:25`.
    let [
        151,
        charging,
        voltage_hi,
        voltage_lo,
        current_hi,
        current_lo,
        ..,
    ] = refloat_command_payload(bytes, RefloatAppDataCommand::ChargingState)?
    else {
        return None;
    };

    // C map: `charging_state_request` expects magic 151 plus signed float16
    // voltage/current with scale 10 at `third_party/refloat/src/charging.c:37-63`.
    let scaled_i16 = |hi, lo| ChargingScalar::from_wire(hi, lo);
    let (voltage, current) = match *charging {
        0 => (
            BatteryVoltage::new(Voltage::ZERO),
            BatteryCurrent::new(Current::ZERO),
        ),
        _ => (
            scaled_i16(*voltage_hi, *voltage_lo).voltage(),
            scaled_i16(*current_hi, *current_lo).current(),
        ),
    };

    let base = payloads.base();
    let status = base.status();
    // C map: the same packet writes `state->charging` before storing
    // voltage/current at `third_party/refloat/src/charging.c:53-63`.
    let ride_state = status.ride_state().with_charging(match *charging {
        // C map: `charging_state_request` writes `state->charging` from the
        // packet byte at `third_party/refloat/src/charging.c:37-63`.
        0 => RefloatChargingState::NotCharging,
        _ => RefloatChargingState::Charging,
    });
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, status.beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );

    Some(
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
            .with_mode4_charging(RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(current),
                RefloatRealtimeChargingVoltage::new(voltage),
            )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::REFLOAT_APP_DATA_PACKAGE_ID;
    use crate::package::test_support::sample_all_data_payloads;

    #[test]
    fn charging_state_command_updates_status_and_mode4_payload_like_refloat() {
        let payloads = handle_packet(
            sample_all_data_payloads(),
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
        )
        .expect("charging state packet should decode");

        assert_eq!(
            payloads.base().status().ride_state().charging(),
            RefloatChargingState::Charging
        );
        assert_eq!(
            payloads.mode4().current().current().current().as_amps(),
            12.3
        );
        assert_eq!(
            payloads.mode4().voltage().voltage().voltage().as_volts(),
            50.0
        );
    }

    #[test]
    fn charging_packet_preserves_signed_current_and_zeroes_inactive_measurements() {
        let packet = |charging, current: [u8; 2]| {
            handle_packet(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::ChargingState.id(),
                    151,
                    charging,
                    1,
                    244,
                    current[0],
                    current[1],
                ],
            )
            .expect("charging state packet should decode")
        };

        let charging = packet(1, (-123_i16).to_be_bytes());
        assert_eq!(
            charging.mode4().current().current().current().as_amps(),
            -12.3
        );

        let inactive = packet(0, 123_i16.to_be_bytes());
        assert_eq!(
            inactive.mode4().current().current().current().as_amps(),
            0.0
        );
        assert_eq!(
            inactive.mode4().voltage().voltage().voltage().as_volts(),
            0.0
        );
    }
}
