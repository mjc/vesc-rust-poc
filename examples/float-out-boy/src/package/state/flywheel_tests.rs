use super::*;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMotorPayload, FloatOutBoyAllDataStatus,
    FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyRealtimeBalanceCurrent,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRideState,
};
use crate::package::test_support::{
    imu_angular_rate, imu_pitch_rate, imu_roll_rate, imu_yaw_rate,
    sample_all_data_payloads_with_ride_state, tick_float_out_boy_state_and_handle_packet,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngleRadians, AngularVelocity, Current, DutyCycle,
    MotorCurrent, RateCurrentGain, Ratio, SignedRatio, Temperature, TemperatureLimitStart, Voltage,
};
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{ImuPitch, ImuRoll, ImuYaw, WireByte};

fn ready_at(pitch: AngleDegrees, roll: AngleDegrees) -> FloatOutBoyAllDataPayloads {
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let attitude = FloatOutBoyAllDataAttitude::new(
        base.attitude().balance_pitch(),
        ImuRoll::new(AngleRadians::from(roll)),
        ImuPitch::new(AngleRadians::from(pitch)),
    );
    FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            attitude,
            FloatOutBoyAllDataStatus::new(base.status().ride_state(), base.status().beep_reason()),
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        FloatOutBoyAppDataCommand::Flywheel.id(),
    ];
    packet.extend_from_slice(payload);
    packet
}

fn set_ride_state(state: &mut FloatOutBoyPackageState, run_state: FloatOutBoyRunState) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let previous = base.status().ride_state();
    let ride_state = FloatOutBoyRideState::new(
        run_state,
        previous.mode(),
        previous.setpoint_adjustment(),
        previous.stop_condition(),
    )
    .with_charging(previous.charging())
    .with_wheelslip(previous.wheelslip())
    .with_darkride(previous.darkride());
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
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
    );
}

fn set_footpad(state: &mut FloatOutBoyPackageState, footpad: FloatOutBoyFootpadState) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            FloatOutBoyFootpadSample::new(Voltage::ZERO, Voltage::ZERO, footpad),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );
}

fn set_duty_cycle(state: &mut FloatOutBoyPackageState, duty_cycle: DutyCycle) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let motor = base.motor();
    let motor = FloatOutBoyAllDataMotorPayload::new(
        BatteryVoltage::new(Voltage::from_volts(60.0)),
        motor.electrical_speed(),
        motor.vehicle_speed(),
        motor.currents(),
        duty_cycle,
        motor.foc_id_current(),
    );
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
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
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
    assert_eq!(ride_state.mode(), FloatOutBoyMode::Flywheel);
    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        9.0
    );
    assert_f32_eq!(
        state
            .serialized_config
            .balance()
            .kp2()
            .as_amps_per_degree_per_second(),
        0.5,
    );
    assert_f32_eq!(
        state
            .serialized_config
            .startup()
            .pitch_tolerance()
            .as_degrees(),
        0.2
    );
    assert_f32_eq!(
        state
            .serialized_config
            .startup()
            .roll_tolerance()
            .as_degrees(),
        25.0
    );
    assert_f32_eq!(
        state.serialized_config.faults().pitch_angle().as_degrees(),
        6.0
    );
    assert_f32_eq!(
        state.serialized_config.faults().roll_angle().as_degrees(),
        35.0
    );
    assert_f32_eq!(
        state.serialized_config.duty_pushback_angle().as_degrees(),
        3.0
    );
    assert_f32_eq!(
        state.serialized_config.duty_pushback_threshold().as_ratio(),
        0.199
    );
}

#[test]
fn flywheel_uses_unserialized_command_values_like_float_out_boy_runtime() {
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
    assert_f32_eq!(
        state.serialized_config.duty_pushback_threshold().as_ratio(),
        0.199
    );
}

#[test]
fn flywheel_start_rejects_first_calibration_below_seventy_degrees() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
        FloatOutBoyMode::Normal,
    );
    assert_eq!(state.serialized_config, before);
}

#[test]
fn flywheel_reuses_calibration_until_valid_forced_recalibration() {
    let mut state = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::from_degrees(10.0),
    ));
    let start = flywheel_packet(&[0x81, 0, 0, 0, 0, 1]);
    let stop = flywheel_packet(&[0x80, 0, 0, 0, 0, 1]);

    assert!(state.handle_flywheel_packet(&start));
    assert!(state.handle_flywheel_packet(&stop));
    state.all_data_payloads = ready_at(
        AngleDegrees::from_degrees(30.0),
        AngleDegrees::from_degrees(20.0),
    );

    assert!(state.handle_flywheel_packet(&start));
    let reused = state.flywheel_attitude(
        FloatOutBoyMode::Flywheel,
        AngleDegrees::from_degrees(30.0),
        AngleDegrees::from_degrees(20.0),
    );
    assert_f32_eq!(reused.0.as_degrees(), 50.0);
    assert_f32_eq!(reused.1.as_degrees(), 10.0);

    assert!(state.handle_flywheel_packet(&stop));
    state.all_data_payloads = ready_at(
        AngleDegrees::from_degrees(85.0),
        AngleDegrees::from_degrees(30.0),
    );
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x82, 0, 0, 0, 0, 1,])));
    let recalibrated = state.flywheel_attitude(
        FloatOutBoyMode::Flywheel,
        AngleDegrees::from_degrees(85.0),
        AngleDegrees::from_degrees(30.0),
    );
    assert!(recalibrated.0.as_degrees().abs() < 0.000_01);
    assert!(recalibrated.1.as_degrees().abs() < 0.000_01);
}

#[test]
fn flywheel_stop_from_ready_or_running_restores_config_and_runtime_derivations() {
    for run_state in [FloatOutBoyRunState::Ready, FloatOutBoyRunState::Running] {
        let firmware = FirmwareTest::new();
        firmware.set_clock_ticks(42);
        let mut state = FloatOutBoyPackageState::new(ready_at(
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::ZERO,
        ));
        state.idle_ticks = TimestampTicks::from_ticks(7);
        assert!(
            state
                .serialized_config
                .editor()
                .set_kp(vescpkg_rs::AngleCurrentGain::new(12.0))
        );
        assert!(state.serialized_config.editor().set_beeper_enabled(true));
        let requested = state.serialized_config;
        assert!(state.store_serialized_config(requested.as_bytes()));
        for _ in 0..240 {
            let _ = state.tick_beeper();
        }
        let persisted = state.serialized_config;
        let duty_threshold = state.runtime_duty_pushback_threshold();
        let duty_angle = state.runtime_duty_pushback_angle();
        let duty_step = state.runtime_duty_pushback_step();
        let return_step = state.runtime_tiltback_return_step();
        let balance = state.runtime_balance_loop_config();

        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(99),
            &mut |_bytes| true,
            &flywheel_packet(&[0x81, 90, 50, 30, 20, 1]),
        ));
        for _ in 0..900 {
            let _ = state.tick_beeper();
        }
        assert_eq!(state.idle_ticks, TimestampTicks::from_ticks(7));
        set_ride_state(&mut state, run_state);
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(99),
            &mut |_bytes| true,
            &flywheel_packet(&[0x80, 0, 0, 0, 0, 0]),
        ));
        assert_eq!(state.idle_ticks, TimestampTicks::from_ticks(42));

        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            FloatOutBoyMode::Normal,
        );
        assert_eq!(state.serialized_config, persisted);
        assert_eq!(state.runtime_duty_pushback_threshold(), duty_threshold);
        assert_eq!(state.runtime_duty_pushback_angle(), duty_angle);
        assert_eq!(state.runtime_duty_pushback_step(), duty_step);
        assert_eq!(state.runtime_tiltback_return_step(), return_step);
        assert_eq!(state.runtime_balance_loop_config(), balance);
        assert_eq!(
            state.take_beeper_level(),
            Some(FloatOutBoyBeeperLevel::High)
        );
        let changes: Vec<_> = (1..=240)
            .filter_map(|tick| state.tick_beeper().map(|level| (tick, level)))
            .collect();
        assert_eq!(
            changes,
            [
                (80, FloatOutBoyBeeperLevel::Low),
                (160, FloatOutBoyBeeperLevel::High),
                (240, FloatOutBoyBeeperLevel::Low),
            ],
        );
    }
}

#[test]
fn rejected_forced_recalibration_restores_the_persisted_config() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x81, 90, 50, 30, 20, 1]),
    ));
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            base.balance_current(),
            FloatOutBoyAllDataAttitude::new(
                base.attitude().balance_pitch(),
                ImuRoll::new(AngleRadians::ZERO),
                ImuPitch::new(AngleRadians::ZERO),
            ),
            FloatOutBoyAllDataStatus::new(base.status().ride_state(), base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Flywheel,
    );
    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        9.0
    );

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(2),
        &mut |_bytes| true,
        &flywheel_packet(&[0x82, 0, 0, 0, 0, 1]),
    ));

    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Normal,
    );
    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        12.0
    );
}

#[test]
fn flywheel_runtime_applies_calibrated_pitch_and_roll_offsets() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
        let mut state = FloatOutBoyPackageState::new(ready_at(
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::from_degrees(90.0),
        ));
        assert!(state.handle_packet_with_telemetry(
            firmware.telemetry(),
            &mut || TimestampTicks::from_ticks(0),
            &mut |_bytes| true,
            &flywheel_packet(&[0x81, 0, 0, 0, 0, 1]),
        ));
        set_ride_state(&mut state, FloatOutBoyRunState::Running);
        set_footpad(&mut state, FloatOutBoyFootpadState::None);
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

    // Float Out Boy computes pitch-rate projection from the raw 90-degree roll, so
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
    let mut state = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    let ready_payloads = state.all_data_payloads;
    state.refresh_motor_runtime_state(firmware.telemetry());
    state.all_data_payloads = ready_payloads;
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 0, 0, 0, 0, 1])));
    set_ride_state(&mut state, FloatOutBoyRunState::Running);
    set_footpad(&mut state, FloatOutBoyFootpadState::None);
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
        FloatOutBoySetpointAdjustment::None
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
fn flywheel_roll_wrap_uses_float_out_boy_strict_boundaries() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(ready_at(
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
            .flywheel_attitude(FloatOutBoyMode::Flywheel, AngleDegrees::ZERO, roll)
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
fn flywheel_defaults_optional_speed_and_relaxed_roll_match_float_out_boy() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));

    assert!(state.handle_packet_with_telemetry(
        firmware.telemetry(),
        &mut || TimestampTicks::from_ticks(0),
        &mut |_bytes| true,
        &flywheel_packet(&[0x85, 0, 0, 0, 0, 1, 99]),
    ));

    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        8.0
    );
    assert_f32_eq!(
        state
            .serialized_config
            .balance()
            .kp2()
            .as_amps_per_degree_per_second(),
        0.3
    );
    assert_f32_eq!(
        state.serialized_config.faults().roll_angle().as_degrees(),
        90.0
    );
    assert_f32_eq!(
        state
            .serialized_config
            .duty_pushback_speed()
            .as_degrees_per_second(),
        49.5
    );
    assert_f32_eq!(
        state
            .serialized_config
            .tiltback_return_speed()
            .as_degrees_per_second(),
        49.5
    );
}

#[test]
fn flywheel_rejection_matrix_never_mutates_package_state() {
    let valid = flywheel_packet(&[0x81, 0, 0, 0, 0, 1]);
    let mut malformed = std::vec::Vec::new();
    for length in 0..valid.len() {
        malformed.push(valid[..length].to_vec());
    }
    let mut wrong_package = valid.clone();
    wrong_package[0] = wrong_package[0].wrapping_add(1);
    malformed.push(wrong_package);
    let mut wrong_command = valid.clone();
    wrong_command[1] = wrong_command[1].wrapping_add(1);
    malformed.push(wrong_command);
    malformed.push(flywheel_packet(&[0x01, 0, 0, 0, 0, 1]));

    for packet in malformed {
        let mut state = FloatOutBoyPackageState::new(ready_at(
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::ZERO,
        ));
        let before = state;

        assert!(!state.handle_flywheel_packet(&packet), "packet={packet:?}");
        assert_eq!(state, before, "packet={packet:?}");
    }

    for run_state in [
        FloatOutBoyRunState::Disabled,
        FloatOutBoyRunState::Startup,
        FloatOutBoyRunState::Running,
    ] {
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            run_state,
            FloatOutBoyMode::Normal,
        ));
        let before = state;

        assert!(state.handle_flywheel_packet(&valid));
        assert_eq!(state, before, "run_state={run_state:?}");
    }

    let mut handtest = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::HandTest,
    ));
    let before = handtest;

    assert!(handtest.handle_flywheel_packet(&valid));
    assert_eq!(handtest, before);
}

#[test]
fn flywheel_konami_uses_the_typed_start_path_after_invalid_sequence_reset() {
    let mut konami = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    set_footpad(&mut konami, FloatOutBoyFootpadState::Left);
    konami.refresh_konami_runtime_state(TimestampTicks::from_ticks(1_501));
    set_footpad(&mut konami, FloatOutBoyFootpadState::Right);
    konami.refresh_konami_runtime_state(TimestampTicks::from_ticks(3_002));
    assert_eq!(
        konami
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Normal,
    );

    for (index, footpad) in [
        FloatOutBoyFootpadState::Left,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Right,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Left,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Right,
        FloatOutBoyFootpadState::None,
    ]
    .into_iter()
    .enumerate()
    {
        set_footpad(&mut konami, footpad);
        konami.refresh_konami_runtime_state(TimestampTicks::from_ticks(
            4_503 + u32::try_from(index).unwrap_or(u32::MAX) * 1_501,
        ));
    }

    let mut direct = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(direct.handle_flywheel_packet(&flywheel_packet(&[0x82, 0, 0, 0, 0, 1,])));

    assert_eq!(
        konami
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Flywheel,
    );
    assert_eq!(konami.serialized_config, direct.serialized_config);
    assert_eq!(
        konami.runtime_balance_loop_config(),
        direct.runtime_balance_loop_config()
    );
    assert_eq!(
        konami.flywheel_attitude(
            FloatOutBoyMode::Flywheel,
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::ZERO,
        ),
        direct.flywheel_attitude(
            FloatOutBoyMode::Flywheel,
            AngleDegrees::from_degrees(80.0),
            AngleDegrees::ZERO,
        ),
    );
}

#[test]
fn headlight_konami_actions_and_confirmation_match_refloat() {
    let mut state = FloatOutBoyPackageState::new(ready_at(AngleDegrees::ZERO, AngleDegrees::ZERO));
    set_footpad(&mut state, FloatOutBoyFootpadState::None);
    let mut config = state.serialized_config.as_bytes().to_vec();
    config[227] = crate::lcm::FloatOutBoyLedMode::Both.id();
    assert!(state.store_serialized_config(&config));
    assert!(state.serialized_config.headlights_enabled());

    for (index, footpad) in [
        FloatOutBoyFootpadState::Right,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Right,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Left,
    ]
    .into_iter()
    .enumerate()
    {
        set_footpad(&mut state, footpad);
        state.refresh_konami_runtime_state(TimestampTicks::from_ticks(
            u32::try_from(index + 1).unwrap_or(u32::MAX) * 1_501,
        ));
    }

    assert!(!state.serialized_config.headlights_enabled());
    assert_eq!(
        state.internal_led_confirmation_start_for_test(),
        Some(0.7505)
    );

    for (index, footpad) in [
        FloatOutBoyFootpadState::Left,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Left,
        FloatOutBoyFootpadState::None,
        FloatOutBoyFootpadState::Right,
    ]
    .into_iter()
    .enumerate()
    {
        set_footpad(&mut state, footpad);
        state.refresh_konami_runtime_state(TimestampTicks::from_ticks(
            10_000 + u32::try_from(index + 1).unwrap_or(u32::MAX) * 1_501,
        ));
    }

    assert!(state.serialized_config.headlights_enabled());
    assert_eq!(
        state.internal_led_confirmation_start_for_test(),
        Some(1.7505)
    );
}

#[test]
fn calibrated_flywheel_pitch_commands_the_expected_final_motor_current() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::ZERO),
        ImuPitch::new(AngleRadians::from_degrees(79.0)),
        ImuYaw::new(AngleRadians::ZERO),
    );
    firmware.set_imu_angular_rate(imu_angular_rate(
        imu_roll_rate(AngularVelocity::ZERO),
        imu_pitch_rate(AngularVelocity::ZERO),
        imu_yaw_rate(AngularVelocity::ZERO),
    ));
    let mut state = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 80, 30, 0, 100, 1,])));
    set_ride_state(&mut state, FloatOutBoyRunState::Running);
    set_footpad(&mut state, FloatOutBoyFootpadState::None);
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    state.all_data_payloads = FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::ZERO)),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::ZERO)),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    );

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(1),
        firmware.telemetry(),
        firmware.imu(),
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(firmware.motor()));

    let base = state.all_data_payloads().base();
    let error = base.setpoints().board().angle().as_degrees()
        - base.attitude().balance_pitch().angle_degrees().as_degrees();
    let expected = error * 8.0 * 0.2;
    let actual = firmware.commanded_current().current().as_amps();
    // The calibrated 80° reference minus the live 79° pitch produces +1°.
    // With Kp=8 A/°, zero rate/I/booster terms, and Refloat's 0.2 output
    // smoothing from rest, the final motor command follows the signed
    // setpoint error exactly.
    assert!(expected < 0.0, "error={error}");
    assert!(base.booster_current().current().is_zero());
    assert!(
        (actual - expected).abs() < 0.000_1,
        "actual={actual}, expected={expected}, base={base:?}",
    );
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
    let mut state = FloatOutBoyPackageState::new(ready_at(
        AngleDegrees::from_degrees(80.0),
        AngleDegrees::ZERO,
    ));
    assert!(
        state
            .serialized_config
            .editor()
            .set_kp(vescpkg_rs::AngleCurrentGain::new(12.0))
    );
    let requested = state.serialized_config;
    assert!(state.store_serialized_config(requested.as_bytes()));
    let persisted = state.serialized_config;
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 90, 0, 0, 0, 1])));

    set_ride_state(&mut state, FloatOutBoyRunState::Running);
    set_footpad(&mut state, FloatOutBoyFootpadState::Both);
    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(1));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready,
    );
    assert_f32_eq!(
        state.serialized_config.balance().kp().as_amps_per_degree(),
        9.0
    );
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Flywheel,
    );

    set_footpad(&mut state, FloatOutBoyFootpadState::Left);
    state.refresh_imu_runtime_state(firmware.imu(), TimestampTicks::from_ticks(2));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    assert_eq!(ride_state.mode(), FloatOutBoyMode::Normal);
    assert_eq!(state.serialized_config, persisted);
    assert!(state.apply_motor_control(
        firmware.motor(),
        ride_state.run_state(),
        TimestampTicks::from_ticks(3),
    ));
    assert!(firmware.commanded_current().current().is_zero());

    state.all_data_payloads = ready_at(AngleDegrees::from_degrees(80.0), AngleDegrees::ZERO);
    assert!(state.handle_flywheel_packet(&flywheel_packet(&[0x81, 90, 0, 0, 0, 1])));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        FloatOutBoyMode::Flywheel,
    );
}
