use super::RefloatPackageState;
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude, RefloatAllDataBasePayload,
    RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatMode, RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch,
    RefloatRealtimeBoosterCurrent, RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints,
    RefloatRunState,
};
use crate::package::test_support::{
    RefloatConfigTestBytes, balance_filter_with_pitch, default_refloat_config_bytes,
    editable_config_from_state, sample_all_data_payloads_with_ride_state,
    tick_refloat_state_and_handle_packet,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

use std::vec::Vec;

#[test]
fn startup_ready_gate_refreshes_imu_attitude_like_refloat() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.25)),
        ImuPitch::new(AngleRadians::from_radians(-0.125)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let payloads = state.all_data_payloads();
    assert_eq!(
        payloads.base().status().ride_state().run_state(),
        RefloatRunState::Ready
    );
    assert_eq!(payloads.base().attitude().roll().angle().as_radians(), 0.25);
    assert_eq!(
        payloads.base().attitude().pitch().angle().as_radians(),
        -0.125
    );
}

#[test]
fn startup_balance_filter_uses_firmware_orientation_like_refloat_data_init() {
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    state.initialize_balance_filter(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(0.995_004_2),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(0.099_833_42),
            ImuQuaternionZ::new(0.0),
        ),
    ));

    assert!((state.balance_filter.balance_pitch().angle().as_radians() - 0.2).abs() < 1.0e-5);
}

#[test]
fn startup_ready_gate_does_not_engage_active_footpads_until_next_loop() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Startup,
        RefloatMode::Normal,
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // C map: `STATE_STARTUP` sets READY and breaks at
    // `third_party/refloat/src/main.c:833-852`; READY engagement cannot run
    // until the next main-loop iteration.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Ready,
    );
}

#[test]
fn startup_ready_resets_runtime_vars_like_refloat() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.25)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(1.2)));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().run_state(),
        RefloatRunState::Ready
    );
    // Refloat calls `reset_runtime_vars(d)` before READY at
    // `third_party/refloat/src/main.c:833-837`; reset clears
    // `balance_current` at `third_party/refloat/src/main.c:246`, resets
    // module setpoints at `third_party/refloat/src/main.c:239-244`, then
    // seeds only the board setpoint from balance pitch at
    // `third_party/refloat/src/main.c:249-252`.
    assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
    assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
    let expected_startup_setpoint = AngleRadians::from_radians(1.2).as_degrees();
    assert!(
        (base.setpoints().board().angle().as_degrees() - expected_startup_setpoint).abs() < 0.0001
    );
    [
        base.setpoints().atr(),
        base.setpoints().brake_tilt(),
        base.setpoints().torque_tilt(),
        base.setpoints().turn_tilt(),
        base.setpoints().remote(),
    ]
    .into_iter()
    .for_each(|setpoint| assert_eq!(setpoint.angle().as_degrees(), 0.0));
}

#[test]
fn disabled_config_applies_before_startup_ready_like_refloat() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let mut incoming = default_refloat_config_bytes();
    incoming.edit_refloat_config(|config| {
        assert!(config.set_disabled(true));
    });
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(state.store_serialized_config(&incoming));
    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Upstream `configure(d)` applies `disabled` before the control-loop
    // startup gate at `third_party/refloat/src/main.c:184-190`; `state_set_disabled` forces
    // `STATE_DISABLED` at `third_party/refloat/src/state.c:41-47`, so `third_party/refloat/src/main.c:833-838`
    // cannot promote STARTUP to READY in this configuration.
    assert!(editable_config_from_state(&state).metadata().disabled());
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Disabled,
    );
}

#[test]
fn ready_engage_resets_runtime_vars_like_refloat() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
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
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(0.05)));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    let ride_state = base.status().ride_state();
    // Upstream `engage(d)` calls `reset_runtime_vars(d)` before
    // `state_engage(d)` at `third_party/refloat/src/main.c:263-270`, then
    // breaks out of the READY branch without running the RUNNING
    // balance-current loop.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
    assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
    let expected_engage_setpoint = AngleRadians::from_radians(0.05).as_degrees();
    assert_eq!(
        base.setpoints().board().angle().as_degrees(),
        expected_engage_setpoint
    );
    assert_eq!(base.setpoints().remote().angle().as_degrees(), 0.0);
    assert!(!state.apply_requested_motor_current(bindings));
}

#[test]
fn ready_normal_charging_does_not_engage_like_refloat_can_engage() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let imu = telemetry.imu();
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let charging_state = base
        .status()
        .ride_state()
        .with_charging(RefloatChargingState::Charging);
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(charging_state, base.status().beep_reason()),
        base.footpad(),
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
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` rejects charging state before checking
    // footpads at `third_party/refloat/src/main.c:328-331`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.charging(), RefloatChargingState::Charging);
}

#[test]
fn motor_payload_refreshes_like_refloat_motor_data_update() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
        MotorCurrent::new(Current::from_amps(12.25)),
        BatteryCurrent::new(Current::from_amps(4.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
    );
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));

    let motor = state.all_data_payloads().base().motor();
    assert_eq!(
        motor.electrical_speed().rpm().as_revolutions_per_minute(),
        1234.0
    );
    assert_eq!(motor.vehicle_speed().speed().as_meters_per_second(), 5.5);
    assert_eq!(motor.motor_current().current().as_amps(), 12.25);
    assert_eq!(motor.battery_current().current().as_amps(), 0.04);
    assert_eq!(motor.duty_cycle().ratio().as_ratio(), 0.375);
}

#[test]
fn foc_id_current_refreshes_like_refloat_all_data() {
    // Refloat v1.2.1 encodes `fabsf(VESC_IF->foc_get_id()) * 3` for
    // compact all-data at `third_party/refloat/src/main.c:1364-1368`.
    let now = TimestampTicks::from_ticks(0);
    let telemetry =
        FirmwareTest::new().with_foc_id_current(Some(MotorCurrent::new(Current::from_amps(-4.0))));
    let imu = telemetry.imu();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    state.refresh_runtime_state(telemetry.telemetry(), imu, now);

    let mut packet = Vec::new();
    let mut now = || now;
    let mut send = |bytes: &[u8]| {
        packet.extend_from_slice(bytes);
        true
    };
    assert!(state.handle_packet_with_runtime(
        telemetry.telemetry(),
        imu,
        &mut now,
        &mut send,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));

    let motor = state.all_data_payloads().base().motor();
    assert_eq!(
        motor
            .foc_id_current()
            .as_measured()
            .expect("measured Id current")
            .current()
            .as_amps(),
        -4.0
    );
    assert_eq!(packet[33], 12);
}

fn running_runtime_fixture() -> (TimestampTicks, FirmwareTest, RefloatPackageState) {
    // C map: this fixture shapes the RUNNING loop's cached runtime state the
    // same way Refloat refreshes it before balance-current and motor-control math.
    let now = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_startup_done(true);
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_degrees(1.0)),
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
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(1.0)));

    (now, telemetry, state)
}

#[test]
fn running_runtime_requests_balance_current_like_refloat_loop() {
    let (app_data, telemetry, mut state) = running_runtime_fixture();
    let imu = telemetry.imu();
    let bindings = telemetry.motor();

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    // Upstream RUNNING computes `d->balance_current` and then requests it
    // via `motor_control_request_current` at `third_party/refloat/src/main.c:949-956`.
    assert!((telemetry.commanded_current().current().as_amps() - 3.8).abs() < 0.0001);
}

#[test]
fn running_motor_apply_uses_current_branch_like_refloat_loop() {
    let (app_data, telemetry, mut state) = running_runtime_fixture();
    let imu = telemetry.imu();
    let bindings = telemetry.motor();

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_motor_control(
        bindings,
        RefloatRunState::Running,
        TimestampTicks::from_ticks(1),
    ));

    // Upstream RUNNING computes and requests balance current at
    // `third_party/refloat/src/main.c:918-956`, then `refloat_thd` calls
    // `motor_control_apply` at `third_party/refloat/src/main.c:1076`; a
    // current request takes the `mc_set_current` branch at
    // `third_party/refloat/src/motor_control.c:92-121`.
    assert_eq!(telemetry.current_command_count(), 1);
    assert_eq!(telemetry.brake_current_command_count(), 0);
    assert_eq!(telemetry.duty_command_count(), 0);
    assert!((telemetry.commanded_current().current().as_amps() - 3.8).abs() < 0.0001);
}
