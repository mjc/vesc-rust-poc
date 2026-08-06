use super::*;
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyMode, FloatOutBoyRunState};
use crate::package::test_support::{
    sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::prelude::{AdcVoltage, AngleRadians, ImuPitch, ImuRoll, ImuYaw, TimestampTicks};
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn charging_state_command_updates_status_and_mode4_payload_like_float_out_boy() {
    let payloads = handle_packet(
        sample_all_data_payloads(),
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
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
    assert_f32_eq!(payloads.mode4().current().current().as_amps(), 12.3);
    assert_f32_eq!(payloads.mode4().voltage().voltage().as_volts(), 50.0);
}

#[test]
fn charging_packet_preserves_signed_current_and_zeroes_inactive_measurements() {
    let packet = |charging, current: [u8; 2]| {
        handle_packet(
            sample_all_data_payloads(),
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
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
    assert_f32_eq!(charging.mode4().current().current().as_amps(), -12.3);

    let inactive = packet(0, 123_i16.to_be_bytes());
    assert_f32_eq!(inactive.mode4().current().current().as_amps(), 0.0);
    assert_f32_eq!(inactive.mode4().voltage().voltage().as_volts(), 0.0);
}

#[test]
fn charging_packet_rejects_short_inactive_measurements_like_float_out_boy() {
    assert!(
        handle_packet(
            sample_all_data_payloads(),
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
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
    let ride_state = state.all_data_payloads().ride_state();
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
    let ride_state = state.all_data_payloads().ride_state();
    assert_eq!(ride_state.charging(), FloatOutBoyChargingState::NotCharging);
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
}
