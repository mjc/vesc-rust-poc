use super::super::test_support::{
    edit_config, sample_all_data_payloads_with_ride_state,
    tick_float_out_boy_state_and_handle_packet,
};
use super::FloatOutBoyPackageState;
use crate::domain::{
    FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads,
    FloatOutBoyAllDataStatus, FloatOutBoyDarkRideState, FloatOutBoyMode,
    FloatOutBoyRealtimeBalancePitch, FloatOutBoyRunState, FloatOutBoyStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn ready_darkride_payloads() -> FloatOutBoyAllDataPayloads {
    // C map: darkride READY fixtures keep the same startup payload shape
    // while forcing `FloatOutBoyDarkRideState::Active` for the READY branch in
    // `third_party/float-out-boy/src/main.c:1038-1054`.
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(FloatOutBoyDarkRideState::Active);
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::ZERO),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn configure_ready_darkride_imu(firmware: &FirmwareTest) {
    // C map: Float Out Boy's darkride READY gate wants a near-upside-down roll
    // sample while pitch and yaw stay neutral.
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(170.0)),
        ImuPitch::new(AngleRadians::ZERO),
        ImuYaw::new(AngleRadians::ZERO),
    );
}

#[test]
fn app_data_ready_darkride_first_second_engages_without_roll_gate_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(5_000);
    let telemetry = FirmwareTest::new();
    configure_ready_darkride_imu(&telemetry);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_darkride_payloads());

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY darkride ignores roll during the first second after
    // disengage at `third_party/float-out-boy/src/main.c:1038-1054`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::None);
}

#[test]
fn app_data_ready_darkride_after_grace_engages_with_upside_down_roll_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(10_001);
    let telemetry = FirmwareTest::new();
    configure_ready_darkride_imu(&telemetry);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_darkride_payloads());

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY darkride engages after one second when roll is near
    // upside-down at `third_party/float-out-boy/src/main.c:1038-1054`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::None);
}

#[test]
fn ready_darkride_disables_and_alerts_after_ten_seconds_like_float_out_boy() {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(90.0)),
        ImuPitch::new(AngleRadians::ZERO),
        ImuYaw::new(AngleRadians::ZERO),
    );
    let imu = telemetry.imu();

    for (ticks, expected_darkride, expected_first_high) in [
        (100_000, FloatOutBoyDarkRideState::Active, None),
        (100_001, FloatOutBoyDarkRideState::Upright, Some(600)),
    ] {
        let mut state = FloatOutBoyPackageState::new(ready_darkride_payloads());
        assert!(state.serialized_config.editor().set_beeper_enabled(true));
        state.refresh_config_runtime_state();

        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            TimestampTicks::from_ticks(ticks),
            telemetry.telemetry(),
            imu,
            &[
                crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
        assert_eq!(ride_state.darkride(), expected_darkride);
        assert_eq!(
            (1..=600).find(|_| matches!(
                state.tick_beeper(),
                Some(crate::beeper::FloatOutBoyBeeperLevel::High)
            )),
            expected_first_high,
        );
    }
}

#[test]
fn ready_darkride_timeout_prevents_same_tick_reactivation_like_float_out_boy() {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(151.0)),
        ImuPitch::new(AngleRadians::ZERO),
        ImuYaw::new(AngleRadians::ZERO),
    );
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_darkride_enabled(true));
    });
    state.upside_down_flags.enabled = true;

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(100_001),
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(ride_state.darkride(), FloatOutBoyDarkRideState::Upright);
}
