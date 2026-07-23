use super::float_out_boy_command_payload;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMode4Payload, FloatOutBoyAllDataPayloads,
    FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand, FloatOutBoyChargingState,
    FloatOutBoyRealtimeChargingCurrent, FloatOutBoyRealtimeChargingVoltage,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::package::time::float_out_boy_ticks_elapsed;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::TimestampTicks;
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};

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
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, status.beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );

    Some(
        FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
            .with_mode4_charging(FloatOutBoyAllDataMode4Payload::new(
                FloatOutBoyRealtimeChargingCurrent::new(current),
                FloatOutBoyRealtimeChargingVoltage::new(voltage),
            )),
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

    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(
            ride_state.with_charging(FloatOutBoyChargingState::NotCharging),
            status.beep_reason(),
        ),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyMode, FloatOutBoyRunState};
    use crate::package::test_support::{
        sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
    };
    use vescpkg_rs::prelude::{
        AdcVoltage, AngleRadians, ImuPitch, ImuRoll, ImuYaw, TimestampTicks,
    };
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn charging_state_command_updates_status_and_mode4_payload_like_float_out_boy() {
        let payloads = handle_packet(
            sample_all_data_payloads(),
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                FloatOutBoyAppDataCommand::ChargingState.id(),
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
            FloatOutBoyChargingState::Charging
        );
        assert_f32_eq!(
            payloads.mode4().current().current().current().as_amps(),
            12.3
        );
        assert_f32_eq!(
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
                    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                    FloatOutBoyAppDataCommand::ChargingState.id(),
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
        assert_f32_eq!(
            charging.mode4().current().current().current().as_amps(),
            -12.3
        );

        let inactive = packet(0, 123_i16.to_be_bytes());
        assert_f32_eq!(
            inactive.mode4().current().current().current().as_amps(),
            0.0
        );
        assert_f32_eq!(
            inactive.mode4().voltage().voltage().voltage().as_volts(),
            0.0
        );
    }

    #[test]
    fn charging_packet_rejects_short_inactive_measurements_like_float_out_boy() {
        assert!(
            handle_packet(
                sample_all_data_payloads(),
                &[
                    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                    FloatOutBoyAppDataCommand::ChargingState.id(),
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
        telemetry.set_imu_ready(true);
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::ZERO),
            ImuPitch::new(AngleRadians::ZERO),
            ImuYaw::new(AngleRadians::ZERO),
        );
        let mut state =
            crate::package::FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
                FloatOutBoyRunState::Ready,
                FloatOutBoyMode::Normal,
            ));
        let packet = [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::ChargingState.id(),
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
            telemetry.motor(),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            TimestampTicks::from_ticks(60_000),
        );
        let ride_state = state.all_data_payloads().base().status().ride_state();
        assert_eq!(ride_state.charging(), FloatOutBoyChargingState::Charging);
        assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);

        state.refresh_main_loop_runtime_state(
            telemetry.telemetry(),
            telemetry.imu(),
            telemetry.motor(),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            TimestampTicks::from_ticks(60_001),
        );
        let ride_state = state.all_data_payloads().base().status().ride_state();
        assert_eq!(ride_state.charging(), FloatOutBoyChargingState::NotCharging);
        assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    }
}
