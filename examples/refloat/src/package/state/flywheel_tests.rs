use super::*;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataMotorPayload, RefloatAllDataStatus,
    RefloatFootpadSample, RefloatFootpadState, RefloatRideState,
};
use crate::package::test_support::{
    imu_angular_rate, imu_pitch_rate, imu_roll_rate, imu_yaw_rate,
    sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngleRadians, AngularVelocity, DutyCycle, RateCurrentGain,
    Ratio, SignedRatio, Temperature, TemperatureLimitStart, Voltage,
};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{ImuPitch, ImuRoll, ImuYaw, WireByte};

fn ready_at(pitch: AngleDegrees, roll: AngleDegrees) -> RefloatAllDataPayloads {
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let attitude = RefloatAllDataAttitude::new(
        base.attitude().balance_pitch(),
        ImuRoll::new(AngleRadians::from(roll)),
        ImuPitch::new(AngleRadians::from(pitch)),
    );
    RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            attitude,
            RefloatAllDataStatus::new(base.status().ride_state(), base.status().beep_reason()),
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

fn flywheel_packet(payload: &[u8]) -> std::vec::Vec<u8> {
    let mut packet = std::vec![
        REFLOAT_APP_DATA_PACKAGE_ID.get(),
        RefloatAppDataCommand::Flywheel.id(),
    ];
    packet.extend_from_slice(payload);
    packet
}

fn set_ride_state(state: &mut RefloatPackageState, run_state: RefloatRunState) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let previous = base.status().ride_state();
    let ride_state = RefloatRideState::new(
        run_state,
        previous.mode(),
        previous.setpoint_adjustment(),
        previous.stop_condition(),
    )
    .with_charging(previous.charging())
    .with_wheelslip(previous.wheelslip())
    .with_darkride(previous.darkride());
    state.all_data_payloads = RefloatAllDataPayloads::new(
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
    );
}

fn set_footpad(state: &mut RefloatPackageState, footpad: RefloatFootpadState) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    state.all_data_payloads = RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            RefloatFootpadSample::new(Voltage::ZERO, Voltage::ZERO, footpad),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );
}

fn set_duty_cycle(state: &mut RefloatPackageState, duty_cycle: DutyCycle) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let motor = base.motor();
    let motor = RefloatAllDataMotorPayload::new(
        BatteryVoltage::new(Voltage::from_volts(60.0)),
        motor.electrical_speed(),
        motor.vehicle_speed(),
        motor.currents(),
        duty_cycle,
        motor.foc_id_current(),
    );
    state.all_data_payloads = RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            motor,
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );
}

#[test]
fn flywheel_start_calibrates_upright_attitude_and_applies_payload_overrides() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::from_degrees(12.0),
    ));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 90, 50, 30, 20, 1, 12]),
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(ride_state.mode(), RefloatMode::Flywheel);
    assert_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        9.0
    );
    assert_eq!(
        state
            .serialized_config
            .balance()
            .kp2()
            .as_amps_per_degree_per_second(),
        0.5,
    );
    assert_eq!(
        state
            .serialized_config
            .startup()
            .pitch_tolerance()
            .as_degrees(),
        0.2
    );
    assert_eq!(
        state
            .serialized_config
            .startup()
            .roll_tolerance()
            .as_degrees(),
        25.0
    );
    assert_eq!(
        state.serialized_config.faults().pitch_angle().as_degrees(),
        6.0
    );
    assert_eq!(
        state.serialized_config.faults().roll_angle().as_degrees(),
        35.0
    );
    assert_eq!(
        state.serialized_config.duty_pushback_angle().as_degrees(),
        3.0
    );
    assert_eq!(
        state.serialized_config.duty_pushback_threshold().as_ratio(),
        0.199
    );
}

#[test]
fn flywheel_uses_unserialized_command_values_like_refloat_runtime() {
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 1, 20, 3, 20, 1])));

    // C compares against the live `cfg[4] * 0.01f` value. Serializing that
    // temporary value through float16 would truncate it to 0.199, but the
    // flywheel control loop never performs that lossy round trip.
    assert_eq!(
        state.runtime_duty_pushback_threshold(),
        WireByte::new(20).scaled(0.01, 0.0, Ratio::from_ratio_const)
    );
    assert_eq!(
        state.runtime_duty_pushback_angle(),
        WireByte::new(3).scaled(0.1, 0.0, AngleDegrees::from_degrees)
    );
    let balance = state.runtime_balance_loop_config();
    assert_eq!(
        balance.kp,
        WireByte::new(1).scaled(0.1, 0.0, AngleCurrentGain::new)
    );
    assert_eq!(
        balance.kp2,
        WireByte::new(20).scaled(0.01, 0.0, RateCurrentGain::new)
    );
    assert_eq!(
        state.serialized_config.duty_pushback_threshold().as_ratio(),
        0.199
    );
}

#[test]
fn flywheel_start_rejects_first_calibration_below_seventy_degrees() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(69.0),
        AngleDegrees::ZERO,
    ));
    let before = state.serialized_config;

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 0, 0, 0, 0, 1]),
    ));

    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        RefloatMode::Normal,
    );
    assert_eq!(state.serialized_config, before);
}

#[test]
fn flywheel_stop_restores_the_persisted_config() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(12.0))
    );
    assert!(state.serialized_config.editor().set_beeper_enabled(true));
    let persisted = *state.serialized_config.as_bytes();
    assert!(state.store_serialized_config(&persisted));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 90, 50, 30, 20, 1]),
    ));
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x80, 0, 0, 0, 0, 0]),
    ));

    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        RefloatMode::Normal,
    );
    assert_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        12.0
    );
    assert_eq!(state.take_beeper_level(), Some(RefloatBeeperLevel::High));
}

#[test]
fn flywheel_runtime_applies_calibrated_pitch_and_roll_offsets() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::from_degrees(12.0),
    ));
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 0, 0, 0, 0, 1]),
    ));
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_degrees(15.0)),
        ImuPitch::new(AngleRadians::from_degrees(75.0)),
        ImuYaw::new(AngleRadians::ZERO),
    );

    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(1));

    let attitude = state.all_data_payloads().base().attitude();
    assert!(
        (AngleDegrees::from(attitude.pitch().angle()) - AngleDegrees::from_degrees(5.0)).abs()
            < AngleDegrees::from_degrees(0.001)
    );
    assert!(
        (AngleDegrees::from(attitude.roll().angle()) - AngleDegrees::from_degrees(3.0)).abs()
            < AngleDegrees::from_degrees(0.001)
    );
    assert!(
        (attitude.balance_pitch().angle_degrees() - AngleDegrees::from_degrees(5.0)).abs()
            < AngleDegrees::from_degrees(0.001)
    );
}

#[test]
fn flywheel_pitch_rate_projection_uses_raw_roll_before_calibration_offset() {
    fn balance_current(gyro_pitch: AngularVelocity) -> Current {
        let firmware = FirmwareTest::new();
        firmware.set_imu_ready(true);
        let mut state = RefloatPackageState::new(ready_at(
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::from_degrees(90.0),
        ));
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |_bytes| true,
            &flywheel_packet(&[0x81, 0, 0, 0, 0, 1]),
        ));
        set_ride_state(&mut state, RefloatRunState::Running);
        set_footpad(&mut state, RefloatFootpadState::None);
        firmware.set_imu_attitude(
            ImuRoll::new(AngleRadians::from_degrees(90.0)),
            ImuPitch::new(AngleRadians::from_degrees(80.0)),
            ImuYaw::new(AngleRadians::ZERO),
        );
        firmware.set_imu_angular_rate(imu_angular_rate(
            imu_roll_rate(AngularVelocity::ZERO),
            imu_pitch_rate(gyro_pitch),
            imu_yaw_rate(AngularVelocity::ZERO),
        ));

        state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(1));
        assert!(state.apply_requested_motor_current(firmware.motor()));

        firmware.commanded_current().current()
    }

    let stationary = balance_current(AngularVelocity::ZERO);
    let pitching = balance_current(AngularVelocity::from_degrees_per_second(10.0));

    // Refloat computes pitch-rate projection from the raw 90-degree roll, so
    // gyro pitch is fully suppressed even though the calibrated roll is zero.
    assert!((pitching - stationary).abs() < Current::from_amps(0.0001));
}

#[test]
fn flywheel_applies_duty_pushback_without_exposing_pushback_status() {
    let firmware = FirmwareTest::new().with_temperature_limit_starts(
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(85.0)),
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(95.0)),
    );
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::ZERO),
        ImuPitch::new(AngleRadians::from_degrees(80.0)),
        ImuYaw::new(AngleRadians::ZERO),
    );
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    let ready_payloads = state.all_data_payloads;
    state.refresh_motor_runtime_state(firmware.telemetry());
    state.all_data_payloads = ready_payloads;
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 0, 0, 0, 0, 1])));
    set_ride_state(&mut state, RefloatRunState::Running);
    set_footpad(&mut state, RefloatFootpadState::None);
    set_duty_cycle(
        &mut state,
        DutyCycle::new(SignedRatio::from_ratio_const(0.2)),
    );
    let initial_board_setpoint = state.all_data_payloads().base().setpoints().board().angle();
    let duty_step = AngleDegrees::from_degrees(5.0 / 832.0);

    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(1));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.setpoints().board().angle(),
        initial_board_setpoint + duty_step
    );
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::None
    );

    set_duty_cycle(
        &mut state,
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(2));

    assert_eq!(
        state.all_data_payloads().base().setpoints().board().angle(),
        initial_board_setpoint + duty_step - duty_step,
    );
}

#[test]
fn flywheel_roll_wrap_uses_refloat_strict_boundaries() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 0, 0, 0, 0, 1]),
    ));

    let transformed_roll = |roll| {
        state
            .flywheel_attitude(RefloatMode::Flywheel, AngleDegrees::ZERO, roll)
            .1
    };

    assert_eq!(
        transformed_roll(AngleDegrees::from_degrees(-200.0)),
        AngleDegrees::from_degrees(-200.0)
    );
    assert_eq!(
        transformed_roll(AngleDegrees::from_degrees(200.0)),
        AngleDegrees::from_degrees(200.0)
    );
    assert_eq!(
        transformed_roll(AngleDegrees::from_degrees(-201.0)),
        AngleDegrees::from_degrees(159.0)
    );
    assert_eq!(
        transformed_roll(AngleDegrees::from_degrees(201.0)),
        AngleDegrees::from_degrees(-159.0)
    );
}

#[test]
fn flywheel_defaults_optional_speed_and_relaxed_roll_match_refloat() {
    let firmware = FirmwareTest::new();
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x85, 0, 0, 0, 0, 1, 99]),
    ));

    assert_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        8.0
    );
    assert_eq!(
        state
            .serialized_config
            .balance()
            .kp2()
            .as_amps_per_degree_per_second(),
        0.3
    );
    assert_eq!(
        state.serialized_config.faults().roll_angle().as_degrees(),
        90.0
    );
    assert_eq!(
        state
            .serialized_config
            .duty_pushback_speed()
            .as_degrees_per_second(),
        49.5
    );
    assert_eq!(
        state
            .serialized_config
            .tiltback_return_speed()
            .as_degrees_per_second(),
        49.5
    );
}

#[test]
fn flywheel_rejects_unarmed_and_short_payloads_without_mutation() {
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    let before = state;

    assert!(!state.handle_flywheel_packet(&flywheel_packet(&[0x01, 0, 0, 0, 0, 1])));
    assert!(!state.handle_flywheel_packet(&flywheel_packet(&[0x81, 0, 0, 0, 0])));
    assert_eq!(state, before);
}

#[test]
fn flywheel_start_obeys_refloat_mode_and_ready_gates() {
    let request = flywheel_packet(&[0x81, 0, 0, 0, 0, 1]);
    for (run_state, mode) in [
        (RefloatRunState::Ready, RefloatMode::HandTest),
        (RefloatRunState::Running, RefloatMode::Normal),
    ] {
        let mut state =
            RefloatPackageState::new(sample_all_data_payloads_with_ride_state(run_state, mode));
        let before = state;

        assert!(state.handle_flywheel_packet(&request));
        assert_eq!(state, before);
    }
}

#[test]
fn flywheel_footpad_abort_restores_config_after_the_footpads_release() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::ZERO),
        ImuPitch::new(AngleRadians::from_degrees(80.0)),
        ImuYaw::new(AngleRadians::ZERO),
    );
    let mut state = RefloatPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(12.0))
    );
    let persisted = *state.serialized_config.as_bytes();
    assert!(state.store_serialized_config(&persisted));
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 90, 0, 0, 0, 1])));

    set_ride_state(&mut state, RefloatRunState::Running);
    set_footpad(&mut state, RefloatFootpadState::Both);
    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(1));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Ready,
    );
    assert_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        9.0
    );

    set_footpad(&mut state, RefloatFootpadState::None);
    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(2));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(ride_state.mode(), RefloatMode::Normal);
    assert_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        12.0
    );
}
