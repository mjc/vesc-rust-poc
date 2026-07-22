use super::super::test_support::{
    balance_filter_with_pitch, edit_config, imu_angular_rate, imu_pitch_rate, imu_roll_rate,
    imu_yaw_rate, sample_all_data_payloads_with_ride_state, tick_refloat_state_and_handle_packet,
};
use super::RefloatPackageState;
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude, RefloatAllDataBasePayload,
    RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataStatus,
    RefloatAppDataCommand, RefloatFocIdCurrent, RefloatFootpadSample, RefloatFootpadState,
    RefloatMode, RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch,
    RefloatRealtimeBoosterCurrent, RefloatRealtimeFilteredMotorCurrent,
    RefloatRealtimeMotorCurrents, RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints,
    RefloatRunState, RefloatSetpointAdjustment, RefloatWheelSlipState,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn app_data_running_uses_balance_filter_pitch_like_refloat_pid() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(10.0)));
        assert!(config.set_kp2(vescpkg_rs::RateCurrentGain::new(0.0)));
        assert!(config.set_ki(vescpkg_rs::IntegralCurrentGain::new(0.0)));
        assert!(config.set_kp_brake(vescpkg_rs::PidScale::new(1.0)));
        assert!(config.set_booster_angle(AngleDegrees::from_degrees(100.0)));
        assert!(config.set_booster_current(MotorCurrent::new(Current::ZERO)));
    });
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(5.0)));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    // C refreshes `imu.balance_pitch` from `balance_filter_get_pitch` at
    // `third_party/refloat/src/imu.c:35-41` before `pid_update` computes
    // `setpoint - imu->balance_pitch` at `third_party/refloat/src/pid.c:40`.
    assert!((telemetry.commanded_current().current().as_amps() + 10.0).abs() < 0.0001);
    assert!(
        (state
            .all_data_payloads()
            .base()
            .attitude()
            .balance_pitch()
            .angle()
            .as_radians()
            * 180.0
            / core::f32::consts::PI
            - 5.0)
            .abs()
            < 0.0001
    );
}

#[test]
fn app_data_running_accumulates_angle_i_balance_current_like_refloat_pid() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
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
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));
    assert!(
        (telemetry.commanded_current().current().as_amps() - 4.001).abs() < 0.0001,
        "{:?}",
        telemetry.commanded_current()
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    // Upstream `pid_update` accumulates `pid->i += pid->p * config->ki`
    // and clamps it at `third_party/refloat/src/pid.c:40-46`; RUNNING adds P + I before
    // smoothing balance current at `third_party/refloat/src/main.c:932-954`.
    assert!(
        (telemetry.commanded_current().current().as_amps() - 7.2018).abs() < 0.0001,
        "{:?}",
        telemetry.commanded_current()
    );
}

#[test]
fn app_data_running_clamps_angle_i_at_default_ki_limit_like_refloat_pid() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10_000.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(0.0)));
    });

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        lifecycle,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    // Refloat default `ki_limit` is 30A (`settings.xml:1679-1707`);
    // `pid_update` clamps the I term at `third_party/refloat/src/pid.c:40-46` before RUNNING
    // smooths it into `balance_current` at `third_party/refloat/src/main.c:932-954`.
    assert!((telemetry.commanded_current().current().as_amps() - 6.0).abs() < 0.0001);
}

#[test]
fn app_data_running_limits_handtest_and_flywheel_current_like_refloat_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();

    for (mode, expected_current) in [
        (RefloatMode::HandTest, 1.4_f32),
        (RefloatMode::Flywheel, 8.0_f32),
    ] {
        let payloads = sample_all_data_payloads_with_ride_state(RefloatRunState::Running, mode);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let footpad = if matches!(mode, RefloatMode::Flywheel) {
            RefloatFootpadSample::new(
                Voltage::from_volts(0.0),
                Voltage::from_volts(0.0),
                RefloatFootpadState::None,
            )
        } else {
            base.footpad()
        };
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            footpad,
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
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
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(bindings));

        // Upstream RUNNING clamps `new_current` to 7A for HANDTEST and
        // 40A for FLYWHEEL at `third_party/refloat/src/main.c:932-942`, then smooths it into
        // `balance_current` at `third_party/refloat/src/main.c:949-954`.
        assert!(
            (telemetry.commanded_current().current().as_amps() - expected_current).abs() < 0.0001,
            "{mode:?}: {:?}",
            telemetry.commanded_current()
        );
    }
}

#[test]
fn app_data_running_wheelslip_without_traction_control_smooths_current_like_refloat_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(RefloatWheelSlipState::Detected);
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        setpoints,
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
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    // Upstream RUNNING only sets `balance_current = 0` when
    // `traction_control` is set at `third_party/refloat/src/main.c:949-954`; wheelslip alone
    // remains a UI/state flag and the current path still smooths.
    assert_ne!(telemetry.commanded_current().current().as_amps(), 0.0);
    assert_ne!(
        state
            .all_data_payloads()
            .base()
            .balance_current()
            .current()
            .current()
            .as_amps(),
        0.0
    );
}

#[test]
fn app_data_normal_algorithm_trace_matches_refloat_loop_order() {
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_degrees(1.5)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    telemetry.set_imu_angular_rate(imu_angular_rate(
        imu_roll_rate(AngularVelocity::from_degrees_per_second(0.0)),
        imu_pitch_rate(AngularVelocity::from_degrees_per_second(0.0)),
        imu_yaw_rate(AngularVelocity::from_degrees_per_second(0.0)),
    ));
    let imu = telemetry.imu();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let stopped_base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.attitude(),
        base.status(),
        base.footpad(),
        base.setpoints(),
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
            RefloatRealtimeMotorCurrents::new(
                MotorCurrent::new(Current::from_amps(0.0)),
                DirectionalMotorCurrent::new(Current::from_amps(0.0)),
                RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                    Current::from_amps(0.0),
                )),
                BatteryCurrent::new(Current::from_amps(0.0)),
            ),
            DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(0.0))),
        ),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        stopped_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(2.0)));
    edit_config(&mut state, |config| {
        assert!(config.set_hertz(vescpkg_rs::SampleRate::from_hertz(100.0)));
        assert!(config.set_startup_speed(AngularVelocity::from_degrees_per_second(50.0)));
    });

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(0),
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    let engaged_base = state.all_data_payloads().base();
    let engaged_ride_state = engaged_base.status().ride_state();
    // Upstream READY/NORMAL engages through `engage(d)` at
    // `third_party/refloat/src/main.c:263-270`; `reset_runtime_vars(d)`
    // seeds only the board setpoint from balance pitch at
    // `third_party/refloat/src/main.c:239-252`, and READY breaks before
    // RUNNING PID in `third_party/refloat/src/main.c:1018-1037`.
    assert_eq!(engaged_ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(engaged_ride_state.mode(), RefloatMode::Normal);
    assert_eq!(
        engaged_ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert!((engaged_base.setpoints().board().angle().as_degrees() - 2.0).abs() < 0.0001);
    assert_eq!(
        engaged_base.balance_current().current().current().as_amps(),
        0.0
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(1),
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    let running_base = state.all_data_payloads().base();
    let balance = state.balance_config_for_test();
    let kp = balance.kp().as_amps_per_degree();
    let ki = balance.ki().as_amps_per_degree_per_tick();
    let ki_limit = balance.ki_limit().current().as_amps();
    let expected_board_setpoint = 1.5;
    let expected_setpoint_error = expected_board_setpoint - 2.0;
    let unclamped_i: f32 = expected_setpoint_error * ki;
    let expected_i = if ki_limit > 0.0 && unclamped_i.abs() > ki_limit {
        ki_limit * unclamped_i.signum()
    } else {
        unclamped_i
    };
    let current_limit = state.motor_current_max.current().as_amps();
    let expected_new_current =
        (expected_setpoint_error * kp + expected_i).clamp(-current_limit, current_limit);
    let expected_smoothed_current = expected_new_current * 0.2;
    // Upstream RUNNING centers with `startup_speed / hertz` at
    // `third_party/refloat/src/main.c:172`,
    // `third_party/refloat/src/main.c:304-310`, and
    // `third_party/refloat/src/main.c:869-875`;
    // then NORMAL PID and the regular motor-current limit run at
    // `third_party/refloat/src/main.c:918-956` before requesting motor
    // current. Raw pitch equals the centered board setpoint here, so the
    // booster proportional is zero by `third_party/refloat/src/main.c:921-922`.
    assert!(
        (running_base.setpoints().board().angle().as_degrees() - expected_board_setpoint).abs()
            < 0.0001
    );
    assert_eq!(
        running_base.booster_current().current().current().as_amps(),
        0.0
    );
    assert!(
        (running_base.balance_current().current().current().as_amps() - expected_smoothed_current)
            .abs()
            < 0.0001
    );
    let bindings = telemetry.motor();
    assert!(state.apply_motor_control(
        bindings,
        running_base.status().ride_state().run_state(),
        TimestampTicks::from_ticks(1),
    ));
    // Upstream main loop calls `motor_control_apply` after the balance loop
    // at `third_party/refloat/src/main.c:1075-1079`; requested current
    // takes the current-control branch at
    // `third_party/refloat/src/motor_control.c:92-99`.
    assert_eq!(telemetry.keep_alive_count(), 1);
    assert_eq!(telemetry.current_off_delay_count(), 1);
    assert_eq!(telemetry.current_command_count(), 1);
    assert!(
        (telemetry.commanded_current().current().as_amps() - expected_smoothed_current).abs()
            < 0.0001
    );
    assert_eq!(telemetry.duty_command_count(), 0);
    assert_eq!(telemetry.brake_current_command_count(), 0);
}
