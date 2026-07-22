use super::RefloatPackageState;
use crate::beeper::{RefloatBeeperCount, RefloatBeeperLevel};
use crate::bms::{RefloatBmsSample, RefloatBmsTemperature};
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude, RefloatAllDataBasePayload,
    RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand, RefloatBeepReason,
    RefloatChargingState, RefloatFootpadSample, RefloatFootpadState, RefloatMode,
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints, RefloatRideState,
    RefloatRunState, RefloatSetpointAdjustment, RefloatWheelSlipState,
};
use crate::package::test_support::{
    RefloatConfigTestBytes, balance_filter_with_pitch, default_refloat_config_bytes, edit_config,
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
    telemetry.set_imu_ready(true);
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
fn startup_ready_above_low_voltage_margin_schedules_one_long_beep_like_refloat() {
    let telemetry = FirmwareTest::new()
        .with_input_voltage(InputVoltage::new(Voltage::from_volts(60.0)))
        .with_battery_cell_count(BatteryCellCount::try_new(18).expect("18s battery"));
    telemetry.set_imu_ready(true);
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    enable_beeper(&mut state);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(0),
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let changes: Vec<_> = (1..=900)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();

    assert_eq!(
        changes,
        [
            (1, RefloatBeeperLevel::Low),
            (300, RefloatBeeperLevel::Low),
            (600, RefloatBeeperLevel::High),
            (900, RefloatBeeperLevel::Low),
        ]
    );
}

#[test]
fn startup_ready_below_low_voltage_margin_reports_low_battery_and_beeps_twice() {
    let telemetry = FirmwareTest::new()
        .with_input_voltage(InputVoltage::new(Voltage::from_volts(58.0)))
        .with_battery_cell_count(BatteryCellCount::try_new(18).expect("18s battery"));
    telemetry.set_imu_ready(true);
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    enable_beeper(&mut state);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(0),
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::LowBattery
    );
    let changes: Vec<_> = (1..=1_500)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (1, RefloatBeeperLevel::Low),
            (300, RefloatBeeperLevel::Low),
            (600, RefloatBeeperLevel::High),
            (900, RefloatBeeperLevel::Low),
            (1_200, RefloatBeeperLevel::High),
            (1_500, RefloatBeeperLevel::Low),
        ]
    );
}

#[test]
fn startup_ready_beep_count_truncates_and_caps_voltage_deficit_like_refloat() {
    let warning_threshold = Voltage::from_volts(59.0);
    let cases = [
        (Voltage::from_volts(60.0), RefloatBeeperCount::ONE),
        (Voltage::from_volts(58.9), RefloatBeeperCount::ONE),
        (Voltage::from_volts(58.0), RefloatBeeperCount::TWO),
        (Voltage::from_volts(57.0), RefloatBeeperCount::THREE),
        (Voltage::from_volts(56.0), RefloatBeeperCount::FOUR),
        (Voltage::from_volts(55.0), RefloatBeeperCount::FIVE),
        (Voltage::from_volts(54.0), RefloatBeeperCount::SIX),
        (Voltage::from_volts(53.0), RefloatBeeperCount::SEVEN),
        (Voltage::from_volts(40.0), RefloatBeeperCount::SEVEN),
    ];

    for (battery_voltage, expected) in cases {
        assert_eq!(
            super::imu_runtime::startup_ready_beep_count(warning_threshold, battery_voltage),
            expected,
            "battery voltage: {battery_voltage:?}"
        );
    }
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
    telemetry.set_imu_ready(true);
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
    telemetry.set_imu_ready(true);
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
    telemetry.set_imu_ready(true);
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
    telemetry.set_imu_ready(true);
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
    telemetry.set_imu_ready(true);
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
    let telemetry = FirmwareTest::new()
        .with_runtime_motor(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
            TotalMotorCurrent::new(Current::from_amps(12.25)),
            InputCurrent::new(Current::from_amps(4.0)),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.375)),
        )
        .with_directional_motor_current(DirectionalMotorCurrent::new(Current::from_amps(-6.75)));
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
    assert_eq!(motor.directional_motor_current().current().as_amps(), -6.75,);
    let normalized_frequency = 5.0 / 832.0;
    let k = vescpkg_rs::tan(core::f32::consts::PI * normalized_frequency);
    let first_filtered_sample = -6.75 * k * k / (1.0 + k / 0.707 + k * k);
    assert!(
        (motor.filtered_motor_current().current().current().as_amps() - first_filtered_sample)
            .abs()
            < 0.0001
    );
    assert_eq!(motor.battery_current().current().as_amps(), 0.04);
    assert_eq!(motor.duty_cycle().ratio().as_ratio(), 0.00375);
}

#[test]
fn foc_id_current_refreshes_like_refloat_all_data() {
    // Refloat v1.2.1 encodes `fabsf(VESC_IF->foc_get_id()) * 3` for
    // compact all-data at `third_party/refloat/src/main.c:1364-1368`.
    let now = TimestampTicks::from_ticks(0);
    let telemetry =
        FirmwareTest::new().with_d_axis_current(Some(DCurrent::new(Current::from_amps(-4.0))));
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
    telemetry.set_imu_ready(true);
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

fn ready_bms_fixture() -> (FirmwareTest, RefloatPackageState) {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        RefloatFootpadSample::new(Voltage::ZERO, Voltage::ZERO, RefloatFootpadState::None),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    (telemetry, state)
}

fn running_reverse_stop_fixture(
    reverse_total_erpm: Rpm,
    board_setpoint: AngleDegrees,
    motor_erpm: Rpm,
) -> (TimestampTicks, FirmwareTest, RefloatPackageState) {
    let now = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(motor_erpm),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_setpoint_adjustment(RefloatSetpointAdjustment::ReverseStop);
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        RefloatRealtimeRuntimeSetpoint::new(board_setpoint),
        base.setpoints().atr(),
        base.setpoints().brake_tilt(),
        base.setpoints().torque_tilt(),
        base.setpoints().turn_tilt(),
        base.setpoints().remote(),
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
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
    state.reverse_total_erpm = reverse_total_erpm;

    (now, telemetry, state)
}

#[test]
fn running_enters_reverse_stop_from_reverse_motor_speed_like_refloat() {
    let app_data = TimestampTicks::from_ticks(0);
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-201.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    telemetry.set_imu_ready(true);
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Normal,
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_reversestop_enabled(true));
    });

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // C map: reverse-stop entry precedes every protective pushback branch at
    // `third_party/refloat/src/main.c:538-552`.
    assert_eq!(
        state
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .setpoint_adjustment(),
        RefloatSetpointAdjustment::ReverseStop,
    );
}

#[test]
fn running_reverse_stop_carries_only_error_pushbacks_into_erpm_like_refloat() {
    let cases = [
        (
            RefloatSetpointAdjustment::PushbackHighVoltage,
            Rpm::from_revolutions_per_minute(80_000.0),
        ),
        (
            RefloatSetpointAdjustment::PushbackLowVoltage,
            Rpm::from_revolutions_per_minute(80_000.0),
        ),
        (
            RefloatSetpointAdjustment::PushbackTemperature,
            Rpm::from_revolutions_per_minute(80_000.0),
        ),
        (RefloatSetpointAdjustment::PushbackDuty, Rpm::ZERO),
    ];

    for (adjustment, expected_total_erpm) in cases {
        let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
            SignedRatio::from_ratio_const(0.10),
            Rpm::from_revolutions_per_minute(-201.0),
            adjustment,
            InputVoltage::new(Voltage::from_volts(72.0)),
        );
        edit_config(&mut state, |config| {
            assert!(config.set_reversestop_enabled(true));
        });

        tick_running_protective_pushback(&mut state, &telemetry, app_data);

        // Refloat carries only SAT values at or above `SAT_PB_HIGH_VOLTAGE`
        // into reverse-stop at `third_party/refloat/src/main.c:538-550`.
        assert_eq!(state.reverse_total_erpm, expected_total_erpm);
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .setpoint_adjustment(),
            RefloatSetpointAdjustment::ReverseStop,
        );
    }
}

#[test]
fn running_reverse_stop_grows_target_past_erpm_tolerance_like_refloat() {
    let (app_data, telemetry, mut state) = running_reverse_stop_fixture(
        Rpm::from_revolutions_per_minute(-20_000.0),
        AngleDegrees::ZERO,
        Rpm::from_revolutions_per_minute(-1_000.0),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // C accumulates ERPM and applies `(abs(total) - 20000) * 0.00008`
    // at `third_party/refloat/src/main.c:522-529`.
    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        AngleDegrees::from_degrees(0.08),
    );
}

#[test]
fn running_reverse_stop_exits_below_half_tolerance_while_moving_forward_like_refloat() {
    let (app_data, telemetry, mut state) = running_reverse_stop_fixture(
        Rpm::from_revolutions_per_minute(-10_001.0),
        AngleDegrees::from_degrees(0.25),
        Rpm::from_revolutions_per_minute(2.0),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // C clears Reverse Stop, the accumulator, and target together at
    // `third_party/refloat/src/main.c:530-536`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .setpoint_adjustment(),
        RefloatSetpointAdjustment::None,
    );
    assert_eq!(state.reverse_total_erpm, Rpm::ZERO);
    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        AngleDegrees::ZERO,
    );
}

#[test]
fn running_reverse_stop_stays_active_at_half_tolerance_while_reversing_like_refloat() {
    let (app_data, telemetry, mut state) = running_reverse_stop_fixture(
        Rpm::from_revolutions_per_minute(-9_999.0),
        AngleDegrees::from_degrees(0.25),
        Rpm::from_revolutions_per_minute(-1.0),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // C requires both half tolerance and nonnegative instantaneous ERPM at
    // `third_party/refloat/src/main.c:530-536`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .setpoint_adjustment(),
        RefloatSetpointAdjustment::ReverseStop,
    );
    assert_eq!(
        state.reverse_total_erpm,
        Rpm::from_revolutions_per_minute(-10_000.0),
    );
    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        AngleDegrees::from_degrees(0.25),
    );
}

fn running_protective_pushback_fixture(
    duty_ratio: SignedRatio,
    motor_erpm: Rpm,
    adjustment: RefloatSetpointAdjustment,
    input_voltage: InputVoltage,
) -> (TimestampTicks, FirmwareTest, RefloatPackageState) {
    let app_data = TimestampTicks::from_ticks(0);
    let duty_cycle = DutyCycle::new(duty_ratio);
    let telemetry = FirmwareTest::new()
        .with_runtime_motor(
            ElectricalSpeed::new(motor_erpm),
            VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
            TotalMotorCurrent::new(Current::from_amps(0.0)),
            InputCurrent::new(Current::from_amps(0.0)),
            duty_cycle,
        )
        .with_input_voltage(input_voltage)
        .with_battery_cell_count(BatteryCellCount::try_new(18).expect("18s battery"));
    telemetry.set_imu_ready(true);
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let motor = base.motor();
    let motor = crate::domain::RefloatAllDataMotorPayload::new(
        motor.battery_voltage(),
        motor.electrical_speed(),
        motor.vehicle_speed(),
        motor.currents(),
        duty_cycle,
        motor.foc_id_current(),
    );
    let status = RefloatAllDataStatus::new(
        base.status()
            .ride_state()
            .with_setpoint_adjustment(adjustment),
        base.status().beep_reason(),
    );
    let board_setpoint = match adjustment {
        RefloatSetpointAdjustment::Centering => AngleDegrees::from_degrees(1.0),
        RefloatSetpointAdjustment::PushbackDuty => AngleDegrees::from_degrees(5.0),
        RefloatSetpointAdjustment::PushbackHighVoltage
        | RefloatSetpointAdjustment::PushbackLowVoltage
        | RefloatSetpointAdjustment::PushbackTemperature => AngleDegrees::from_degrees(8.0),
        _ => AngleDegrees::ZERO,
    };
    let board_setpoint = if motor_erpm.is_negative() {
        -board_setpoint
    } else {
        board_setpoint
    };
    let setpoints = base
        .setpoints()
        .with_board(RefloatRealtimeRuntimeSetpoint::new(board_setpoint));
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        status,
        base.footpad(),
        setpoints,
        base.booster_current(),
        motor,
    );
    let state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    (app_data, telemetry, state)
}

fn set_protective_ride_state(
    state: &mut RefloatPackageState,
    mode: RefloatMode,
    adjustment: RefloatSetpointAdjustment,
    wheelslip: RefloatWheelSlipState,
) {
    let payloads = state.all_data_payloads();
    let base = payloads.base();
    let previous = base.status().ride_state();
    let ride_state = RefloatRideState::new(
        previous.run_state(),
        mode,
        adjustment,
        previous.stop_condition(),
    )
    .with_charging(previous.charging())
    .with_wheelslip(wheelslip)
    .with_darkride(previous.darkride());
    let footpad = if matches!(mode, RefloatMode::Flywheel) {
        RefloatFootpadSample::new(Voltage::ZERO, Voltage::ZERO, RefloatFootpadState::None)
    } else {
        base.footpad()
    };
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    state.all_data_payloads =
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}

fn settle_motor_acceleration(state: &mut RefloatPackageState, motor_erpm: Rpm) {
    for _ in 0..40 {
        state.motor_acceleration.record(motor_erpm);
    }
}

fn tick_running_protective_pushback(
    state: &mut RefloatPackageState,
    telemetry: &FirmwareTest,
    now: TimestampTicks,
) {
    assert!(tick_refloat_state_and_handle_packet(
        state,
        now,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
}

fn enable_bms(state: &mut RefloatPackageState) {
    let mut config = default_refloat_config_bytes();
    config[265] = 1;
    assert!(state.store_serialized_config(&config));
}

fn enable_beeper(state: &mut RefloatPackageState) {
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
}

fn first_beeper_high_tick(state: &mut RefloatPackageState, limit: usize) -> Option<usize> {
    (1..=limit).find(|_| matches!(state.tick_beeper(), Some(RefloatBeeperLevel::High)))
}

#[test]
fn running_temperature_warning_uses_refloat_margin_priority_and_long_alert() {
    let cases = [
        (
            MosfetTemperature::new(Temperature::from_degrees_celsius(82.5)),
            MotorTemperature::new(Temperature::from_degrees_celsius(20.0)),
            RefloatBeepReason::MosfetTemperature,
        ),
        (
            MosfetTemperature::new(Temperature::from_degrees_celsius(20.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(92.5)),
            RefloatBeepReason::MotorTemperature,
        ),
        (
            MosfetTemperature::new(Temperature::from_degrees_celsius(82.5)),
            MotorTemperature::new(Temperature::from_degrees_celsius(92.5)),
            RefloatBeepReason::MosfetTemperature,
        ),
    ];

    for (mosfet_temperature, motor_temperature, expected_reason) in cases {
        let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
            SignedRatio::from_ratio_const(0.0),
            Rpm::from_revolutions_per_minute(1_000.0),
            RefloatSetpointAdjustment::None,
            InputVoltage::new(Voltage::from_volts(72.0)),
        );
        let telemetry = telemetry
            .with_temperature_limit_starts(
                TemperatureLimitStart::new(Temperature::from_degrees_celsius(85.0)),
                TemperatureLimitStart::new(Temperature::from_degrees_celsius(95.0)),
            )
            .with_temperatures(mosfet_temperature, motor_temperature);
        enable_beeper(&mut state);

        tick_running_protective_pushback(&mut state, &telemetry, app_data);

        let base = state.all_data_payloads().base();
        assert_eq!(base.status().beep_reason(), expected_reason);
        assert_eq!(
            base.status().ride_state().setpoint_adjustment(),
            RefloatSetpointAdjustment::None
        );
        assert_eq!(first_beeper_high_tick(&mut state, 600), Some(600));
    }
}

fn record_bms_sample(
    state: &mut RefloatPackageState,
    cell_low_voltage: Voltage,
    cell_high_voltage: Voltage,
    cell_low_temperature: RefloatBmsTemperature,
    cell_high_temperature: RefloatBmsTemperature,
    bms_high_temperature: RefloatBmsTemperature,
    message_age: VescSeconds,
) {
    state.record_bms_sample(RefloatBmsSample::new(
        cell_low_voltage,
        cell_high_voltage,
        cell_low_temperature,
        cell_high_temperature,
        bms_high_temperature,
        message_age,
    ));
}

#[test]
fn running_enters_duty_pushback_above_configured_threshold_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.81),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Default Refloat duty pushback targets 5 degrees above 0.80 duty and
    // moves by `duty_pushback_speed / hertz` each loop.
    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackDuty,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        state.runtime_duty_pushback_step(),
    );
}

#[test]
fn running_wheelslip_refreshes_timer_and_zeros_target_above_max_duty_like_refloat() {
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.91),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::PushbackDuty,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    set_protective_ride_state(
        &mut state,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::PushbackDuty,
        RefloatWheelSlipState::Detected,
    );
    settle_motor_acceleration(&mut state, Rpm::from_revolutions_per_minute(1_000.0));
    state.traction_control = true;
    state.wheelslip_ticks = TimestampTicks::from_ticks(1);
    let now = TimestampTicks::from_ticks(5_000);

    tick_running_protective_pushback(&mut state, &telemetry, now);

    assert_eq!(state.wheelslip_ticks, now);
    assert!(!state.traction_control);
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .wheelslip(),
        RefloatWheelSlipState::Detected
    );
    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        AngleDegrees::ZERO
    );
}

#[test]
fn running_wheelslip_exit_uses_strict_timer_and_raw_duty_thresholds_like_refloat() {
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.84),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    set_protective_ride_state(
        &mut state,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatWheelSlipState::Detected,
    );
    settle_motor_acceleration(&mut state, Rpm::from_revolutions_per_minute(1_000.0));
    state.wheelslip_ticks = TimestampTicks::from_ticks(0);

    tick_running_protective_pushback(&mut state, &telemetry, TimestampTicks::from_ticks(2_000));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .wheelslip(),
        RefloatWheelSlipState::Detected
    );

    tick_running_protective_pushback(&mut state, &telemetry, TimestampTicks::from_ticks(2_001));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .wheelslip(),
        RefloatWheelSlipState::None
    );
    drop(telemetry);

    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.85),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    set_protective_ride_state(
        &mut state,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatWheelSlipState::Detected,
    );
    settle_motor_acceleration(&mut state, Rpm::from_revolutions_per_minute(1_000.0));
    state.wheelslip_ticks = TimestampTicks::from_ticks(0);

    tick_running_protective_pushback(&mut state, &telemetry, TimestampTicks::from_ticks(2_001));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .wheelslip(),
        RefloatWheelSlipState::Detected
    );
}

#[test]
fn reverse_stop_entry_precedes_wheelslip_detection_like_refloat() {
    let (now, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.50),
        Rpm::from_revolutions_per_minute(-3_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    edit_config(&mut state, |config| {
        assert!(config.set_reversestop_enabled(true));
    });

    tick_running_protective_pushback(&mut state, &telemetry, now);

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::ReverseStop
    );
    assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::None);
}

#[test]
fn running_duty_pushback_uses_negative_target_for_reverse_erpm_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.81),
        Rpm::from_revolutions_per_minute(-1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        -state.runtime_duty_pushback_step(),
    );
}

#[test]
fn running_duty_at_threshold_does_not_push_back_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.80),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::None,
    );
    assert_eq!(base.setpoints().board().angle(), AngleDegrees::ZERO);
}

#[test]
fn running_duty_pushback_clears_below_threshold_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.79),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::PushbackDuty,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::None,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        AngleDegrees::from_degrees(5.0) - state.runtime_tiltback_return_step(),
    );
}

#[test]
fn running_enters_high_voltage_pushback_one_volt_above_threshold_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(78.5)),
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        app_data,
        telemetry.telemetry(),
        telemetry.imu(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackHighVoltage,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        AngleDegrees::from_degrees(8.0),
    );
}

#[test]
fn bms_cell_over_voltage_enters_immediate_high_voltage_pushback_like_refloat() {
    let now = TimestampTicks::from_ticks(10_000);
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    enable_bms(&mut state);
    enable_beeper(&mut state);
    record_bms_sample(
        &mut state,
        Voltage::from_volts(4.0),
        Voltage::from_volts(4.4),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::ZERO,
    );
    state.refresh_bms_runtime_state(now);

    tick_running_protective_pushback(&mut state, &telemetry, now);

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackHighVoltage,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        AngleDegrees::from_degrees(8.0),
    );
    assert_eq!(
        base.status().beep_reason(),
        RefloatBeepReason::CellHighVoltage
    );
    assert_eq!(state.high_voltage_ticks, TimestampTicks::from_ticks(0));
    assert_eq!(first_beeper_high_tick(&mut state, 160), Some(160));
}

#[test]
fn bms_connection_fault_uses_high_voltage_angle_and_error_pushback_like_refloat() {
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    enable_bms(&mut state);
    enable_beeper(&mut state);
    record_bms_sample(
        &mut state,
        Voltage::from_volts(4.0),
        Voltage::from_volts(4.1),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::from_seconds(6.0),
    );
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(0));
    let now = TimestampTicks::from_ticks(50_001);
    state.refresh_bms_runtime_state(now);

    tick_running_protective_pushback(&mut state, &telemetry, now);

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackError,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        AngleDegrees::from_degrees(8.0),
    );
    assert_eq!(
        base.status().beep_reason(),
        RefloatBeepReason::BmsConnection
    );
    assert_eq!(first_beeper_high_tick(&mut state, 600), Some(600));
}

#[test]
fn bms_temperature_faults_use_low_voltage_angle_and_source_reason_order() {
    let cases = [
        (
            RefloatBmsTemperature::from_degrees_celsius(20),
            RefloatBmsTemperature::from_degrees_celsius(46),
            RefloatBmsTemperature::from_degrees_celsius(35),
            RefloatBeepReason::CellOverTemperature,
        ),
        (
            RefloatBmsTemperature::from_degrees_celsius(-1),
            RefloatBmsTemperature::from_degrees_celsius(30),
            RefloatBmsTemperature::from_degrees_celsius(35),
            RefloatBeepReason::CellUnderTemperature,
        ),
        (
            RefloatBmsTemperature::from_degrees_celsius(20),
            RefloatBmsTemperature::from_degrees_celsius(30),
            RefloatBmsTemperature::from_degrees_celsius(61),
            RefloatBeepReason::BmsOverTemperature,
        ),
    ];

    for (cell_low_temperature, cell_high_temperature, bms_temperature, expected_reason) in cases {
        let (_, telemetry, mut state) = running_protective_pushback_fixture(
            SignedRatio::from_ratio_const(0.10),
            Rpm::from_revolutions_per_minute(1_000.0),
            RefloatSetpointAdjustment::None,
            InputVoltage::new(Voltage::from_volts(72.0)),
        );
        enable_bms(&mut state);
        enable_beeper(&mut state);
        record_bms_sample(
            &mut state,
            Voltage::from_volts(4.0),
            Voltage::from_volts(4.1),
            cell_low_temperature,
            cell_high_temperature,
            bms_temperature,
            VescSeconds::ZERO,
        );
        let now = TimestampTicks::from_ticks(0);
        state.refresh_bms_runtime_state(now);

        tick_running_protective_pushback(&mut state, &telemetry, now);

        let base = state.all_data_payloads().base();
        assert_eq!(
            base.status().ride_state().setpoint_adjustment(),
            RefloatSetpointAdjustment::PushbackTemperature,
        );
        assert_eq!(
            base.setpoints().board().angle(),
            AngleDegrees::from_degrees(10.0),
        );
        assert_eq!(base.status().beep_reason(), expected_reason);
        assert_eq!(first_beeper_high_tick(&mut state, 600), Some(600));
    }
}

#[test]
fn bms_cell_under_voltage_bypasses_pack_sag_checks_like_refloat() {
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(72.0)),
    );
    enable_bms(&mut state);
    enable_beeper(&mut state);
    record_bms_sample(
        &mut state,
        Voltage::from_volts(2.6),
        Voltage::from_volts(2.7),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::ZERO,
    );
    let now = TimestampTicks::from_ticks(0);
    state.refresh_bms_runtime_state(now);

    tick_running_protective_pushback(&mut state, &telemetry, now);

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackLowVoltage,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        AngleDegrees::from_degrees(10.0),
    );
    assert_eq!(
        base.status().beep_reason(),
        RefloatBeepReason::CellLowVoltage
    );
    assert_eq!(first_beeper_high_tick(&mut state, 160), Some(160));
}

#[test]
fn ready_bms_connection_alert_uses_strict_fifteen_second_timer_like_refloat() {
    let (telemetry, mut state) = ready_bms_fixture();
    enable_bms(&mut state);
    record_bms_sample(
        &mut state,
        Voltage::from_volts(4.0),
        Voltage::from_volts(4.1),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::from_seconds(6.0),
    );
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(0));
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(50_001));

    state.refresh_imu_runtime_state(telemetry.imu(), TimestampTicks::from_ticks(150_000));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::BmsConnection,
    );

    state.refresh_imu_runtime_state(telemetry.imu(), TimestampTicks::from_ticks(150_001));
    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::BmsConnection,
    );
}

#[test]
fn ready_bms_connection_alert_schedules_four_short_beeps_like_refloat() {
    let (telemetry, mut state) = ready_bms_fixture();
    enable_bms(&mut state);
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    state.refresh_config_runtime_state();
    record_bms_sample(
        &mut state,
        Voltage::from_volts(4.0),
        Voltage::from_volts(4.1),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::from_seconds(6.0),
    );
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(0));
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(50_001));
    state.refresh_imu_runtime_state(telemetry.imu(), TimestampTicks::from_ticks(150_001));

    let changes: Vec<_> = (1..=720)
        .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
        .collect();

    // The main loop's preceding footpad `beep_off(false)` writes low before
    // the BMS alert is queued; the four-beep sequence still contributes the
    // following nine transitions exactly like Refloat.
    assert_eq!(changes.len(), 10);
    assert_eq!(changes.first(), Some(&(1, RefloatBeeperLevel::Low)));
    assert_eq!(changes.last(), Some(&(720, RefloatBeeperLevel::Low)));
}

#[test]
fn ready_bms_cell_balance_alert_requires_disengage_and_alert_delays_like_refloat() {
    let (telemetry, mut state) = ready_bms_fixture();
    enable_bms(&mut state);
    record_bms_sample(
        &mut state,
        Voltage::from_volts(3.8),
        Voltage::from_volts(4.1),
        RefloatBmsTemperature::from_degrees_celsius(20),
        RefloatBmsTemperature::from_degrees_celsius(30),
        RefloatBmsTemperature::from_degrees_celsius(35),
        VescSeconds::ZERO,
    );
    state.refresh_bms_runtime_state(TimestampTicks::from_ticks(0));
    state.disengage_ticks = TimestampTicks::from_ticks(100_000);

    state.refresh_imu_runtime_state(telemetry.imu(), TimestampTicks::from_ticks(150_000));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::CellBalance,
    );

    state.refresh_imu_runtime_state(telemetry.imu(), TimestampTicks::from_ticks(150_001));
    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::CellBalance,
    );
}

#[test]
fn ready_idle_nag_waits_for_stable_voltage_and_beeps_every_minute_like_refloat() {
    let (telemetry, mut state) = ready_bms_fixture();
    enable_beeper(&mut state);
    let imu = telemetry.imu();

    state.refresh_imu_runtime_state(imu, TimestampTicks::from_ticks(18_000_000));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::Idle,
    );

    state.refresh_imu_runtime_state(imu, TimestampTicks::from_ticks(18_600_000));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::Idle,
    );

    state.refresh_imu_runtime_state(imu, TimestampTicks::from_ticks(18_600_001));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::Idle,
    );

    state.refresh_imu_runtime_state(imu, TimestampTicks::from_ticks(19_200_001));
    assert_ne!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::Idle,
    );

    state.refresh_imu_runtime_state(imu, TimestampTicks::from_ticks(19_200_002));
    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::Idle,
    );
    assert_eq!(first_beeper_high_tick(&mut state, 600), Some(600));
}

#[test]
fn running_high_voltage_pushback_uses_strict_half_second_delay_like_refloat() {
    let (_, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(78.0)),
    );

    tick_running_protective_pushback(&mut state, &telemetry, TimestampTicks::from_ticks(5_000));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .setpoint_adjustment(),
        RefloatSetpointAdjustment::None,
    );
    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        RefloatBeepReason::HighVoltage,
    );

    tick_running_protective_pushback(&mut state, &telemetry, TimestampTicks::from_ticks(5_001));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackHighVoltage,
    );
}

#[test]
fn running_low_voltage_refreshes_high_voltage_timer_before_every_selection_branch_like_refloat() {
    let cases = [
        (
            RefloatMode::Normal,
            RefloatSetpointAdjustment::Centering,
            RefloatWheelSlipState::None,
        ),
        (
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatWheelSlipState::None,
        ),
        (
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatWheelSlipState::Detected,
        ),
        (
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::None,
            RefloatWheelSlipState::None,
        ),
    ];
    let now = TimestampTicks::from_ticks(10_000);

    for (mode, adjustment, wheelslip) in cases {
        let (_, telemetry, mut state) = running_protective_pushback_fixture(
            SignedRatio::from_ratio_const(0.10),
            Rpm::from_revolutions_per_minute(-1_000.0),
            adjustment,
            InputVoltage::new(Voltage::from_volts(72.0)),
        );
        set_protective_ride_state(&mut state, mode, adjustment, wheelslip);
        if matches!(adjustment, RefloatSetpointAdjustment::ReverseStop) {
            state.reverse_ticks = now;
        }

        tick_running_protective_pushback(&mut state, &telemetry, now);

        // Refloat refreshes this timer before every setpoint-adjustment branch
        // at `third_party/refloat/src/main.c:512-518`.
        assert_eq!(
            state.high_voltage_ticks, now,
            "mode={mode:?}, adjustment={adjustment:?}, wheelslip={wheelslip:?}",
        );
    }
}

#[test]
fn running_high_voltage_pushback_uses_negative_target_for_reverse_erpm_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.10),
        Rpm::from_revolutions_per_minute(-1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(78.5)),
    );

    tick_running_protective_pushback(&mut state, &telemetry, app_data);

    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        AngleDegrees::from_degrees(-8.0),
    );
}

#[test]
fn running_duty_pushback_precedes_high_voltage_like_refloat() {
    let (app_data, telemetry, mut state) = running_protective_pushback_fixture(
        SignedRatio::from_ratio_const(0.81),
        Rpm::from_revolutions_per_minute(1_000.0),
        RefloatSetpointAdjustment::None,
        InputVoltage::new(Voltage::from_volts(78.5)),
    );

    tick_running_protective_pushback(&mut state, &telemetry, app_data);

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::PushbackDuty,
    );
    assert_eq!(
        base.setpoints().board().angle(),
        state.runtime_duty_pushback_step(),
    );
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
    assert_eq!(
        telemetry.commanded_current(),
        state.all_data_payloads().base().balance_current().current()
    );
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
    assert_eq!(
        telemetry.commanded_current(),
        state.all_data_payloads().base().balance_current().current()
    );
}
