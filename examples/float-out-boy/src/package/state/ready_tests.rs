use super::super::test_support::{
    balance_filter_with_pitch, edit_config, sample_all_data_payloads_with_ride_state,
    tick_float_out_boy_state_and_handle_packet,
};
use super::FloatOutBoyPackageState;
use crate::domain::{
    FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads,
    FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRealtimeBalancePitch, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
    FloatOutBoyStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn ready_payloads(
    mode: FloatOutBoyMode,
    balance_pitch: AngleDegrees,
) -> FloatOutBoyAllDataPayloads {
    // C map: these READY fixtures build the same startup attitude and base payload
    // shape that Float Out Boy feeds into `can_engage(d)` before the READY branch in
    // `third_party/float-out-boy/src/main.c:1033-1067`.
    let payloads = sample_all_data_payloads_with_ride_state(FloatOutBoyRunState::Ready, mode);
    let base = payloads.base();
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from(balance_pitch)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn ready_payloads_with_footpads(
    mode: FloatOutBoyMode,
    balance_pitch: AngleDegrees,
    footpad: FloatOutBoyFootpadSample,
) -> FloatOutBoyAllDataPayloads {
    // C map: this variant keeps the same READY base shape while substituting the
    // footpad sample that Float Out Boy's READY loop inspects before engagement.
    let payloads = sample_all_data_payloads_with_ride_state(FloatOutBoyRunState::Ready, mode);
    let base = payloads.base();
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from(balance_pitch)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

fn configure_ready_imu(firmware: &FirmwareTest, roll: AngleRadians) {
    // C map: Float Out Boy's READY tests drive the IMU attitude gate with a single
    // pitch/roll sample while leaving yaw at zero.
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(roll),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
}

#[test]
fn app_data_ready_uses_configured_startup_tolerances_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_payloads(
        FloatOutBoyMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_startup_pitch_tolerance(AngleDegrees::from_degrees(4.0)));
        assert!(config.set_startup_roll_tolerance(AngleDegrees::from_degrees(45.0)));
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

    // Upstream READY engages only inside configured startup pitch/roll
    // tolerances at `third_party/float-out-boy/src/main.c:1033-1036`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready
    );
}

#[test]
fn app_data_ready_pushstart_uses_wide_pitch_gate_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_payloads(
        FloatOutBoyMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_pushstart_enabled(true));
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
    // Upstream READY push-start engages above 1000 ERPM when `can_engage`
    // passes and pitch/roll are within 45 degrees at `third_party/float-out-boy/src/main.c:1056-1067`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::None);
}

#[test]
fn app_data_ready_pushstart_reverse_stop_blocks_negative_erpm_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_payloads(
        FloatOutBoyMode::Normal,
        AngleDegrees::from_degrees(20.0),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(20.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_pushstart_enabled(true));
        assert!(config.set_reversestop_enabled(true));
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
    // Upstream ignores backwards push-start when reverse stop is enabled
    // at `third_party/float-out-boy/src/main.c:1061-1064`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
}

#[test]
fn app_data_ready_normal_both_footpads_engages_like_float_out_boy_start_conditions() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(ready_payloads(
        FloatOutBoyMode::Normal,
        AngleDegrees::from_degrees(0.05),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(0.05)));

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
    // Upstream READY engages when startup pitch/roll tolerances and
    // `can_engage(d)` pass at `third_party/float-out-boy/src/main.c:1033-1036`; `state_engage`
    // moves to RUNNING and sets SAT_CENTERING at `third_party/float-out-boy/src/state.c:36-39`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), FloatOutBoyStopCondition::None);
}

#[test]
fn app_data_ready_flywheel_without_footpads_engages_like_float_out_boy_can_engage() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let no_footpads = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::None,
    );
    let mut state = FloatOutBoyPackageState::new(ready_payloads_with_footpads(
        FloatOutBoyMode::Flywheel,
        AngleDegrees::from_degrees(0.05),
        no_footpads,
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
    // Upstream `can_engage(d)` keeps FLYWHEEL mode engaged after footpad
    // checks at `third_party/float-out-boy/src/main.c:346-349`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_flywheel_both_footpads_stops_flywheel_like_float_out_boy_ready_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::ZERO);
    let imu = telemetry.imu();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Flywheel,
    );
    let base = payloads.base();
    let both_footpads = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(1.0),
        FloatOutBoyFootpadState::Both,
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            FloatOutBoyAllDataAttitude::new(
                FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
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
    // Upstream READY handles FLYWHEEL abort/both-footpad before start
    // conditions at `third_party/float-out-boy/src/main.c:957-963`; `flywheel_stop` returns to
    // NORMAL mode at `third_party/float-out-boy/src/main.c:1869-1873`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(ride_state.mode(), FloatOutBoyMode::Normal);
}

#[test]
fn app_data_ready_single_footpad_engages_when_dual_switch_config_is_set() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(FloatOutBoyMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::Left,
    );
    let upright_base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_dual_switch(true));
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
    // Upstream `can_engage(d)` allows a single footpad when
    // `fault_is_dual_switch` is enabled at `third_party/float-out-boy/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_single_footpad_default_config_does_not_engage_like_float_out_boy_can_engage() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(FloatOutBoyMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::Left,
    );
    let upright_base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        upright_base,
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
    // Upstream `can_engage(d)` keeps a single footpad gated unless
    // `fault_is_dual_switch` or simple start is enabled at `third_party/float-out-boy/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Ready);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::None
    );
}

#[test]
fn app_data_ready_simple_start_single_footpad_engages_after_disengage_grace_like_float_out_boy() {
    let lifecycle = TimestampTicks::from_ticks(20_001);
    let telemetry = FirmwareTest::new();
    configure_ready_imu(&telemetry, AngleRadians::from_radians(0.1));
    let imu = telemetry.imu();
    let payloads = ready_payloads(FloatOutBoyMode::Normal, AngleDegrees::from_degrees(0.05));
    let base = payloads.base();
    let single_footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.8),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::Left,
    );
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
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
    assert_eq!(ride_state.run_state(), FloatOutBoyRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::Centering
    );
}
