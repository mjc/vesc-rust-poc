use super::float_out_boy_imu_callback_with_state;
use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState};
use crate::package::FloatOutBoyPackageState;
use crate::package::test_support::{
    balance_filter_with_pitch, edit_config, imu_accel_x, imu_accel_y, imu_accel_z,
    imu_acceleration, imu_angular_rate, imu_period, imu_pitch_rate, imu_read_sample, imu_roll_rate,
    imu_yaw_rate, sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn imu_read_handler_updates_float_out_boy_balance_filter() {
    let telemetry = FirmwareTest::new().with_motor_current_limits(
        MotorCurrentLimit::new(Current::from_amps(40.0)),
        MotorCurrentLimit::new(Current::from_amps(40.0)),
    );
    telemetry.set_imu_ready(true);
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    ));
    edit_config(&mut state, |config| {
        assert!(config.set_kp(AngleCurrentGain::new(10.0)));
        assert!(config.set_kp2(RateCurrentGain::new(0.0)));
        assert!(config.set_ki(IntegralCurrentGain::new(0.0)));
        assert!(config.set_booster_current(MotorCurrent::new(Current::ZERO)));
    });
    state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_degrees(5.0)));
    state.refresh_motor_config_runtime_state(telemetry.telemetry());

    <super::FloatOutBoyImuRead as vescpkg_rs::ImuReadHandler>::read(
        &mut state,
        imu_read_sample(
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(0.0)),
                imu_accel_y(AccelerationG::from_g(0.0)),
                imu_accel_z(AccelerationG::from_g(1.0)),
            ),
            imu_angular_rate(
                imu_roll_rate(AngularVelocity::from_degrees_per_second(0.0)),
                imu_pitch_rate(AngularVelocity::from_degrees_per_second(1.0)),
                imu_yaw_rate(AngularVelocity::from_degrees_per_second(0.0)),
            ),
            imu_period(VescSeconds::from_seconds(0.1)),
        ),
    );
    assert_eq!(telemetry.current_command_count(), 1);
    assert!(!telemetry.commanded_current().current().is_zero());

    // C map: `imu_ref_callback` applies each sample to the balance filter,
    // runs PID, and applies motor control before returning.
    assert!(
        state
            .all_data_payloads()
            .base()
            .attitude()
            .balance_pitch()
            .angle()
            .as_radians()
            > 0.0
    );
}

#[test]
fn imu_callback_state_update_feeds_normal_balance_pitch_like_float_out_boy_loop() {
    let telemetry = FirmwareTest::new();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    ));

    float_out_boy_imu_callback_with_state(
        &mut state,
        imu_read_sample(
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(0.0)),
                imu_accel_y(AccelerationG::from_g(0.0)),
                imu_accel_z(AccelerationG::from_g(1.0)),
            ),
            imu_angular_rate(
                imu_roll_rate(AngularVelocity::from_degrees_per_second(0.0)),
                imu_pitch_rate(AngularVelocity::from_degrees_per_second(1.0)),
                imu_yaw_rate(AngularVelocity::from_degrees_per_second(0.0)),
            ),
            imu_period(VescSeconds::from_seconds(0.1)),
        ),
    );
    state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));

    // Upstream `imu_ref_callback` updates the balance filter at
    // `third_party/float-out-boy/src/main.c:760-765`; the main loop copies that
    // filter into `imu.balance_pitch` at `third_party/float-out-boy/src/imu.c:35-41`
    // before RUNNING PID reads it at `third_party/float-out-boy/src/pid.c:40`.
    assert!(
        state
            .all_data_payloads()
            .base()
            .attitude()
            .balance_pitch()
            .angle()
            .as_radians()
            > 0.0
    );
}
