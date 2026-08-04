use super::*;

#[test]
fn float_out_boy_limits_keep_source_backed_typed_values() {
    assert_eq!(
        quick_stop::STOPPED_ERPM,
        Rpm::from_revolutions_per_minute(200.0)
    );
    assert_eq!(quick_stop::PITCH, AngleDegrees::from_degrees(14.0));

    assert_eq!(
        reverse_stop::ENTRY_ERPM,
        Rpm::from_revolutions_per_minute(200.0)
    );
    assert_eq!(
        reverse_stop::TOLERANCE_ERPM,
        Rpm::from_revolutions_per_minute(20_000.0)
    );
    assert_eq!(
        reverse_stop::TOTAL_ERPM,
        Rpm::from_revolutions_per_minute(200_000.0)
    );
    assert_eq!(reverse_stop::PITCH, AngleDegrees::from_degrees(18.0));
    assert_eq!(
        reverse_stop::TIMER_FAST_PITCH,
        AngleDegrees::from_degrees(10.0)
    );
    assert_eq!(
        reverse_stop::TIMER_SLOW_PITCH,
        AngleDegrees::from_degrees(5.0)
    );
    assert_eq!(
        reverse_stop::carryover_total_erpm(AngleDegrees::from_degrees(0.08)),
        Rpm::from_revolutions_per_minute(-21_000.0)
    );
    assert_eq!(
        reverse_stop::target_angle(Rpm::from_revolutions_per_minute(21_000.0)),
        AngleDegrees::from_degrees(0.08)
    );

    assert_eq!(
        REMOTE_SETPOINT_FAULT_ANGLE,
        AngleDegrees::from_degrees(30.0)
    );
    assert_eq!(MOVING_FAULT_ROLL, AngleDegrees::from_degrees(40.0));

    assert_eq!(
        darkride::TIMED_HIGH_ERPM,
        Rpm::from_revolutions_per_minute(1000.0)
    );
    assert_eq!(darkride::TIMED_HIGH_DELAY, VescSeconds::from_seconds(0.1));
    assert_eq!(
        darkride::HIGH_ERPM,
        Rpm::from_revolutions_per_minute(2000.0)
    );
    assert_eq!(darkride::LOW_ERPM, Rpm::from_revolutions_per_minute(300.0));
    assert_eq!(darkride::LOW_DELAY, VescSeconds::from_seconds(0.5));
    assert_eq!(darkride::ROLL_LOWER, AngleDegrees::from_degrees(100.0));
    assert_eq!(darkride::ROLL_UPPER, AngleDegrees::from_degrees(135.0));

    assert_eq!(
        push_start::ERPM_MIN,
        Rpm::from_revolutions_per_minute(1000.0)
    );
    assert_eq!(push_start::ANGLE, AngleDegrees::from_degrees(45.0));

    assert_eq!(
        traction_loss::ACCELERATION_DETECT,
        Rpm::from_revolutions_per_minute(15.0)
    );
    assert_eq!(
        traction_loss::ACCELERATION_CLEAR,
        Rpm::from_revolutions_per_minute(10.0)
    );
    assert_eq!(traction_loss::DUTY, SignedRatio::from_ratio_const(0.3));
    assert_eq!(
        traction_loss::ERPM,
        Rpm::from_revolutions_per_minute(2000.0)
    );
    assert_eq!(
        traction_loss::DUTY_MARGIN,
        vescpkg_rs::prelude::Ratio::from_ratio_const(0.05)
    );
    assert_eq!(traction_loss::CLEAR_DELAY, VescSeconds::from_seconds(0.2));
    assert_eq!(
        traction_loss::RAW_DUTY_CLEAR,
        vescpkg_rs::prelude::Ratio::from_ratio_const(0.85)
    );
}
