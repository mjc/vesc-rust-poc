use super::*;
use crate::motor_torque::{MotorTorque, MotorTorqueConstant, MotorTorqueLimit};
use vescpkg_rs::prelude::{AngleRadians, Rpm};

fn motor_current(current: Current) -> MotorCurrent {
    MotorCurrent::new(current)
}

fn motor_current_limit(current: Current) -> MotorCurrentLimit {
    MotorCurrentLimit::new(current)
}

fn motor_torque(current: Current) -> MotorTorque {
    MotorTorqueConstant::REFLOAT_COMPAT.torque_from_current(current)
}

fn motor_torque_limit(current: Current) -> MotorTorqueLimit {
    MotorTorqueLimit::new(motor_torque(current))
}

fn electrical_speed(speed: Rpm) -> ElectricalSpeed {
    ElectricalSpeed::new(speed)
}

fn setpoint(angle: AngleDegrees) -> FloatOutBoyRealtimeRuntimeSetpoint {
    FloatOutBoyRealtimeRuntimeSetpoint::new(angle)
}

fn roll(angle: AngleRadians) -> ImuRoll {
    ImuRoll::new(angle)
}

fn base_config() -> LoopConfig {
    LoopConfig {
        kp: AngleCurrentGain::new(0.0),
        kp2: RateCurrentGain::new(0.0),
        ki: IntegralCurrentGain::new(0.0),
        kp_brake: PidScale::new(1.0),
        kp2_brake: PidScale::new(1.0),
        ki_limit: motor_torque_limit(Current::from_amps(0.0)),
        booster_angle: AngleDegrees::from_degrees(0.0),
        booster_ramp: AngleDegrees::from_degrees(0.0),
        booster_torque: motor_torque(Current::from_amps(0.0)),
        brkbooster_angle: AngleDegrees::from_degrees(0.0),
        brkbooster_ramp: AngleDegrees::from_degrees(0.0),
        brkbooster_torque: motor_torque(Current::from_amps(0.0)),
        hertz: SampleRate::from_hertz(100.0),
    }
}

fn base_input() -> LoopInput {
    LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(0.0)),
        brake_tilt_setpoint: FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
            0.0,
        )),
        balance_pitch: AngleDegrees::from_degrees(0.0),
        raw_pitch: AngleDegrees::from_degrees(0.0),
        roll: roll(AngleRadians::from_radians(0.0)),
        gyro_pitch: AngularVelocity::from_degrees_per_second(0.0),
        gyro_yaw: AngularVelocity::from_degrees_per_second(0.0),
        motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(0.0)),
        motor_current: motor_current(Current::from_amps(1.0)),
        motor_current_max: motor_current_limit(Current::from_amps(100.0)),
        motor_current_min: motor_current_limit(Current::from_amps(100.0)),
        mode: FloatOutBoyMode::Normal,
        darkride: FloatOutBoyDarkRideState::Upright,
        traction_control: FloatOutBoyTractionControlState::FilteringCurrent,
    }
}

fn base_state() -> LoopState {
    LoopState {
        balance_current: motor_current(Current::from_amps(0.0)),
        booster_torque: motor_torque(Current::from_amps(0.0)),
        pid: PidState::default(),
        softstart_pid_limit: motor_current_limit(Current::from_amps(100.0)),
    }
}

fn assert_current(actual: MotorCurrent, expected: MotorCurrent) {
    assert!((actual.current().as_amps() - expected.current().as_amps()).abs() < 0.0001);
}

fn assert_current_limit(actual: MotorCurrentLimit, expected: MotorCurrentLimit) {
    assert!((actual.current().as_amps() - expected.current().as_amps()).abs() < 0.0001);
}

fn assert_torque(actual: MotorTorque, expected: MotorTorque) {
    assert!((actual.as_newton_meters() - expected.as_newton_meters()).abs() < 0.0001);
}

fn compatibility_amps(torque: MotorTorque) -> f32 {
    MotorTorqueConstant::REFLOAT_COMPAT
        .current_from_torque(torque)
        .as_amps()
}

fn assert_scale(actual: PidScale, expected: PidScale) {
    assert!((actual.value() - expected.value()).abs() < 0.0001);
}

#[test]
fn balance_loop_converts_compatibility_current_through_live_torque_constant() {
    let config = LoopConfig {
        kp: AngleCurrentGain::new(1.0),
        ..base_config()
    };
    let input = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
        motor_current_max: motor_current_limit(Current::from_amps(100.0)),
        ..base_input()
    };
    let compatibility = base_state().advance_balance_loop_elapsed(config, input, base_elapsed());
    let output = base_state().advance_balance_loop_elapsed_with_torque(
        config,
        input,
        base_elapsed(),
        MotorTorqueConstant::from_firmware_config(
            vescpkg_rs::prelude::FocMotorFluxLinkage::new(
                vescpkg_rs::prelude::FluxLinkage::from_webers(0.004),
            ),
            vescpkg_rs::prelude::MotorPoleCount::try_new(14).ok(),
        ),
    );

    let expected = MotorTorqueConstant::REFLOAT_COMPAT.newton_meters_per_amp() / 0.042
        * compatibility.requested_current.current().as_amps();
    assert_f32_eq!(output.requested_current.current().as_amps(), expected);
}

fn live_motor_torque_constant() -> MotorTorqueConstant {
    MotorTorqueConstant::from_firmware_config(
        vescpkg_rs::prelude::FocMotorFluxLinkage::new(
            vescpkg_rs::prelude::FluxLinkage::from_webers(0.004),
        ),
        vescpkg_rs::prelude::MotorPoleCount::try_new(14).ok(),
    )
}

#[test]
fn balance_loop_clamps_non_compatibility_current_in_live_domain() {
    let output = base_state().advance_balance_loop_elapsed_with_torque(
        LoopConfig {
            kp: AngleCurrentGain::new(100.0),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
            motor_current_max: motor_current_limit(Current::from_amps(10.0)),
            motor_current_min: motor_current_limit(Current::from_amps(10.0)),
            ..base_input()
        },
        VescSeconds::from_seconds(1.0),
        live_motor_torque_constant(),
    );

    assert!(output.requested_current.current().as_amps().abs() <= 10.0001);
}

#[test]
fn balance_loop_softstart_uses_live_ramp_for_non_compatibility_motor() {
    let torque_constant = live_motor_torque_constant();
    let output = LoopState {
        softstart_pid_limit: motor_current_limit(Current::ZERO),
        ..base_state()
    }
    .advance_balance_loop_elapsed_with_torque(
        base_config(),
        base_input(),
        VescSeconds::from_seconds(0.004),
        torque_constant,
    );

    assert_current_limit(
        output.state.softstart_pid_limit,
        motor_current_limit(Current::from_amps(0.4)),
    );
}

#[test]
fn balance_loop_softstart_uses_acceleration_limit_while_braking() {
    let torque_constant = live_motor_torque_constant();
    let initial_limit = motor_current_limit(Current::from_amps(2.0));
    let output = LoopState {
        softstart_pid_limit: initial_limit,
        ..base_state()
    }
    .advance_balance_loop_elapsed_with_torque(
        base_config(),
        LoopInput {
            motor_current: motor_current(Current::from_amps(-1.0)),
            motor_current_max: motor_current_limit(Current::from_amps(80.0)),
            motor_current_min: motor_current_limit(Current::from_amps(2.0)),
            ..base_input()
        },
        VescSeconds::from_seconds(0.004),
        torque_constant,
    );

    assert_current_limit(
        output.state.softstart_pid_limit,
        motor_current_limit(Current::from_amps(2.4)),
    );
}

fn advance_loop(config: LoopConfig, input: LoopInput, state: LoopState) -> LoopOutput {
    state.advance_balance_loop(config, input)
}

fn base_elapsed() -> VescSeconds {
    base_config()
        .hertz
        .sample_period()
        .expect("positive test rate")
}

fn alpha(cutoff_hertz: f32) -> f32 {
    crate::ema::EmaAlpha::from_elapsed(
        vescpkg_rs::Frequency::from_hertz(cutoff_hertz),
        base_elapsed(),
    )
    .factor()
}

#[test]
fn balance_loop_softstart_uses_measured_elapsed_time() {
    let state = LoopState {
        softstart_pid_limit: motor_current_limit(Current::ZERO),
        ..base_state()
    };

    let output = state.advance_balance_loop_elapsed(
        base_config(),
        base_input(),
        VescSeconds::from_seconds(0.004),
    );

    assert_current_limit(
        output.state.softstart_pid_limit,
        motor_current_limit(Current::from_amps(0.4)),
    );
}

#[test]
fn pid_integral_matches_over_equal_elapsed_time_at_different_cadences() {
    let config = LoopConfig {
        ki: IntegralCurrentGain::new(1.0),
        ki_limit: motor_torque_limit(Current::ZERO),
        ..base_config()
    };
    let input = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
        ..base_input()
    };
    let mut fast = base_state();
    let mut slow = base_state();

    for _ in 0..100 {
        fast = fast
            .advance_balance_loop_elapsed(config, input, VescSeconds::from_seconds(0.01))
            .state;
    }
    for _ in 0..50 {
        slow = slow
            .advance_balance_loop_elapsed(config, input, VescSeconds::from_seconds(0.02))
            .state;
    }

    assert!(
        (compatibility_amps(fast.pid.integral_torque)
            - compatibility_amps(slow.pid.integral_torque))
        .abs()
            < 0.001
    );
    assert!((compatibility_amps(fast.pid.integral_torque) - 720.0).abs() < 0.001);
}

#[test]
fn balance_loop_unit_updates_pid_scales_by_erpm_direction_like_float_out_boy_pid() {
    let config = LoopConfig {
        kp_brake: PidScale::new(2.0),
        kp2_brake: PidScale::new(3.0),
        ..base_config()
    };
    let state = base_state();
    let elapsed = base_elapsed();

    let coasting = state.with_updated_pid_state(
        config,
        electrical_speed(Rpm::from_revolutions_per_minute(0.0)),
        state.pid.integral_torque,
        elapsed,
    );
    let forward = state.with_updated_pid_state(
        config,
        electrical_speed(Rpm::from_revolutions_per_minute(1000.0)),
        state.pid.integral_torque,
        elapsed,
    );
    let reverse = state.with_updated_pid_state(
        config,
        electrical_speed(Rpm::from_revolutions_per_minute(-1000.0)),
        state.pid.integral_torque,
        elapsed,
    );

    assert_scale(coasting.pid.kp_brake_scale, PidScale::new(1.0));
    assert_scale(coasting.pid.kp2_brake_scale, PidScale::new(1.0));
    assert_scale(coasting.pid.kp_accel_scale, PidScale::new(1.0));
    assert_scale(coasting.pid.kp2_accel_scale, PidScale::new(1.0));

    assert_scale(forward.pid.kp_brake_scale, PidScale::new(1.0 + alpha(1.0)));
    assert_scale(
        forward.pid.kp2_brake_scale,
        PidScale::new(1.0 + 2.0 * alpha(1.0)),
    );
    assert_scale(forward.pid.kp_accel_scale, PidScale::new(1.0));
    assert_scale(forward.pid.kp2_accel_scale, PidScale::new(1.0));

    assert_scale(reverse.pid.kp_brake_scale, PidScale::new(1.0));
    assert_scale(reverse.pid.kp2_brake_scale, PidScale::new(1.0));
    assert_scale(reverse.pid.kp_accel_scale, PidScale::new(1.0 + alpha(1.0)));
    assert_scale(
        reverse.pid.kp2_accel_scale,
        PidScale::new(1.0 + 2.0 * alpha(1.0)),
    );
}

#[test]
fn balance_loop_unit_persists_pid_integral_across_ticks_like_float_out_boy_pid() {
    let config = LoopConfig {
        ki: IntegralCurrentGain::new(1.0),
        ki_limit: motor_torque_limit(Current::from_amps(100.0)),
        ..base_config()
    };
    let input = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
        ..base_input()
    };

    let first = advance_loop(config, input, base_state());
    let second = advance_loop(config, input, first.state);

    assert_torque(
        second.state.pid.integral_torque,
        motor_torque(Current::from_amps(14.4)),
    );
}

#[test]
fn balance_loop_preserves_multisample_pid_trajectory() {
    let config = LoopConfig {
        kp: AngleCurrentGain::new(1.25),
        kp2: RateCurrentGain::new(0.35),
        ki: IntegralCurrentGain::new(0.075),
        kp_brake: PidScale::new(1.8),
        kp2_brake: PidScale::new(0.65),
        ki_limit: motor_torque_limit(Current::from_amps(50.0)),
        ..base_config()
    };
    let inputs = [
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(4.25)),
            balance_pitch: AngleDegrees::from_degrees(1.5),
            roll: roll(AngleRadians::from_radians(0.3)),
            gyro_pitch: AngularVelocity::from_degrees_per_second(12.5),
            gyro_yaw: AngularVelocity::from_degrees_per_second(-4.0),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(1_200.0)),
            ..base_input()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(-2.0)),
            balance_pitch: AngleDegrees::from_degrees(0.75),
            roll: roll(AngleRadians::from_radians(-0.4)),
            gyro_pitch: AngularVelocity::from_degrees_per_second(-6.0),
            gyro_yaw: AngularVelocity::from_degrees_per_second(8.0),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(-1_500.0)),
            ..base_input()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
            balance_pitch: AngleDegrees::from_degrees(-0.25),
            roll: roll(AngleRadians::from_radians(0.7)),
            gyro_pitch: AngularVelocity::from_degrees_per_second(3.0),
            gyro_yaw: AngularVelocity::from_degrees_per_second(9.0),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(250.0)),
            ..base_input()
        },
    ];
    let mut state = base_state();
    let actual = inputs.map(|input| {
        let output = advance_loop(config, input, state);
        state = output.state;
        [
            output.requested_current.current().as_amps(),
            compatibility_amps(state.pid.integral_torque),
            state.pid.kp_brake_scale.value(),
            state.pid.kp2_brake_scale.value(),
            state.pid.kp_accel_scale.value(),
            state.pid.kp2_accel_scale.value(),
        ]
    });

    // Refloat stores PID terms as torque and converts them back to current in
    // `pid_control`; pin the resulting f32 round-trip instead of current-only math.
    assert_eq!(
        actual.map(|sample| sample.map(f32::to_bits)),
        [
            [
                1_056_857_588,
                1_069_421_691,
                1_065_761_627,
                1_064_995_857,
                1_065_353_216,
                1_065_353_216,
            ],
            [
                995_960_576,
                0,
                1_065_736_772,
                1_065_017_605,
                1_065_761_627,
                1_064_995_857,
            ],
            [
                1_033_286_027,
                1_059_900_620,
                1_065_713_430,
                1_065_038_030,
                1_065_736_772,
                1_065_017_605,
            ],
        ],
    );
}

#[test]
fn balance_loop_unit_zero_ki_limit_keeps_integrating_like_float_out_boy_pid() {
    let config = LoopConfig {
        ki: IntegralCurrentGain::new(1.0),
        ki_limit: motor_torque_limit(Current::ZERO),
        ..base_config()
    };
    let positive = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(2.0)),
        ..base_input()
    };
    let negative = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(-3.0)),
        ..base_input()
    };

    let first = advance_loop(config, positive, base_state());
    let second = advance_loop(config, positive, first.state);
    let reversed = advance_loop(config, negative, second.state);

    assert_torque(
        first.state.pid.integral_torque,
        motor_torque(Current::from_amps(14.4)),
    );
    assert_torque(
        second.state.pid.integral_torque,
        motor_torque(Current::from_amps(28.8)),
    );
    assert_torque(
        reversed.state.pid.integral_torque,
        motor_torque(Current::from_amps(7.2)),
    );
}

#[test]
fn balance_loop_unit_limits_normal_current_like_float_out_boy_main_loop() {
    let config = LoopConfig {
        kp: AngleCurrentGain::new(10.0),
        ..base_config()
    };
    let cases = [
        (
            motor_current(Current::from_amps(1.0)),
            setpoint(AngleDegrees::from_degrees(10.0)),
            motor_current(Current::from_amps(3.0)),
            motor_current(Current::from_amps(3.0 * alpha(25.0))),
        ),
        (
            motor_current(Current::from_amps(-1.0)),
            setpoint(AngleDegrees::from_degrees(-10.0)),
            motor_current(Current::from_amps(2.0)),
            motor_current(Current::from_amps(-2.0 * alpha(25.0))),
        ),
    ];

    for (measured_current, board_setpoint, current_limit, expected_current) in cases {
        let output = advance_loop(
            config,
            LoopInput {
                setpoint: board_setpoint,
                motor_current: measured_current,
                motor_current_max: motor_current_limit(Current::from_amps(3.0)),
                motor_current_min: motor_current_limit(current_limit.current()),
                ..base_input()
            },
            base_state(),
        );

        // Upstream `pid_update` computes P/I at
        // `third_party/float-out-boy/src/pid.c:40-46`; RUNNING selects max
        // or min current limit at `third_party/float-out-boy/src/main.c:932-942`
        // and smooths at `third_party/float-out-boy/src/main.c:949-954`.
        assert_current(output.state.balance_current, expected_current);
    }
}

#[test]
fn balance_loop_unit_treats_motor_current_min_as_magnitude_like_float_out_boy_main_loop() {
    let output = advance_loop(
        LoopConfig {
            kp: AngleCurrentGain::new(10.0),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(-10.0)),
            motor_current: motor_current(Current::from_amps(-1.0)),
            motor_current_max: motor_current_limit(Current::from_amps(100.0)),
            motor_current_min: motor_current_limit(Current::from_amps(-2.0)),
            ..base_input()
        },
        base_state(),
    );

    // Upstream treats `current_limit` as a positive scalar before clamping
    // `new_current` at `third_party/float-out-boy/src/main.c:932-942`, even
    // though VESC stores braking current as a negative config value.
    assert_current(
        output.requested_current,
        motor_current(Current::from_amps(-2.0 * alpha(25.0))),
    );
}

#[test]
fn balance_loop_unit_clamps_to_a_zero_firmware_current_limit() {
    let output = advance_loop(
        LoopConfig {
            kp: AngleCurrentGain::new(10.0),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(10.0)),
            motor_current_max: motor_current_limit(Current::ZERO),
            ..base_input()
        },
        base_state(),
    );

    assert_eq!(output.requested_current.current(), Current::ZERO);
}

#[test]
fn balance_loop_unit_positive_pitch_rate_commands_negative_damping_current() {
    let output = advance_loop(
        LoopConfig {
            kp2: RateCurrentGain::new(2.0),
            ..base_config()
        },
        LoopInput {
            gyro_pitch: AngularVelocity::from_degrees_per_second(10.0),
            ..base_input()
        },
        base_state(),
    );

    // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
    // `third_party/float-out-boy/src/pid.c:66-72`; RUNNING smooths the requested
    // current at `third_party/float-out-boy/src/main.c:949-954`.
    assert_current(
        output.requested_current,
        motor_current(Current::from_amps(-20.0 * alpha(25.0))),
    );
}

#[test]
fn balance_loop_unit_negative_pitch_rate_commands_positive_damping_current() {
    let output = advance_loop(
        LoopConfig {
            kp2: RateCurrentGain::new(2.0),
            ..base_config()
        },
        LoopInput {
            gyro_pitch: AngularVelocity::from_degrees_per_second(-10.0),
            ..base_input()
        },
        base_state(),
    );

    // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
    // `third_party/float-out-boy/src/pid.c:66-72`; RUNNING smooths the requested
    // current at `third_party/float-out-boy/src/main.c:949-954`.
    assert_current(
        output.requested_current,
        motor_current(Current::from_amps(20.0 * alpha(25.0))),
    );
}

#[test]
fn balance_loop_unit_filters_booster_and_softstart_like_float_out_boy_main_loop() {
    let output = advance_loop(
        LoopConfig {
            booster_angle: AngleDegrees::from_degrees(1.0),
            booster_ramp: AngleDegrees::from_degrees(1.0),
            booster_torque: motor_torque(Current::from_amps(20.0)),
            brkbooster_angle: AngleDegrees::from_degrees(1.0),
            brkbooster_ramp: AngleDegrees::from_degrees(1.0),
            brkbooster_torque: motor_torque(Current::from_amps(20.0)),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(3.0)),
            motor_current: motor_current(Current::from_amps(1.0)),
            motor_current_max: motor_current_limit(Current::from_amps(3.0)),
            motor_current_min: motor_current_limit(Current::from_amps(2.0)),
            ..base_input()
        },
        LoopState {
            softstart_pid_limit: motor_current_limit(Current::from_amps(0.0)),
            ..base_state()
        },
    );

    // Upstream `booster_update` ramps/filter current at
    // `third_party/float-out-boy/src/booster.c:63-75`; RUNNING soft-start clamps
    // pitch-based current and increments the limit at
    // `third_party/float-out-boy/src/main.c:924-930`.
    assert_torque(
        output.state.booster_torque,
        motor_torque(Current::from_amps(20.0 * alpha(1.0))),
    );
    assert_current(
        output.state.balance_current,
        motor_current(Current::from_amps(0.0)),
    );
    assert_current(
        output.requested_current,
        motor_current(Current::from_amps(0.0)),
    );
    assert_current_limit(
        output.state.softstart_pid_limit,
        motor_current_limit(Current::from_amps(1.0)),
    );
}

#[test]
fn balance_loop_unit_booster_proportional_subtracts_raw_pitch_like_float_out_boy_main_loop() {
    let output = advance_loop(
        LoopConfig {
            booster_angle: AngleDegrees::ZERO,
            booster_ramp: AngleDegrees::from_degrees(1.0),
            booster_torque: motor_torque(Current::from_amps(20.0)),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(5.0)),
            brake_tilt_setpoint: setpoint(AngleDegrees::from_degrees(2.0)),
            raw_pitch: AngleDegrees::from_degrees(3.0),
            ..base_input()
        },
        base_state(),
    );

    // Upstream subtracts brake tilt and raw pitch from booster proportional before
    // `booster_update` at `third_party/float-out-boy/src/main.c:921-922`.
    assert_torque(
        output.state.booster_torque,
        motor_torque(Current::from_amps(0.0)),
    );
}

#[test]
fn balance_loop_unit_booster_subtracts_brake_tilt_like_float_out_boy_main_loop() {
    let output = advance_loop(
        LoopConfig {
            booster_angle: AngleDegrees::from_degrees(0.0),
            booster_ramp: AngleDegrees::from_degrees(1.0),
            booster_torque: motor_torque(Current::from_amps(20.0)),
            brkbooster_angle: AngleDegrees::from_degrees(0.0),
            brkbooster_ramp: AngleDegrees::from_degrees(1.0),
            brkbooster_torque: motor_torque(Current::from_amps(20.0)),
            ..base_config()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(5.0)),
            brake_tilt_setpoint: FloatOutBoyRealtimeRuntimeSetpoint::new(
                AngleDegrees::from_degrees(5.0),
            ),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(1000.0)),
            motor_current: motor_current(Current::from_amps(1.0)),
            ..base_input()
        },
        base_state(),
    );

    // Upstream subtracts brake tilt from booster proportional before
    // `booster_update` at `third_party/float-out-boy/src/main.c:921-922`.
    assert_torque(
        output.state.booster_torque,
        motor_torque(Current::from_amps(0.0)),
    );
    assert_current(
        output.requested_current,
        motor_current(Current::from_amps(0.0)),
    );
}

#[test]
fn booster_profile_deadbands_ramps_and_saturates_like_float_out_boy_booster() {
    let config = LoopConfig {
        booster_torque: motor_torque(Current::from_amps(20.0)),
        booster_angle: AngleDegrees::from_degrees(1.0),
        booster_ramp: AngleDegrees::from_degrees(2.0),
        ..base_config()
    };
    let target = |angle| {
        Branch::Accel.target_torque(
            config,
            electrical_speed(Rpm::ZERO),
            Proportional::new(AngleDegrees::from_degrees(angle)),
        )
    };

    assert_torque(target(0.5), motor_torque(Current::from_amps(0.0)));
    assert_torque(target(2.0), motor_torque(Current::from_amps(10.0)));
    assert_torque(target(-2.0), motor_torque(Current::from_amps(-10.0)));
    assert_torque(target(4.0), motor_torque(Current::from_amps(20.0)));
}

#[test]
fn balance_loop_preserves_multisample_booster_trajectory() {
    let config = LoopConfig {
        booster_angle: AngleDegrees::from_degrees(2.0),
        booster_ramp: AngleDegrees::from_degrees(4.0),
        booster_torque: motor_torque(Current::from_amps(20.0)),
        brkbooster_angle: AngleDegrees::from_degrees(3.0),
        brkbooster_ramp: AngleDegrees::from_degrees(5.0),
        brkbooster_torque: motor_torque(Current::from_amps(15.0)),
        ..base_config()
    };
    let inputs = [
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(3.0)),
            motor_current: motor_current(Current::from_amps(1.0)),
            ..base_input()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(5.0)),
            motor_current: motor_current(Current::from_amps(1.0)),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(8_000.0)),
            ..base_input()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(-5.0)),
            motor_current: motor_current(Current::from_amps(-1.0)),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(8_000.0)),
            ..base_input()
        },
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(-10.0)),
            motor_current: motor_current(Current::from_amps(-1.0)),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(15_000.0)),
            ..base_input()
        },
    ];
    let mut state = base_state();
    let actual = inputs.map(|input| {
        let output = advance_loop(config, input, state);
        state = output.state;
        compatibility_amps(state.booster_torque).to_bits()
    });

    // Refloat filters booster torque before converting pitch demand to current;
    // pin that torque-domain trajectory including its f32 rounding.
    assert_eq!(
        actual,
        [1_050_397_660, 1_068_721_242, 1_061_469_059, 3_213_709_451]
    );
}

#[test]
fn balance_loop_unit_pitch_rate_mixes_axes_and_darkride_like_float_out_boy_imu() {
    let upright = PitchRate::from_imu(
        roll(AngleRadians::from_radians(0.0)),
        AngularVelocity::from_degrees_per_second(12.0),
        AngularVelocity::from_degrees_per_second(100.0),
        FloatOutBoyDarkRideState::Upright,
    );
    let darkride = PitchRate::from_imu(
        roll(AngleRadians::from_radians(0.0)),
        AngularVelocity::from_degrees_per_second(12.0),
        AngularVelocity::from_degrees_per_second(100.0),
        FloatOutBoyDarkRideState::Active,
    );

    // Upstream mixes roll, gyro Y, and gyro Z at
    // `third_party/float-out-boy/src/imu.c:46-51`, then flips darkride at
    // `third_party/float-out-boy/src/imu.c:52-53`.
    assert_f32_eq!(upright.rate().as_degrees_per_second(), 12.0);
    assert_f32_eq!(darkride.rate().as_degrees_per_second(), -12.0);
}

#[test]
fn balance_loop_unit_darkride_and_traction_control_match_float_out_boy_main_loop() {
    let config = LoopConfig {
        kp: AngleCurrentGain::new(1.0),
        ..base_config()
    };
    let base_input = LoopInput {
        setpoint: setpoint(AngleDegrees::from_degrees(10.0)),
        darkride: FloatOutBoyDarkRideState::Active,
        ..base_input()
    };
    let state = LoopState {
        balance_current: motor_current(Current::from_amps(10.0)),
        ..base_state()
    };

    let darkride_output = advance_loop(config, base_input, state);
    let traction_output = advance_loop(
        config,
        LoopInput {
            traction_control: FloatOutBoyTractionControlState::Freewheeling,
            ..base_input
        },
        state,
    );

    // Upstream RUNNING flips darkride current at
    // `third_party/float-out-boy/src/main.c:944-946`; traction control freewheels
    // at `third_party/float-out-boy/src/main.c:949-954`.
    assert_current(
        darkride_output.state.balance_current,
        motor_current(Current::from_amps(10.0 - 20.0 * alpha(25.0))),
    );
    assert_current(
        traction_output.state.balance_current,
        motor_current(Current::from_amps(0.0)),
    );
}
