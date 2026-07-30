use super::*;

const EPSILON: f32 = 0.000_001;

fn cutoff_config() -> SmoothSetpointConfig {
    SmoothSetpointConfig {
        time_constant: VescSeconds::from_seconds(0.2),
        on_speed_time_constant: VescSeconds::from_seconds(0.08),
        off_speed_time_constant: VescSeconds::from_seconds(0.16),
        winddown_time_constant: VescSeconds::from_seconds(0.2),
        on_speed_up: AngularVelocity::from_degrees_per_second(24.0),
        off_speed_up: AngularVelocity::from_degrees_per_second(12.0),
        on_speed_down: AngularVelocity::from_degrees_per_second(20.0),
        off_speed_down: AngularVelocity::from_degrees_per_second(10.0),
    }
}

fn configured() -> SmoothSetpoint {
    let mut setpoint = SmoothSetpoint::default();
    setpoint.configure(cutoff_config(), SampleRate::from_hertz(500.0));
    setpoint
}

fn assert_angle_close(actual: AngleDegrees, expected: f32) {
    assert!((actual.as_degrees() - expected).abs() < EPSILON);
}

#[test]
fn first_update_matches_refloat_second_order_filter() {
    let mut setpoint = configured();

    setpoint.update(
        AngleDegrees::from_degrees(10.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::ONE,
        VescSeconds::from_seconds(0.002),
    );

    assert_angle_close(setpoint.value(), 0.000_112_55);
    assert_angle_close(setpoint.filtered_target, 0.213_527);
}

#[test]
fn time_constant_alpha_uses_refloat_half_omega_cap() {
    let alpha = FilterAlpha::from_time_constant(
        VescSeconds::from_seconds(0.000_1),
        SampleRate::from_hertz(500.0),
    );

    assert!((alpha.0.as_ratio() - 0.375).abs() < f32::EPSILON);
    assert!((alpha.scaled(2.146).0.as_ratio() - 0.804_75).abs() < f32::EPSILON);
}

#[test]
fn speed_limit_selects_directional_on_and_off_speeds() {
    let mut setpoint = configured();
    setpoint.value = AngleDegrees::from_degrees(2.0);

    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Forward,
            AngleDegrees::from_degrees(3.0)
        ),
        AngularVelocity::from_degrees_per_second(24.0)
    );
    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Forward,
            AngleDegrees::from_degrees(1.0)
        ),
        AngularVelocity::from_degrees_per_second(12.0)
    );
    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Reverse,
            AngleDegrees::from_degrees(3.0)
        ),
        AngularVelocity::from_degrees_per_second(20.0)
    );
    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Reverse,
            AngleDegrees::from_degrees(1.0)
        ),
        AngularVelocity::from_degrees_per_second(10.0)
    );
}

#[test]
fn sign_crossing_uses_the_faster_directional_limit() {
    let mut setpoint = configured();
    setpoint.value = AngleDegrees::from_degrees(2.0);

    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Forward,
            AngleDegrees::from_degrees(-3.0)
        ),
        AngularVelocity::from_degrees_per_second(24.0)
    );
    assert_eq!(
        setpoint.speed_limit(
            SmoothSetpointDirection::Reverse,
            AngleDegrees::from_degrees(-3.0)
        ),
        AngularVelocity::from_degrees_per_second(20.0)
    );
}

#[test]
fn speed_limit_caps_a_large_internal_step() {
    let mut setpoint = configured();
    setpoint.filtered_target = AngleDegrees::from_degrees(100.0);
    setpoint.step = AngleDegrees::from_degrees(100.0);

    setpoint.update(
        AngleDegrees::from_degrees(100.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::ONE,
        VescSeconds::from_seconds(0.002),
    );

    assert_angle_close(setpoint.value(), 24.0 / 500.0);
}

#[test]
fn multiplier_scales_filter_and_speed_limit() {
    let elapsed = VescSeconds::from_seconds(0.002);
    let mut normal_filter = configured();
    let mut boosted_filter = normal_filter;

    normal_filter.update(
        AngleDegrees::from_degrees(10.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::ONE,
        elapsed,
    );
    boosted_filter.update(
        AngleDegrees::from_degrees(10.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::from_factor(2.0),
        elapsed,
    );

    assert!(boosted_filter.filtered_target > normal_filter.filtered_target);

    let mut normal_limit = configured();
    normal_limit.filtered_target = AngleDegrees::from_degrees(100.0);
    normal_limit.step = AngleDegrees::from_degrees(100.0);
    let mut boosted_limit = normal_limit;
    normal_limit.update(
        AngleDegrees::from_degrees(100.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::ONE,
        elapsed,
    );
    boosted_limit.update(
        AngleDegrees::from_degrees(100.0),
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::from_factor(2.0),
        elapsed,
    );

    assert_angle_close(
        boosted_limit.value(),
        normal_limit.value().as_degrees() * 2.0,
    );
}

#[test]
fn repeated_winddown_matches_refloat_exponential_decay() {
    let mut setpoint = configured();
    setpoint.value = AngleDegrees::from_degrees(10.0);

    setpoint.wind_down();
    setpoint.wind_down();

    assert_angle_close(setpoint.value(), 10.0 * 0.990_05 * 0.990_05);
    assert!(setpoint.is_winddown);
}

#[test]
fn first_update_after_winddown_restarts_from_the_decayed_value() {
    let mut setpoint = configured();
    setpoint.value = AngleDegrees::from_degrees(10.0);
    setpoint.filtered_target = AngleDegrees::from_degrees(-20.0);
    setpoint.step = AngleDegrees::from_degrees(-5.0);
    setpoint.wind_down();
    let decayed = setpoint.value;

    setpoint.update(
        decayed,
        SmoothSetpointDirection::Forward,
        SmoothSetpointMultiplier::ONE,
        VescSeconds::from_seconds(0.002),
    );

    assert_eq!(setpoint.value, decayed);
    assert_eq!(setpoint.filtered_target, decayed);
    assert_eq!(setpoint.step, AngleDegrees::ZERO);
    assert!(!setpoint.is_winddown);
}

#[test]
fn reset_clears_motion_but_retains_configuration() {
    let mut setpoint = configured();
    let configured_speeds = (
        setpoint.on_speed_up,
        setpoint.off_speed_up,
        setpoint.on_speed_down,
        setpoint.off_speed_down,
    );
    setpoint.value = AngleDegrees::from_degrees(3.0);
    setpoint.filtered_target = AngleDegrees::from_degrees(4.0);
    setpoint.step = AngleDegrees::from_degrees(0.5);
    setpoint.is_winddown = true;

    setpoint.reset();

    assert_eq!(setpoint.value, AngleDegrees::ZERO);
    assert_eq!(setpoint.filtered_target, AngleDegrees::ZERO);
    assert_eq!(setpoint.step, AngleDegrees::ZERO);
    assert!(!setpoint.is_winddown);
    assert_eq!(
        configured_speeds,
        (
            setpoint.on_speed_up,
            setpoint.off_speed_up,
            setpoint.on_speed_down,
            setpoint.off_speed_down,
        )
    );
}

#[test]
fn configure_does_not_reset_live_motion() {
    let mut setpoint = configured();
    setpoint.value = AngleDegrees::from_degrees(3.0);
    setpoint.filtered_target = AngleDegrees::from_degrees(4.0);
    setpoint.step = AngleDegrees::from_degrees(0.5);

    setpoint.configure(cutoff_config(), SampleRate::from_hertz(250.0));

    assert_eq!(setpoint.value, AngleDegrees::from_degrees(3.0));
    assert_eq!(setpoint.filtered_target, AngleDegrees::from_degrees(4.0));
    assert_eq!(setpoint.step, AngleDegrees::from_degrees(0.5));
}

#[test]
fn direction_from_erpm_treats_zero_as_forward_like_refloat() {
    assert_eq!(
        SmoothSetpointDirection::from_erpm(Rpm::ZERO),
        SmoothSetpointDirection::Forward
    );
    assert_eq!(
        SmoothSetpointDirection::from_erpm(Rpm::from_revolutions_per_minute(-1.0)),
        SmoothSetpointDirection::Reverse
    );
}

#[test]
fn positive_and_negative_zero_share_refloats_nonnegative_sign() {
    assert!(same_source_sign(
        AngleDegrees::from_degrees(0.0),
        AngleDegrees::from_degrees(-0.0)
    ));
}
