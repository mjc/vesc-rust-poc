use super::*;
use vescpkg_rs::prelude::{Current, ElectricalSpeed, PidScale};

fn degrees(value: f32) -> AngleDegrees {
    AngleDegrees::from_degrees(value)
}

fn rpm(value: f32) -> Rpm {
    Rpm::from_revolutions_per_minute(value)
}

fn amps(value: f32) -> Current {
    Current::from_amps(value)
}

fn nominal_elapsed() -> VescSeconds {
    VescSeconds::from_seconds(1.0 / LOOP_RATE_COMPAT.as_hertz())
}

fn nominal_rate() -> SampleRate {
    LOOP_RATE_COMPAT
}

fn compat_torque(current_amps: f32) -> MotorTorque {
    MotorTorqueConstant::REFLOAT_COMPAT.torque_from_current(amps(current_amps))
}

fn input() -> RideModifierInput {
    RideModifierInput {
        base_setpoint: AngleDegrees::ZERO,
        remote_setpoint: AngleDegrees::ZERO,
        balance_pitch: AngleDegrees::ZERO,
        motor_erpm: rpm(3_000.0),
        filtered_torque: MotorTorque::ZERO,
        motor_current: MotorCurrent::new(Current::ZERO),
        acceleration: ElectricalAcceleration::ZERO,
        darkride: false,
        wheelslip: FloatOutBoyWheelSlipState::None,
    }
}

fn active_turn_tilt() -> (FloatOutBoyConfigImage, RideModifierState) {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_turn_tilt_strength(PidScale::new(5.0)));
    assert!(editor.set_turn_tilt_angle_limit(degrees(10.0)));
    assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(rpm(1_000.0))));

    let mut state = RideModifierState::default();
    for tick in 1..100 {
        let tick = i16::try_from(tick).unwrap_or(i16::MAX);
        state.aggregate_yaw(
            degrees(f32::from(tick) * 0.1),
            nominal_elapsed(),
            nominal_rate(),
        );
        state.advance(&config, input());
    }
    state.aggregate_yaw(degrees(10.0), nominal_elapsed(), nominal_rate());
    (config, state)
}

#[test]
fn turn_tilt_uses_filtered_yaw_and_erpm_direction_like_float_out_boy() {
    let (config, mut state) = active_turn_tilt();
    let setpoints = state.advance(&config, input());

    assert!(setpoints.turn_tilt().angle().is_positive());
    assert_eq!(setpoints.board().angle(), setpoints.turn_tilt().angle());
}

#[test]
fn turn_tilt_yaw_rate_matches_over_equal_time_at_different_cadences() {
    let mut fast = RideModifierState::default();
    let mut slow = RideModifierState::default();

    for step in 1..=50_i16 {
        fast.aggregate_yaw(
            degrees(f32::from(step) * 0.1),
            VescSeconds::from_seconds(0.002),
            SampleRate::from_hertz(500.0),
        );
    }
    for step in 1..=25_i16 {
        slow.aggregate_yaw(
            degrees(f32::from(step) * 0.2),
            VescSeconds::from_seconds(0.004),
            SampleRate::from_hertz(250.0),
        );
    }

    assert!(
        (fast.turn.yaw.rate.as_degrees_per_second() - slow.turn.yaw.rate.as_degrees_per_second())
            .abs()
            < 0.01
    );
    assert!((fast.turn.yaw.aggregate - slow.turn.yaw.aggregate).abs() < degrees(0.01));
}

#[test]
fn turn_tilt_preserves_yaw_direction_across_positive_to_negative_wrap() {
    let mut turn = TurnTiltState {
        yaw: YawMotion {
            last: degrees(179.95),
            ..YawMotion::default()
        },
        ..TurnTiltState::default()
    };

    turn.aggregate(degrees(-179.95), nominal_elapsed(), nominal_rate());

    let alpha = crate::ema::EmaAlpha::from_sample_rate(TURN_TILT_YAW_CUTOFF, nominal_rate());
    assert!((turn.yaw.rate.as_degrees_per_second() - 72.0 * alpha.factor()).abs() < 0.000_1);
}

#[test]
fn turn_tilt_filters_zero_yaw_change_instead_of_replaying_stale_motion() {
    let mut turn = TurnTiltState {
        yaw: YawMotion {
            last: degrees(10.0),
            rate: AngularVelocity::from_degrees_per_second(72.0),
            ..YawMotion::default()
        },
        ..TurnTiltState::default()
    };

    turn.aggregate(degrees(10.0), nominal_elapsed(), nominal_rate());

    let alpha = crate::ema::EmaAlpha::from_sample_rate(TURN_TILT_YAW_CUTOFF, nominal_rate());
    assert!((turn.yaw.rate.as_degrees_per_second() - 72.0 * alpha.retained()).abs() < 0.000_1);
}

#[test]
fn disabling_turn_tilt_preserves_an_existing_setpoint_like_refloat() {
    let (mut config, mut state) = active_turn_tilt();
    let active = state.advance(&config, input()).turn_tilt().angle();
    assert!(active.is_positive());

    assert!(config.editor().set_turn_tilt_strength(PidScale::new(0.0)));
    let disabled = state.advance(&config, input()).turn_tilt().angle();

    assert_eq!(disabled, active);
}

#[test]
fn brake_tilt_uses_balance_offset_while_regenerating_like_float_out_boy() {
    let mut config = FloatOutBoyConfigImage::defaults();
    assert!(config.editor().set_brake_tilt_strength(PidScale::new(10.0)));
    let mut state = RideModifierState::default();
    let setpoints = state.advance(
        &config,
        RideModifierInput {
            balance_pitch: degrees(5.0),
            motor_current: MotorCurrent::new(amps(-5.0)),
            ..input()
        },
    );

    assert!(setpoints.brake_tilt().angle().is_positive());
}

#[test]
fn wheelslip_winds_down_and_aggregates_the_stronger_matching_torque_like_float_out_boy() {
    let mut state = RideModifierState {
        nose: degrees(1.0),
        ..RideModifierState::default()
    };
    state.turn.angle.set_value_for_test(degrees(2.0));
    state.atr.angle.set_value_for_test(degrees(4.0));
    state.brake.set_value_for_test(degrees(5.0));
    state.torque.set_value_for_test(degrees(3.0));
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
        AngleDegrees::from_degrees(2.0 * (1.0 - 0.009_95))
    );
    assert_eq!(
        setpoints.torque_tilt().angle(),
        AngleDegrees::from_degrees(3.0 * (1.0 - 0.009_95))
    );
    assert_eq!(
        setpoints.atr().angle(),
        AngleDegrees::from_degrees(4.0 * (1.0 - 0.009_95))
    );
    assert_eq!(
        setpoints.brake_tilt().angle(),
        AngleDegrees::from_degrees(5.0 * (1.0 - 0.009_95))
    );
    assert_eq!(
        setpoints.board().angle(),
        AngleDegrees::from_degrees(1.0 + 2.0 * (1.0 - 0.009_95) + (4.0 + 5.0) * (1.0 - 0.009_95)),
    );
}

#[test]
fn darkride_keeps_remote_tilt_but_suppresses_ride_modifiers_like_float_out_boy() {
    let mut state = RideModifierState {
        nose: degrees(1.0),
        ..RideModifierState::default()
    };
    state.turn.angle.set_value_for_test(degrees(2.0));
    state.torque.set_value_for_test(degrees(3.0));
    let retained = state;
    let config = FloatOutBoyConfigImage::defaults();
    let setpoints = state.advance(
        &config,
        RideModifierInput {
            base_setpoint: degrees(4.0),
            remote_setpoint: degrees(-1.0),
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
    let positive = degrees(3.0);
    let positive_zero = degrees(0.0);
    let negative_zero = degrees(-0.0);

    assert!(same_source_sign(positive_zero, positive));
    assert!(same_source_sign(negative_zero, positive));
    assert_eq!(combine_torque_offsets(negative_zero, positive), positive,);
}

#[test]
fn atr_transition_boost_scales_only_opposite_sign_transitions() {
    let configured = PidScale::new(2.0);
    let factor = |setpoint, target| {
        atr_transition_multiplier(
            AngleDegrees::from_degrees(setpoint),
            AngleDegrees::from_degrees(target),
            configured,
        )
        .factor()
    };

    assert!((factor(-2.0, 0.5) - 2.0).abs() < f32::EPSILON);
    assert!((factor(-1.0, 0.5) - 1.5).abs() < f32::EPSILON);
    assert!((factor(2.0, 0.5) - 1.0).abs() < f32::EPSILON);
    assert!((factor(-0.5, 0.5) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn low_speed_high_torque_uses_torque_direction_like_refloat() {
    assert_eq!(
        motor_direction(rpm(-250.0), MotorTorque::from_newton_meters(18.0)),
        SmoothSetpointDirection::Forward
    );
    assert_eq!(
        motor_direction(rpm(-251.0), MotorTorque::from_newton_meters(18.0)),
        SmoothSetpointDirection::Reverse
    );
    assert_eq!(
        motor_direction(rpm(-250.0), MotorTorque::from_newton_meters(17.9)),
        SmoothSetpointDirection::Reverse
    );
    assert_eq!(
        motor_direction(rpm(-250.0), MotorTorque::from_newton_meters(-18.0)),
        SmoothSetpointDirection::Reverse
    );
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
    state.update_nose(&config, rpm(10_000.0), VescSeconds::from_seconds(0.01));
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

    let target = nose_target(&config, rpm(2_000.0));

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
            torque_target(balance, compat_torque(current), braking),
            AngleDegrees::from_degrees(expected),
        );
    }

    let mut state = RideModifierState::default();
    let motor = ModifierMotorState {
        erpm: Rpm::from_revolutions_per_minute(3_000.0),
        direction: SmoothSetpointDirection::Forward,
        braking: false,
    };
    state.update_torque(
        balance,
        compat_torque(100.0),
        motor,
        VescSeconds::from_seconds(0.01),
    );
    let active = state.torque.value();
    assert!(active.is_positive());
    for _ in 0..100 {
        state.update_torque(
            balance,
            MotorTorque::ZERO,
            motor,
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.torque.value() < active);
}

#[test]
fn torque_tilt_speed_tuning_changes_the_slew_rate() {
    let setpoints =
        [10.0, 100.0].map(|speed| {
            let mut config = FloatOutBoyConfigImage::defaults();
            let mut editor = config.editor();
            assert!(editor.set_torque_tilt_start_current(MotorCurrent::new(Current::ZERO)));
            assert!(editor.set_torque_tilt_strength(PidScale::new(0.1)));
            assert!(editor.set_torque_tilt_angle_limit(AngleDegrees::from_degrees(10.0)));
            assert!(editor.set_torque_tilt_on_speed(
                vescpkg_rs::AngularVelocity::from_degrees_per_second(speed),
            ));
            let mut state = RideModifierState::default();
            for _ in 0..50 {
                state.update_torque(
                    config.balance(),
                    compat_torque(100.0),
                    ModifierMotorState {
                        erpm: rpm(3_000.0),
                        direction: SmoothSetpointDirection::Forward,
                        braking: false,
                    },
                    VescSeconds::from_seconds(0.01),
                );
            }
            state.torque.value()
        });

    assert!(
        setpoints[1].abs() > setpoints[0].abs(),
        "slow={:?}, fast={:?}",
        setpoints[0],
        setpoints[1]
    );
}

#[test]
fn torque_tilt_uses_firmware_derived_torque_instead_of_raw_current() {
    let mut config = FloatOutBoyConfigImage::defaults();
    let mut editor = config.editor();
    assert!(editor.set_torque_tilt_start_current(MotorCurrent::new(amps(10.0))));
    assert!(editor.set_torque_tilt_strength(PidScale::new(0.1)));
    assert!(editor.set_torque_tilt_angle_limit(degrees(10.0)));
    let current = amps(30.0);
    let low_torque_constant = MotorTorqueConstant::from_firmware_config(
        vescpkg_rs::prelude::FocMotorFluxLinkage::new(
            vescpkg_rs::prelude::FluxLinkage::from_webers(0.004),
        ),
        vescpkg_rs::prelude::MotorPoleCount::try_new(14).ok(),
    );

    let compatibility_target = torque_target(
        config.balance(),
        MotorTorqueConstant::REFLOAT_COMPAT.torque_from_current(current),
        false,
    );
    let configured_target = torque_target(
        config.balance(),
        low_torque_constant.torque_from_current(current),
        false,
    );

    assert_f32_eq!(compatibility_target.as_degrees(), 2.0);
    assert_eq!(configured_target, AngleDegrees::ZERO);
}

#[test]
fn atr_expected_acceleration_switches_slope_at_fifteen_newton_meters() {
    let erpm = rpm(1_000.0);
    let ratio = PidScale::new(2.0);
    let compatibility_constant = MotorTorqueConstant::REFLOAT_COMPAT.newton_meters_per_amp();
    let factor = 2.0 * compatibility_constant;
    let offset = 8.0 * compatibility_constant;

    let below = atr_expected_acceleration(MotorTorque::from_newton_meters(14.0), erpm, ratio);
    let boundary = atr_expected_acceleration(MotorTorque::from_newton_meters(15.0), erpm, ratio);
    let above = atr_expected_acceleration(MotorTorque::from_newton_meters(16.0), erpm, ratio);

    assert_f32_eq!(below.as_erpm_delta(), (14.0 - offset) / factor);
    assert_f32_eq!(boundary.as_erpm_delta(), (15.0 - offset) / factor);
    assert_f32_eq!(
        above.as_erpm_delta(),
        boundary.as_erpm_delta() + 1.0 / (factor * 1.3)
    );
}

#[test]
fn atr_expected_acceleration_preserves_torque_and_erpm_signs() {
    let ratio = PidScale::new(1.0);
    let forward =
        atr_expected_acceleration(MotorTorque::from_newton_meters(20.0), rpm(1_000.0), ratio);
    let reverse =
        atr_expected_acceleration(MotorTorque::from_newton_meters(-20.0), rpm(-1_000.0), ratio);

    assert_f32_eq!(forward.as_erpm_delta(), -reverse.as_erpm_delta());
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
        motor_erpm: rpm(4_000.0),
        filtered_torque: compat_torque(30.0),
        motor_current: MotorCurrent::new(amps(30.0)),
        ..input()
    };

    for _ in 0..200 {
        state.update_atr(
            balance,
            accelerating,
            ModifierMotorState::from_input(accelerating),
            VescSeconds::from_seconds(0.01),
        );
    }
    let accelerating_setpoint = state.atr.angle.value();
    assert!(accelerating_setpoint.is_positive());
    assert!(accelerating_setpoint <= AngleDegrees::from_degrees(3.0));
    assert!((state.atr.speed_boost.as_units() - 1.0 / 7.0).abs() < 0.000_001);

    let braking = RideModifierInput {
        filtered_torque: compat_torque(-30.0),
        motor_current: MotorCurrent::new(amps(-30.0)),
        ..accelerating
    };
    for _ in 0..400 {
        state.update_atr(
            balance,
            braking,
            ModifierMotorState::from_input(braking),
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.atr.angle.value().is_negative());
    assert!(state.atr.angle.value() >= AngleDegrees::from_degrees(-3.0));
    assert!(state.atr.speed_boost.as_units().abs() < f32::EPSILON);

    let before_recovery = state.atr.angle.value();
    for _ in 0..1_000 {
        let recovery = RideModifierInput {
            filtered_torque: compat_torque(8.0),
            motor_current: MotorCurrent::new(Current::from_amps(8.0)),
            ..accelerating
        };
        state.update_atr(
            balance,
            recovery,
            ModifierMotorState::from_input(recovery),
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.atr.angle.value() > before_recovery);
}

#[test]
fn atr_speed_tuning_changes_the_slew_rate() {
    let setpoints = [1.0, 100.0].map(|speed| {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_atr_strength_up(PidScale::new(1.0)));
        assert!(editor.set_atr_strength_down(PidScale::new(1.0)));
        assert!(editor.set_atr_threshold_up(AngleDegrees::ZERO));
        assert!(editor.set_atr_threshold_down(AngleDegrees::ZERO));
        assert!(editor.set_atr_angle_limit(AngleDegrees::from_degrees(3.0)));
        assert!(
            editor.set_atr_on_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(speed),)
        );
        assert!(
            editor.set_atr_off_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(speed),)
        );
        assert!(editor.set_atr_amps_accel_ratio(PidScale::new(1.0)));
        assert!(editor.set_atr_amps_decel_ratio(PidScale::new(1.0)));
        let input = RideModifierInput {
            motor_erpm: rpm(4_000.0),
            filtered_torque: compat_torque(30.0),
            motor_current: MotorCurrent::new(amps(30.0)),
            ..input()
        };
        let mut state = RideModifierState::default();
        for _ in 0..10 {
            state.update_atr(
                config.balance(),
                input,
                ModifierMotorState::from_input(input),
                VescSeconds::from_seconds(0.01),
            );
        }
        state.atr.angle.value()
    });

    assert!(
        setpoints[1].abs() > setpoints[0].abs(),
        "slow={:?}, fast={:?}",
        setpoints[0],
        setpoints[1]
    );
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
            yaw: YawMotion {
                aggregate: degrees(20.0),
                rate: TURN_TILT_YAW_RATE_LIMIT,
                ..YawMotion::default()
            },
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
        balance_pitch: degrees(5.0),
        motor_current: MotorCurrent::new(amps(-5.0)),
        ..input()
    };
    for _ in 0..100 {
        state.update_brake(
            balance,
            braking,
            ModifierMotorState::from_input(braking),
            VescSeconds::from_seconds(0.01),
        );
    }
    let sustained = state.brake.value();
    assert!(sustained.is_positive());
    for _ in 0..100 {
        let coasting = RideModifierInput {
            motor_current: MotorCurrent::new(Current::ZERO),
            ..braking
        };
        state.update_brake(
            balance,
            coasting,
            ModifierMotorState::from_input(coasting),
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.brake.value() < sustained);

    state.update_turn(
        balance,
        ModifierMotorState::from_input(input()),
        VescSeconds::from_seconds(0.01),
    );
    let active_turn = state.turn.angle.value();
    assert!(active_turn.is_positive());
    state.turn.yaw.aggregate = AngleDegrees::ZERO;
    state.turn.yaw.rate = AngularVelocity::ZERO;
    for _ in 0..100 {
        state.update_turn(
            balance,
            ModifierMotorState::from_input(input()),
            VescSeconds::from_seconds(0.01),
        );
    }
    assert!(state.turn.angle.value() < active_turn);
}
