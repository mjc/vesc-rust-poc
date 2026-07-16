use super::refloat_command_payload;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataMode4Payload, RefloatAllDataPayloads,
    RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::package::time::refloat_ticks_elapsed;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::TimestampTicks;
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};
use vescpkg_rs::protocol_buffer::get_float16;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct ChargingScalar(f32);

impl ChargingScalar {
    #[inline(never)]
    fn from_wire(hi: u8, lo: u8) -> Self {
        let mut index = 0;
        Self(
            get_float16(&[hi, lo], 10.0, &mut index)
                .expect("two bytes always contain one VESC float16"),
        )
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

#[cfg(any(test, target_arch = "arm"))]
pub(super) fn timeout(
    payloads: RefloatAllDataPayloads,
    now: TimestampTicks,
    last_update: TimestampTicks,
) -> RefloatAllDataPayloads {
    let base = payloads.base();
    let status = base.status();
    let ride_state = status.ride_state();
    if !matches!(ride_state.charging(), RefloatChargingState::Charging)
        || !refloat_ticks_elapsed(now, last_update, 5)
    {
        return payloads;
    }

    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(
            ride_state.with_charging(RefloatChargingState::NotCharging),
            status.beep_reason(),
        ),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{REFLOAT_APP_DATA_PACKAGE_ID, RefloatMode, RefloatRunState};
    use crate::package::test_support::{
        sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
    };
    use vescpkg_rs::prelude::{AngleRadians, ImuPitch, ImuRoll, ImuYaw, TimestampTicks};
    use vescpkg_rs::test_support::FirmwareTest;

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

    #[test]
    fn charging_packet_rejects_short_inactive_measurements_like_refloat() {
        assert!(
            handle_packet(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::ChargingState.id(),
                    151,
                    0,
                ],
            )
            .is_none()
        );
    }

    #[test]
    fn charging_times_out_after_five_seconds_and_allows_ready_to_engage() {
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_startup_done(true);
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::ZERO),
            ImuPitch::new(AngleRadians::ZERO),
            ImuYaw::new(AngleRadians::ZERO),
        );
        let mut state = crate::package::RefloatPackageState::new(
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal),
        );
        let packet = [
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::ChargingState.id(),
            151,
            1,
            1,
            244,
            0,
            123,
        ];
        let mut now = || TimestampTicks::from_ticks(10_000);
        let mut discard = |_bytes: &[u8]| true;
        assert!(state.handle_packet_with_telemetry(
            telemetry.telemetry(),
            &mut now,
            &mut discard,
            &packet,
        ));

        state.refresh_main_loop_runtime_state(
            telemetry.telemetry(),
            telemetry.imu(),
            Voltage::from_volts(2.5),
            Voltage::from_volts(2.5),
            60_000,
        );
        let ride_state = state.all_data_payloads().base().status().ride_state();
        assert_eq!(ride_state.charging(), RefloatChargingState::Charging);
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);

        state.refresh_main_loop_runtime_state(
            telemetry.telemetry(),
            telemetry.imu(),
            Voltage::from_volts(2.5),
            Voltage::from_volts(2.5),
            60_001,
        );
        let ride_state = state.all_data_payloads().base().status().ride_state();
        assert_eq!(ride_state.charging(), RefloatChargingState::NotCharging);
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    }
}
