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
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRideState, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

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
    assert_eq!(
        motor.commanded_current_off_delay().duration().as_seconds(),
        0.05
    );
    assert_eq!(motor.current_command_count(), 1);
    assert_eq!(motor.commanded_current().current().as_amps(), 6.25);
    assert!(!state.apply_requested_motor_current(bindings));
    assert_eq!(motor.current_command_count(), 1);
}

#[test]
fn running_limits_normal_current_from_motor_config_like_float_out_boy_loop() {
    let lifecycle = TimestampTicks::from_ticks(0);
    for (motor_current, expected_current) in [(1.0_f32, 0.6_f32), (-1.0_f32, -0.4_f32)] {
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
            FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config.edit_float_out_boy_config(|config| {
            assert!(config.set_kp2(vescpkg_rs::RateCurrentGain::new(0.0)))
        });
        config.edit_float_out_boy_config(|config| {
            assert!(config.set_ki(vescpkg_rs::IntegralCurrentGain::new(0.0)))
        });
        assert!(state.store_serialized_config(&config));

        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            lifecycle,
            telemetry.telemetry(),
            imu,
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
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
