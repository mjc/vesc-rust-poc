use super::super::test_support::{
    edit_config, sample_all_data_payloads_with_ride_state, tick_refloat_state_and_handle_packet,
};
use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataPayloads, RefloatAllDataStatus, RefloatDarkRideState,
    RefloatFootpadSample, RefloatFootpadState, RefloatMode, RefloatRunState, RefloatStopCondition,
    RefloatWheelSlipState,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn running_payloads(mode: RefloatMode) -> RefloatAllDataPayloads {
    sample_all_data_payloads_with_ride_state(RefloatRunState::Running, mode)
}

fn darkride_payloads(mode: RefloatMode) -> RefloatAllDataPayloads {
    let payloads = running_payloads(mode);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    )
}

fn darkride_no_footpads_payloads(mode: RefloatMode) -> RefloatAllDataPayloads {
    let payloads = darkride_payloads(mode);
    let base = payloads.base();
    let no_footpads = RefloatFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        RefloatFootpadState::None,
    );
    RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    )
}

#[test]
fn app_data_running_flywheel_both_footpads_stops_like_refloat_fault_check() {
    // C map: RUNNING FLYWHEEL with both footpads stops at
    // `third_party/refloat/src/main.c:492-495` and then writes READY through
    // `state_stop` at `third_party/refloat/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Flywheel,
    ));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_flywheel_stop_clears_wheelslip_like_refloat_state_stop() {
    // C map: the same `state_stop` write that makes the run state READY also
    // clears wheelslip at `third_party/refloat/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Flywheel);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(RefloatWheelSlipState::Detected);
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

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
    assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::None);
}

#[test]
fn app_data_running_darkride_footpads_stop_like_refloat_fault_check() {
    // C map: darkride still hits the shared `can_engage(d)` stop branch at
    // `third_party/refloat/src/main.c:387-390` and then writes READY through
    // `state_stop` at `third_party/refloat/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(darkride_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_timed_low_erpm_stops_like_refloat_fault_check() {
    // C map: darkride reverse-stops above 300 ERPM after 500ms at
    // `third_party/refloat/src/main.c:374-383`.
    let lifecycle = TimestampTicks::from_ticks(5_001);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(500.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(darkride_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_darkride_enabled_high_roll_stops_like_refloat_fault_check() {
    // C map: non-darkride `check_faults(d)` stops immediately when darkride
    // faults are enabled and roll is 100-135 degrees at
    // `third_party/refloat/src/main.c:465-470`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(110.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let payloads = running_payloads(RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = RefloatFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        RefloatFootpadState::None,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_darkride_enabled(true));
    });

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
}

#[test]
fn app_data_running_darkride_no_footpads_does_not_use_normal_full_switch_fault() {
    // C map: darkride still uses the dedicated fault branch, not the normal
    // full-switch path at `third_party/refloat/src/main.c:392-425`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(darkride_no_footpads_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_running_roll_stopped_after_delay_like_refloat_fault_check() {
    // C map: roll stops above `fault_roll` after `fault_delay_roll` at
    // `third_party/refloat/src/main.c:474-482`.
    let lifecycle = TimestampTicks::from_ticks(3_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(70.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(running_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
    assert!(!state.apply_requested_motor_current(telemetry.motor()));
    assert_eq!(telemetry.current_command_count(), 0);
}

#[test]
fn app_data_running_pitch_stopped_after_delay_like_refloat_fault_check() {
    // C map: pitch stops above `fault_pitch` after `fault_delay_pitch` when
    // remote setpoint is below 30 degrees at
    // `third_party/refloat/src/main.c:497-503`.
    let lifecycle = TimestampTicks::from_ticks(3_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_degrees(70.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(running_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Pitch);
}

#[test]
fn app_data_running_darkride_simple_start_single_footpad_stops_during_engage_grace() {
    // C map: darkride's `can_engage(d)` branch stops immediately when a single
    // footpad becomes eligible to engage again at
    // `third_party/refloat/src/main.c:387-390`.
    let lifecycle = TimestampTicks::from_ticks(5_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let single_footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        RefloatFootpadState::Left,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_simplestart_enabled(true));
    });

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_high_erpm_stops_like_refloat_fault_check() {
    // C map: darkride high-ERPM stops through the reverse-stop branch at
    // `third_party/refloat/src/main.c:361-380` before `state_stop` writes READY.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2100.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(darkride_payloads(RefloatMode::Normal));

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
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}
