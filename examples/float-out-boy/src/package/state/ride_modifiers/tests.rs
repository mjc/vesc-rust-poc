use super::*;
use vescpkg_rs::prelude::{ElectricalSpeed, PidScale};

fn input() -> RideModifierInput {
    RideModifierInput {
        base_setpoint: AngleDegrees::ZERO,
        remote_setpoint: AngleDegrees::ZERO,
        balance_pitch: AngleDegrees::ZERO,
        motor_erpm: Rpm::from_revolutions_per_minute(3_000.0),
        filtered_current: Current::ZERO,
        motor_current: MotorCurrent::new(Current::ZERO),
        acceleration: Rpm::ZERO,
        darkride: false,
        wheelslip: FloatOutBoyWheelSlipState::None,
    }
}

#[test]
fn nose_angling_uses_measured_elapsed_time_after_a_delayed_iteration() {
    let config = FloatOutBoyConfigImage::defaults();
    let mut input = input();
    input.motor_erpm = Rpm::from_revolutions_per_minute(6_000.0);
    let mut nominal = RideModifierState::default();
    let mut delayed = RideModifierState::default();

    let nominal = nominal
        .advance_elapsed(&config, input, VescSeconds::from_seconds(0.002))
        .board()
        .angle();
    let delayed = delayed
        .advance_elapsed(&config, input, VescSeconds::from_seconds(0.004))
        .board()
        .angle();

    assert_eq!(delayed, nominal * 2.0);
}

#[test]
fn turn_tilt_uses_filtered_yaw_and_erpm_direction_like_float_out_boy() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_turn_tilt_strength(PidScale::new(5.0)));
    assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(10.0)));
    assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(1_000.0),
    )));

    let mut state = RideModifierState::default();
    for tick in 1..100 {
        let tick = i16::try_from(tick).unwrap_or(i16::MAX);
        state.aggregate_yaw(AngleDegrees::from_degrees(f32::from(tick) * 0.1));
        state.advance(&config, input());
    }
    state.aggregate_yaw(AngleDegrees::from_degrees(10.0));
    let setpoints = state.advance(&config, input());

    assert!(setpoints.turn_tilt().angle().is_positive());
    assert_eq!(setpoints.board().angle(), setpoints.turn_tilt().angle());
}

#[test]
fn disabling_turn_tilt_winds_down_an_existing_setpoint() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_turn_tilt_strength(PidScale::new(5.0)));
    assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(10.0)));
    assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(1_000.0),
    )));

    let mut state = RideModifierState::default();
    for tick in 1..100 {
        let tick = i16::try_from(tick).unwrap_or(i16::MAX);
        state.aggregate_yaw(AngleDegrees::from_degrees(f32::from(tick) * 0.1));
        state.advance(&config, input());
    }
    state.aggregate_yaw(AngleDegrees::from_degrees(10.0));
    let active = state.advance(&config, input()).turn_tilt().angle();
    assert!(active.is_positive());

    assert!(config.editor().set_turn_tilt_strength(PidScale::new(0.0)));
    let disabled = state.advance(&config, input()).turn_tilt().angle();

    assert!(disabled < active);
}

#[test]
fn brake_tilt_uses_balance_offset_while_regenerating_like_float_out_boy() {
    let mut config = FloatOutBoyConfigImage::defaults();
    assert!(config.editor().set_brake_tilt_strength(PidScale::new(10.0)));
    let mut state = RideModifierState::default();
    let setpoints = state.advance(
        &config,
        RideModifierInput {
            balance_pitch: AngleDegrees::from_degrees(5.0),
            motor_current: MotorCurrent::new(Current::from_amps(-5.0)),
            ..input()
        },
    );

    assert!(setpoints.brake_tilt().angle().is_positive());
}

#[test]
fn wheelslip_winds_down_and_aggregates_the_stronger_matching_torque_like_float_out_boy() {
    let mut state = RideModifierState {
        nose: AngleDegrees::from_degrees(1.0),
        turn: TurnTiltState {
            angle: SmoothAngle {
                setpoint: AngleDegrees::from_degrees(2.0),
                ..SmoothAngle::default()
            },
            ..TurnTiltState::default()
        },
        atr: AtrState {
            angle: SmoothAngle {
                target: AngleDegrees::from_degrees(4.0),
                setpoint: AngleDegrees::from_degrees(4.0),
                ..SmoothAngle::default()
            },
            ..AtrState::default()
        },
        brake: SmoothAngle {
            target: AngleDegrees::from_degrees(5.0),
            setpoint: AngleDegrees::from_degrees(5.0),
            ..SmoothAngle::default()
        },
        torque: SmoothAngle {
            setpoint: AngleDegrees::from_degrees(3.0),
            ..SmoothAngle::default()
        },
    };
    let config = FloatOutBoyConfigImage::defaults();

    let setpoints = state.advance(
        &config,
        RideModifierInput {
            wheelslip: FloatOutBoyWheelSlipState::Detected,
            ..input()
        },
    );

    assert_eq!(state.nose, AngleDegrees::from_degrees(1.0));
    assert_eq!(
        setpoints.turn_tilt().angle(),
        AngleDegrees::from_degrees(2.0 * 0.995)
    );
    assert_eq!(
        setpoints.torque_tilt().angle(),
        AngleDegrees::from_degrees(3.0 * 0.995)
    );
    assert_eq!(
        setpoints.atr().angle(),
        AngleDegrees::from_degrees(4.0 * 0.995)
    );
    assert_eq!(
        setpoints.brake_tilt().angle(),
        AngleDegrees::from_degrees(5.0 * 0.995)
    );
    assert_eq!(
        state.atr.angle.target,
        AngleDegrees::from_degrees(4.0 * 0.99)
    );
    assert_eq!(state.brake.target, AngleDegrees::from_degrees(5.0 * 0.99));
    assert_eq!(
        setpoints.board().angle(),
        AngleDegrees::from_degrees(1.0 + 2.0 * 0.995 + (4.0 + 5.0) * 0.995),
    );
}

#[test]
fn darkride_keeps_remote_tilt_but_suppresses_ride_modifiers_like_float_out_boy() {
    let mut state = RideModifierState {
        nose: AngleDegrees::from_degrees(1.0),
        turn: TurnTiltState {
            angle: SmoothAngle {
                setpoint: AngleDegrees::from_degrees(2.0),
                ..SmoothAngle::default()
            },
            ..TurnTiltState::default()
        },
        torque: SmoothAngle {
            setpoint: AngleDegrees::from_degrees(3.0),
            ..SmoothAngle::default()
        },
        ..RideModifierState::default()
    };
    let retained = state;
    let config = FloatOutBoyConfigImage::defaults();
    let setpoints = state.advance(
        &config,
        RideModifierInput {
            base_setpoint: AngleDegrees::from_degrees(4.0),
            remote_setpoint: AngleDegrees::from_degrees(-1.0),
            darkride: true,
            ..input()
        },
    );

    assert_eq!(state, retained);
    assert_eq!(setpoints.board().angle(), AngleDegrees::from_degrees(3.0));
    assert_eq!(setpoints.remote().angle(), AngleDegrees::from_degrees(-1.0));
    assert_eq!(
        state
            .runtime_setpoints(RideModifierInput {
                base_setpoint: AngleDegrees::from_degrees(4.0),
                remote_setpoint: AngleDegrees::from_degrees(-1.0),
                ..input()
            })
            .board()
            .angle(),
        AngleDegrees::from_degrees(9.0),
    );
}

#[test]
fn source_sign_treats_both_zero_encodings_as_nonnegative() {
    let positive = AngleDegrees::from_degrees(3.0);
    let positive_zero = AngleDegrees::from_degrees(0.0);
    let negative_zero = AngleDegrees::from_degrees(-0.0);

    assert!(same_source_sign(positive_zero, positive));
    assert!(same_source_sign(negative_zero, positive));
    assert_eq!(combine_torque_offsets(negative_zero, positive), positive,);
}

#[test]
fn nose_angling_covers_source_thresholds_limits_directions_and_winddown() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_tiltback_constant(AngleDegrees::from_degrees(2.0)));
    assert!(editor.set_tiltback_constant_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(500.0),
    )));
    assert!(editor.set_tiltback_variable(PidScale::new(1.0)));
    assert!(editor.set_tiltback_variable_max(AngleDegrees::from_degrees(3.0)));
    assert!(editor.set_tiltback_variable_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(1_000.0),
    )));
    assert!(
        editor.set_nose_angling_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),)
    );

    for (erpm, expected) in [
        (500.0, 0.0),
        (1_000.0, 2.0),
        (2_000.0, 3.0),
        (10_000.0, 5.0),
        (-2_000.0, -3.0),
        (-10_000.0, -5.0),
    ] {
        assert_eq!(
            nose_target(&config, Rpm::from_revolutions_per_minute(erpm)),
            AngleDegrees::from_degrees(expected),
        );
    }

    let mut state = RideModifierState::default();
    state.update_nose(
        &config,
        Rpm::from_revolutions_per_minute(10_000.0),
        VescSeconds::from_seconds(0.01),
    );
    assert!((state.nose.as_degrees() - 1.0).abs() < 0.000_001);
    state.update_nose(&config, Rpm::ZERO, VescSeconds::from_seconds(0.01));
    assert_eq!(state.nose, AngleDegrees::ZERO);
}

#[test]
fn zero_variable_nose_rate_stays_finite_instead_of_propagating_refloat_nan() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_tiltback_constant(AngleDegrees::from_degrees(2.0)));
    assert!(editor.set_tiltback_constant_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(500.0),
    )));
    assert!(editor.set_tiltback_variable(PidScale::new(0.0)));
    assert!(editor.set_tiltback_variable_max(AngleDegrees::ZERO));

    let target = nose_target(&config, Rpm::from_revolutions_per_minute(2_000.0));

    assert_eq!(target, AngleDegrees::from_degrees(2.0));
    assert!(target.as_degrees().is_finite());
}

#[test]
fn torque_tilt_covers_source_threshold_regen_limit_and_return() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_torque_tilt_start_current(MotorCurrent::new(Current::from_amps(10.0,))));
    assert!(editor.set_torque_tilt_strength(PidScale::new(0.1)));
    assert!(editor.set_torque_tilt_regen_strength(PidScale::new(0.2)));
    assert!(editor.set_torque_tilt_angle_limit(AngleDegrees::from_degrees(3.0)));
    assert!(
        editor
            .set_torque_tilt_on_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),)
    );
    assert!(
        editor
            .set_torque_tilt_off_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(50.0),)
    );
    let balance = config.balance();

    for (current, braking, expected) in [
        (5.0, false, 0.0),
        (20.0, false, 1.0),
        (100.0, false, 3.0),
        (-20.0, true, -2.0),
        (-100.0, true, -3.0),
    ] {
        assert_eq!(
            torque_target(balance, Current::from_amps(current), braking),
            AngleDegrees::from_degrees(expected),
        );
    }

    let mut state = RideModifierState::default();
    state.update_torque(
        balance,
        Current::from_amps(100.0),
        false,
        3_000.0,
        VescSeconds::from_seconds(0.01),
    );
    let active = state.torque.setpoint;
    assert!(active.is_positive());
    state.update_torque(
        balance,
        Current::ZERO,
        false,
        3_000.0,
        VescSeconds::from_seconds(0.01),
    );
    assert!(state.torque.setpoint < active);
}

#[test]
fn atr_covers_acceleration_speed_boost_braking_limit_and_recovery() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_atr_strength_up(PidScale::new(1.0)));
    assert!(editor.set_atr_strength_down(PidScale::new(1.0)));
    assert!(editor.set_atr_threshold_up(AngleDegrees::ZERO));
    assert!(editor.set_atr_threshold_down(AngleDegrees::ZERO));
    assert!(editor.set_atr_speed_boost(PidScale::new(0.5)));
    assert!(editor.set_atr_angle_limit(AngleDegrees::from_degrees(3.0)));
    assert!(editor.set_atr_on_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),));
    assert!(editor.set_atr_off_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(50.0),));
    assert!(editor.set_atr_amps_accel_ratio(PidScale::new(1.0)));
    assert!(editor.set_atr_amps_decel_ratio(PidScale::new(1.0)));
    let balance = config.balance();
    let mut state = RideModifierState::default();
    let accelerating = RideModifierInput {
        motor_erpm: Rpm::from_revolutions_per_minute(4_000.0),
        filtered_current: Current::from_amps(30.0),
        motor_current: MotorCurrent::new(Current::from_amps(30.0)),
        ..input()
    };

    for _ in 0..200 {
        state.update_atr(
            balance,
            accelerating,
            false,
            4_000.0,
            1.0,
            VescSeconds::from_seconds(0.01),
        );
    }
    let accelerating_setpoint = state.atr.angle.setpoint;
    assert!(accelerating_setpoint.is_positive());
    assert!(accelerating_setpoint <= AngleDegrees::from_degrees(3.0));
    assert!((state.atr.speed_boost - 1.0 / 7.0).abs() < 0.000_001);

    let braking = RideModifierInput {
        filtered_current: Current::from_amps(-30.0),
        motor_current: MotorCurrent::new(Current::from_amps(-30.0)),
        ..accelerating
    };
    for _ in 0..400 {
        state.update_atr(
            balance,
            braking,
            true,
            4_000.0,
            1.0,
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.atr.angle.setpoint.is_negative());
    assert!(state.atr.angle.setpoint >= AngleDegrees::from_degrees(-3.0));
    assert!(state.atr.speed_boost.abs() < f32::EPSILON);

    let before_recovery = state.atr.angle.setpoint;
    for _ in 0..1_000 {
        state.update_atr(
            balance,
            RideModifierInput {
                filtered_current: Current::from_amps(8.0),
                motor_current: MotorCurrent::new(Current::from_amps(8.0)),
                ..accelerating
            },
            false,
            4_000.0,
            1.0,
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.atr.angle.setpoint > before_recovery);
}

#[test]
fn brake_and_turn_tilt_cover_source_gates_saturation_direction_and_return() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_brake_tilt_strength(PidScale::new(10.0)));
    assert!(editor.set_brake_tilt_lingering(PidScale::new(2.0)));
    assert!(editor.set_turn_tilt_strength(PidScale::new(100.0)));
    assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(3.0)));
    assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
        Rpm::from_revolutions_per_minute(1_000.0),
    )));
    let balance = config.balance();
    let mut state = RideModifierState {
        turn: TurnTiltState {
            yaw: WrappedAngleMotion::from_parts(
                AngleDegrees::ZERO,
                AngleDegrees::from_degrees(0.1),
                AngleDegrees::from_degrees(20.0),
            ),
            ..TurnTiltState::default()
        },
        ..RideModifierState::default()
    };

    assert_eq!(
        turn_target(
            &state.turn,
            balance,
            Rpm::from_revolutions_per_minute(500.0),
        ),
        AngleDegrees::ZERO,
    );
    assert_eq!(
        turn_target(
            &state.turn,
            balance,
            Rpm::from_revolutions_per_minute(3_000.0),
        ),
        AngleDegrees::from_degrees(3.0),
    );
    assert_eq!(
        turn_target(
            &state.turn,
            balance,
            Rpm::from_revolutions_per_minute(-3_000.0),
        ),
        AngleDegrees::from_degrees(-3.0),
    );

    let braking = RideModifierInput {
        balance_pitch: AngleDegrees::from_degrees(5.0),
        motor_current: MotorCurrent::new(Current::from_amps(-5.0)),
        ..input()
    };
    for _ in 0..100 {
        state.update_brake(
            balance,
            braking,
            true,
            3_000.0,
            1.0,
            VescSeconds::from_seconds(0.01),
        );
    }
    let sustained = state.brake.setpoint;
    assert!(sustained.is_positive());
    for _ in 0..100 {
        state.update_brake(
            balance,
            RideModifierInput {
                motor_current: MotorCurrent::new(Current::ZERO),
                ..braking
            },
            false,
            3_000.0,
            1.0,
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.brake.setpoint < sustained);

    state.update_turn(
        balance,
        Rpm::from_revolutions_per_minute(3_000.0),
        VescSeconds::from_seconds(0.01),
    );
    let active_turn = state.turn.angle.setpoint;
    assert!(active_turn.is_positive());
    state.turn.yaw.clear_motion();
    state.update_turn(
        balance,
        Rpm::from_revolutions_per_minute(3_000.0),
        VescSeconds::from_seconds(0.01),
    );
    assert!(state.turn.angle.setpoint < active_turn);
}
