use super::super::test_support::{
    edit_config, sample_all_data_payloads_with_ride_state,
    tick_float_out_boy_state_and_handle_packet,
};
use super::FloatOutBoyPackageState;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyDarkRideState, FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRunState, FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn running_payloads(mode: FloatOutBoyMode) -> FloatOutBoyAllDataPayloads {
    sample_all_data_payloads_with_ride_state(FloatOutBoyRunState::Running, mode)
}

fn darkride_payloads(mode: FloatOutBoyMode) -> FloatOutBoyAllDataPayloads {
    let payloads = running_payloads(mode);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(FloatOutBoyDarkRideState::Active);
    FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
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

fn darkride_no_footpads_payloads(mode: FloatOutBoyMode) -> FloatOutBoyAllDataPayloads {
    let payloads = darkride_payloads(mode);
    let base = payloads.base();
    let no_footpads = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::None,
    );
    FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
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

fn upright_no_footpads_payloads() -> FloatOutBoyAllDataPayloads {
    let payloads = running_payloads(FloatOutBoyMode::Normal);
    let base = payloads.base();
    let no_footpads =
        FloatOutBoyFootpadSample::new(Voltage::ZERO, Voltage::ZERO, FloatOutBoyFootpadState::None);
    FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
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
fn running_darkride_activates_and_clears_with_float_out_boy_roll_hysteresis() {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(upright_no_footpads_payloads());
    edit_config(&mut state, |config| {
        assert!(config.set_darkride_enabled(true));
    });
    let packet = [
        crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
    ];

    for (ticks, roll, expected) in [
        (0, 0.0, FloatOutBoyDarkRideState::Upright),
        (1, 150.0, FloatOutBoyDarkRideState::Upright),
        (2, 151.0, FloatOutBoyDarkRideState::Active),
        (3, 120.0, FloatOutBoyDarkRideState::Active),
        (4, 119.0, FloatOutBoyDarkRideState::Upright),
    ] {
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::from_degrees(roll)),
            ImuPitch::new(AngleRadians::ZERO),
            ImuYaw::new(AngleRadians::ZERO),
        );
        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            TimestampTicks::from_ticks(ticks),
            telemetry.telemetry(),
            imu,
            &packet,
        ));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .darkride(),
            expected,
            "roll={roll}"
        );
    }
}

#[test]
fn running_darkride_stays_upright_when_feature_is_disabled() {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(upright_no_footpads_payloads());
    let packet = [
        crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
    ];

    for (ticks, roll) in [(0, 0.0), (1, 151.0)] {
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::from_degrees(roll)),
            ImuPitch::new(AngleRadians::ZERO),
            ImuYaw::new(AngleRadians::ZERO),
        );
        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            TimestampTicks::from_ticks(ticks),
            telemetry.telemetry(),
            imu,
            &packet,
        ));
    }

    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .darkride(),
        FloatOutBoyDarkRideState::Upright
    );
}

#[test]
fn running_darkride_wheelslip_uses_float_out_boy_thirty_millisecond_runaway_stop() {
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1500.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let payloads = darkride_no_footpads_payloads(FloatOutBoyMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(FloatOutBoyWheelSlipState::Detected);
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.upside_down_flags.started = true;
    state.upside_down_fault_ticks = TimestampTicks::from_ticks(0);
    state.fault_switch_ticks = TimestampTicks::from_ticks(10_000);

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(10_301),
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::ReverseStop
    );
}

#[test]
fn running_upright_wheelslip_does_not_use_darkride_runaway_timer() {
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1500.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let payloads = running_payloads(FloatOutBoyMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(FloatOutBoyWheelSlipState::Detected);
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.upside_down_flags.started = true;
    state.upside_down_fault_ticks = TimestampTicks::from_ticks(0);
    state.fault_switch_ticks = TimestampTicks::from_ticks(10_000);

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(10_301),
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            crate::domain::FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));

    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Running
    );
}

#[test]
fn app_data_running_flywheel_both_footpads_stops_like_float_out_boy_fault_check() {
    // C map: RUNNING FLYWHEEL with both footpads stops at
    // `third_party/float-out-boy/src/main.c:492-495` and then writes READY through
    // `state_stop` at `third_party/float-out-boy/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Flywheel,
    ));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_flywheel_stop_clears_wheelslip_like_float_out_boy_state_stop() {
    // C map: the same `state_stop` write that makes the run state READY also
    // clears wheelslip at `third_party/float-out-boy/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Flywheel,
    );
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(FloatOutBoyWheelSlipState::Detected);
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

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
    assert_eq!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::None);
}

#[test]
fn app_data_running_darkride_footpads_stop_like_float_out_boy_fault_check() {
    // C map: darkride still hits the shared `can_engage(d)` stop branch at
    // `third_party/float-out-boy/src/main.c:387-390` and then writes READY through
    // `state_stop` at `third_party/float-out-boy/src/state.c:29-33`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(darkride_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_timed_low_erpm_stops_like_float_out_boy_fault_check() {
    // C map: darkride reverse-stops above 300 ERPM after 500ms at
    // `third_party/float-out-boy/src/main.c:374-383`.
    let lifecycle = TimestampTicks::from_ticks(5_001);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(500.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(darkride_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_darkride_enabled_high_roll_stops_like_float_out_boy_fault_check() {
    // C map: non-darkride `check_faults(d)` stops immediately when darkride
    // faults are enabled and roll is 100-135 degrees at
    // `third_party/float-out-boy/src/main.c:465-470`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(110.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let payloads = running_payloads(FloatOutBoyMode::Normal);
    let base = payloads.base();
    let no_footpads = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::None,
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::Roll);
}

#[test]
fn app_data_running_darkride_no_footpads_does_not_use_normal_full_switch_fault() {
    // C map: darkride still uses the dedicated fault branch, not the normal
    // full-switch path at `third_party/float-out-boy/src/main.c:392-425`.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state =
        FloatOutBoyPackageState::new(darkride_no_footpads_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::None);
}

#[test]
fn app_data_running_roll_stopped_after_delay_like_float_out_boy_fault_check() {
    // C map: roll stops above `fault_roll` after `fault_delay_roll` at
    // `third_party/float-out-boy/src/main.c:474-482`.
    let lifecycle = TimestampTicks::from_ticks(3_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(70.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(running_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::Roll);
    assert!(!state.apply_requested_motor_current(telemetry.motor()));
    assert_eq!(telemetry.current_command_count(), 0);
}

#[test]
fn app_data_running_pitch_stopped_after_delay_like_float_out_boy_fault_check() {
    // C map: pitch stops above `fault_pitch` after `fault_delay_pitch` when
    // remote setpoint is below 30 degrees at
    // `third_party/float-out-boy/src/main.c:497-503`.
    let lifecycle = TimestampTicks::from_ticks(3_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_degrees(70.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(running_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::Pitch);
}

#[test]
fn app_data_running_darkride_simple_start_single_footpad_stops_during_engage_grace() {
    // C map: darkride's `can_engage(d)` branch stops immediately when a single
    // footpad becomes eligible to engage again at
    // `third_party/float-out-boy/src/main.c:387-390`.
    let lifecycle = TimestampTicks::from_ticks(5_000);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(FloatOutBoyDarkRideState::Active);
    let single_footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::Left,
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_high_erpm_stops_like_float_out_boy_fault_check() {
    // C map: darkride high-ERPM stops through the reverse-stop branch at
    // `third_party/float-out-boy/src/main.c:361-380` before `state_stop` writes READY.
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2100.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(darkride_payloads(FloatOutBoyMode::Normal));

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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        FloatOutBoyStopCondition::ReverseStop
    );
}
