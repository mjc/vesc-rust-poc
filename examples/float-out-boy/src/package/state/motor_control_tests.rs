use super::super::test_support::{
    FloatOutBoyConfigTestBytes, sample_all_data_payloads_with_ride_state,
    tick_float_out_boy_state_and_handle_packet,
};
use super::FloatOutBoyPackageState;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand,
    FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterTorque, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRideState, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn output_alpha() -> f32 {
    let omega = 2.0 * core::f32::consts::PI * 25.0 / 500.0;
    omega - 0.5 * omega * omega
}

#[test]
fn requested_current_applies_like_float_out_boy_motor_control() {
    let motor = FirmwareTest::new();
    let bindings = motor.motor();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    state.request_motor_current(MotorCurrent::new(Current::from_amps(6.25)));
    assert!(state.apply_requested_motor_current(bindings));

    // Upstream `motor_control_apply` resets timeout, keeps current control
    // on for 50ms, sends the requested current, then clears the request at
    // `third_party/float-out-boy/src/motor_control.c:92-99` and `third_party/float-out-boy/src/motor_control.c:121-122`.
    assert_eq!(motor.keep_alive_count(), 1);
    assert_eq!(motor.current_off_delay_count(), 1);
    assert_f32_eq!(
        motor.commanded_current_off_delay().duration().as_seconds(),
        0.05
    );
    assert_eq!(motor.current_command_count(), 1);
    assert_f32_eq!(motor.commanded_current().current().as_amps(), 6.25);
    assert!(!state.apply_requested_motor_current(bindings));
    assert_eq!(motor.current_command_count(), 1);
}

#[test]
fn unified_remote_move_drives_the_ready_motor_path_through_typed_torque() {
    let firmware = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::ZERO),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    let imu = firmware.imu();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        FloatOutBoyFootpadSample::new(Voltage::ZERO, Voltage::ZERO, FloatOutBoyFootpadState::None),
        base.setpoints(),
        base.booster_torque(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::from_groups(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let command_time = TimestampTicks::from_ticks(30_001);

    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        command_time,
        firmware.telemetry(),
        imu,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Remote.id(),
            127,
        ],
    ));
    assert_eq!(
        state.remote_move_target_for_test(),
        Some(Speed::from_kilometers_per_hour(5.0))
    );
    assert!(tick_float_out_boy_state_and_handle_packet(
        &mut state,
        TimestampTicks::from_ticks(30_021),
        firmware.telemetry(),
        imu,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));
    let expected_current = 6.01 / state.motor_torque_constant().as_newton_meters_per_amp();
    assert!(state.apply_requested_motor_current(firmware.motor()));

    // Cutoff defaults a zero configured move limit to 5 km/h for command input.
    // At 2 ms, the PI requests 6.01 Nm, then the production path converts it
    // with the live firmware-derived motor torque constant.
    assert!(
        (firmware.commanded_current().current().as_amps() - expected_current).abs() < 0.0001,
        "actual={:?} expected={expected_current} torque_constant={} vehicle_speed={}",
        firmware.commanded_current(),
        state.motor_torque_constant().as_newton_meters_per_amp(),
        state
            .all_data_payloads()
            .base()
            .motor()
            .vehicle_speed()
            .speed()
            .as_kilometers_per_hour(),
    );
}

#[test]
fn idle_motor_control_uses_smoothed_erpm_for_one_sample_spike_like_refloat() {
    let firmware = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::ZERO),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));

    state.refresh_motor_runtime_state(firmware.telemetry());
    assert!(state.apply_motor_control(
        firmware.motor(),
        FloatOutBoyRunState::Ready,
        TimestampTicks::from_ticks(0),
    ));

    let firmware = firmware.with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1_000.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    state.refresh_motor_runtime_state(firmware.telemetry());
    assert!(state.apply_motor_control(
        firmware.motor(),
        FloatOutBoyRunState::Ready,
        TimestampTicks::from_ticks(10_001),
    ));

    // Refloat smooths 0 -> 1,000 ERPM to 100 ERPM before motor control. That
    // stays below the 200 ERPM moving threshold, so the expired idle timer
    // releases with 0 A instead of refreshing and applying parking duty again.
    assert_eq!(firmware.current_command_count(), 1);
    assert_eq!(firmware.duty_command_count(), 1);
    assert_f32_eq!(firmware.commanded_current().current().as_amps(), 0.0);
}

#[test]
fn running_limits_normal_current_from_motor_config_like_float_out_boy_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    for (motor_current, current_limit) in [(1.0_f32, 3.0_f32), (-1.0_f32, -2.0_f32)] {
        let expected_current = current_limit * output_alpha();
        let telemetry = FirmwareTest::new()
            .with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                TotalMotorCurrent::new(Current::from_amps(motor_current)),
                InputCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            )
            .with_motor_current_limits(
                MotorCurrentLimit::new(Current::from_amps(3.0)),
                MotorCurrentLimit::new(Current::from_amps(2.0)),
            );
        telemetry.set_imu_ready(true);
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::from_radians(0.0)),
            ImuPitch::new(AngleRadians::from_radians(0.0)),
            ImuYaw::new(AngleRadians::from_radians(0.0)),
        );
        let imu = telemetry.imu();
        let bindings = telemetry.motor();
        let payloads = sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
        );
        let base = payloads.base();
        let setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
            10.0 * motor_current.signum(),
        ));
        let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = FloatOutBoyAllDataBasePayload::new(
            FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            FloatOutBoyAllDataAttitude::new(
                FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            FloatOutBoyAllDataStatus::new(
                FloatOutBoyRideState::new(
                    FloatOutBoyRunState::Running,
                    FloatOutBoyMode::Normal,
                    FloatOutBoySetpointAdjustment::Centering,
                    FloatOutBoyStopCondition::None,
                ),
                base.status().beep_reason(),
            ),
            FloatOutBoyFootpadSample::new(
                Voltage::from_volts(0.0),
                Voltage::from_volts(0.0),
                FloatOutBoyFootpadState::None,
            ),
            setpoints,
            FloatOutBoyRealtimeBoosterTorque::new(
                crate::motor_torque::MotorTorque::from_newton_meters(0.0),
            ),
            base.motor(),
        );
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::from_groups(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config.edit_float_out_boy_config(|config| {
            assert!(config.set_kp2(vescpkg_rs::RateCurrentGain::new(0.0)));
        });
        config.edit_float_out_boy_config(|config| {
            assert!(config.set_ki(vescpkg_rs::IntegralCurrentGain::new(0.0)));
        });
        assert!(state.store_serialized_config(&config));

        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            lifecycle,
            telemetry.telemetry(),
            imu,
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
                FloatOutBoyAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(bindings));

        // Upstream `motor_data_update` caches `l_current_max` and
        // `fabsf(l_current_min)` at `third_party/float-out-boy/src/motor_data.c:90-91`; RUNNING uses
        // max while accelerating and min while braking at `third_party/float-out-boy/src/main.c:932-942`.
        assert!(
            (telemetry.commanded_current().current().as_amps() - expected_current).abs() < 0.0001,
            "{motor_current}: {:?}",
            telemetry.commanded_current()
        );
    }
}
