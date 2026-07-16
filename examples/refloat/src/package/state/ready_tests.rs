use super::super::test_support::{
    balance_filter_with_pitch, edit_config, sample_all_data_payloads_with_ride_state,
    tick_refloat_state_and_handle_packet,
};
use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataPayloads,
    RefloatFootpadSample, RefloatFootpadState, RefloatMode, RefloatRealtimeBalancePitch,
    RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn ready_payloads(mode: RefloatMode, balance_pitch: AngleDegrees) -> RefloatAllDataPayloads {
    // C map: these READY fixtures build the same startup attitude and base payload
    // shape that Refloat feeds into `can_engage(d)` before the READY branch in
    // `third_party/refloat/src/main.c:1033-1067`.
    let payloads = sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, mode);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from(balance_pitch)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn ready_payloads_with_footpads(
    mode: RefloatMode,
    balance_pitch: AngleDegrees,
    footpad: RefloatFootpadSample,
) -> RefloatAllDataPayloads {
    // C map: this variant keeps the same READY base shape while substituting the
    // footpad sample that Refloat's READY loop inspects before engagement.
    let payloads = sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, mode);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from(balance_pitch)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn configure_ready_imu(firmware: &FirmwareTest, roll: AngleRadians) {
    // C map: Refloat's READY tests drive the IMU attitude gate with a single
    // pitch/roll sample while leaving yaw at zero.
    firmware.set_imu_startup_done(true);
    firmware.set_imu_attitude(
        ImuRoll::new(roll),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
}

#[test]
fn app_data_ready_uses_configured_startup_tolerances_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_payloads(
        RefloatMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_startup_pitch_tolerance(AngleDegrees::from_degrees(4.0)));
        assert!(config.set_startup_roll_tolerance(AngleDegrees::from_degrees(45.0)));
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

    // Upstream READY engages only inside configured startup pitch/roll
    // tolerances at `third_party/refloat/src/main.c:1033-1036`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Ready
    );
}

#[test]
fn app_data_ready_pushstart_uses_wide_pitch_gate_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_payloads(
        RefloatMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_pushstart_enabled(true));
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
    // Upstream READY push-start engages above 1000 ERPM when `can_engage`
    // passes and pitch/roll are within 45 degrees at `third_party/refloat/src/main.c:1056-1067`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_pushstart_reverse_stop_blocks_negative_erpm_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_payloads(
        RefloatMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_pushstart_enabled(true));
        assert!(config.set_reversestop_enabled(true));
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
    // Upstream ignores backwards push-start when reverse stop is enabled
    // at `third_party/refloat/src/main.c:1061-1064`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
}

#[test]
fn app_data_ready_normal_both_footpads_engages_like_refloat_start_conditions() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(ready_payloads(
        RefloatMode::Normal,
        AngleDegrees::from_degrees(0.05),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(0.05)));

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
    // Upstream READY engages when startup pitch/roll tolerances and
    // `can_engage(d)` pass at `third_party/refloat/src/main.c:1033-1036`; `state_engage`
    // moves to RUNNING and sets SAT_CENTERING at `third_party/refloat/src/state.c:36-39`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_flywheel_without_footpads_engages_like_refloat_can_engage() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let no_footpads = RefloatFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        RefloatFootpadState::None,
    );
    let mut state = RefloatPackageState::new(ready_payloads_with_footpads(
        RefloatMode::Flywheel,
        AngleDegrees::from_degrees(0.05),
        no_footpads,
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
    // Upstream `can_engage(d)` keeps FLYWHEEL mode engaged after footpad
    // checks at `third_party/refloat/src/main.c:346-349`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_flywheel_both_footpads_stops_flywheel_like_refloat_ready_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Flywheel);
    let base = payloads.base();
    let both_footpads = RefloatFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(1.0),
        RefloatFootpadState::Both,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            both_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
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
    // Upstream READY handles FLYWHEEL abort/both-footpad before start
    // conditions at `third_party/refloat/src/main.c:957-963`; `flywheel_stop` returns to
    // NORMAL mode at `third_party/refloat/src/main.c:1869-1873`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.mode(), RefloatMode::Normal);
}

#[test]
fn app_data_ready_single_footpad_engages_when_dual_switch_config_is_set() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(RefloatMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        RefloatFootpadState::Left,
    );
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_dual_switch(true));
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
    // Upstream `can_engage(d)` allows a single footpad when
    // `fault_is_dual_switch` is enabled at `third_party/refloat/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_single_footpad_default_config_does_not_engage_like_refloat_can_engage() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(RefloatMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        RefloatFootpadState::Left,
    );
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
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
    // Upstream `can_engage(d)` keeps a single footpad gated unless
    // `fault_is_dual_switch` or simple start is enabled at `third_party/refloat/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::None
    );
}

#[test]
fn app_data_ready_simple_start_single_footpad_engages_after_disengage_grace_like_refloat() {
    let lifecycle = TimestampTicks::from_ticks(20_001);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(RefloatMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        RefloatFootpadState::Left,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
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
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}
