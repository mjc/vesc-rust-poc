use super::super::test_support::{
    sample_all_data_payloads_with_ride_state, tick_refloat_state_and_handle_packet,
};
use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataPayloads,
    RefloatAllDataStatus, RefloatDarkRideState, RefloatMode, RefloatRealtimeBalancePitch,
    RefloatRunState, RefloatStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn ready_darkride_payloads() -> RefloatAllDataPayloads {
    // C map: darkride READY fixtures keep the same startup payload shape
    // while forcing `RefloatDarkRideState::Active` for the READY branch in
    // `third_party/refloat/src/main.c:1038-1054`.
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::ZERO),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn configure_ready_darkride_imu(firmware: &FirmwareTest) {
    // C map: Refloat's darkride READY gate wants a near-upside-down roll
    // sample while pitch and yaw stay neutral.
    firmware.set_imu_startup_done(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(170.0)),
        ImuPitch::new(AngleRadians::ZERO),
        ImuYaw::new(AngleRadians::ZERO),
    );
}

#[test]
fn app_data_ready_darkride_first_second_engages_without_roll_gate_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(5_000);
    let telemetry = FirmwareTest::new();
    configure_ready_darkride_imu(&telemetry);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_darkride_payloads());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            crate::domain::REFLOAT_APP_DATA_PACKAGE_ID.get(),
            crate::domain::RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY darkride ignores roll during the first second after
    // disengage at `third_party/refloat/src/main.c:1038-1054`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_darkride_after_grace_engages_with_upside_down_roll_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(10_001);
    let telemetry = FirmwareTest::new();
    configure_ready_darkride_imu(&telemetry);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_darkride_payloads());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            crate::domain::REFLOAT_APP_DATA_PACKAGE_ID.get(),
            crate::domain::RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY darkride engages after one second when roll is near
    // upside-down at `third_party/refloat/src/main.c:1038-1054`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}
