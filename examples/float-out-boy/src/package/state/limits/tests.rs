use super::*;

#[test]
fn float_out_boy_limits_keep_source_backed_typed_values() {
    let quickstop = QuickStopLimits::FLOAT_OUT_BOY;
    assert_eq!(
        quickstop.stopped_erpm,
        Rpm::from_revolutions_per_minute(200.0)
    );
    assert_eq!(quickstop.pitch, AngleDegrees::from_degrees(14.0));

    let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
    assert_eq!(
        reverse_stop.total_erpm,
        Rpm::from_revolutions_per_minute(200_000.0)
    );
    assert_eq!(reverse_stop.pitch, AngleDegrees::from_degrees(18.0));
    assert_eq!(
        reverse_stop.timer_fast_pitch,
        AngleDegrees::from_degrees(10.0)
    );
    assert_eq!(
        reverse_stop.timer_slow_pitch,
        AngleDegrees::from_degrees(5.0)
    );

    assert_eq!(
        RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle(),
        AngleDegrees::from_degrees(30.0)
    );
    assert_eq!(
        MovingFaultLimits::FLOAT_OUT_BOY.roll,
        AngleDegrees::from_degrees(40.0)
    );

    let darkride = DarkrideLimits::FLOAT_OUT_BOY;
    assert_eq!(darkride.high_erpm, Rpm::from_revolutions_per_minute(2000.0));
    assert_eq!(darkride.roll_lower, AngleDegrees::from_degrees(100.0));
    assert_eq!(darkride.roll_upper, AngleDegrees::from_degrees(135.0));

    let push_start = PushStartLimits::FLOAT_OUT_BOY;
    assert_eq!(
        push_start.erpm_min,
        Rpm::from_revolutions_per_minute(1000.0)
    );
    assert_eq!(push_start.angle, AngleDegrees::from_degrees(45.0));

    let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
    assert_eq!(
        traction_loss.acceleration_detect,
        Rpm::from_revolutions_per_minute(15.0)
    );
    assert_eq!(
        traction_loss.acceleration_clear,
        Rpm::from_revolutions_per_minute(10.0)
    );
    assert_eq!(traction_loss.duty, SignedRatio::from_ratio_const(0.3));
    assert_eq!(traction_loss.erpm, Rpm::from_revolutions_per_minute(2000.0));
    assert_eq!(
        traction_loss.duty_margin,
        vescpkg_rs::prelude::Ratio::from_ratio_const(0.05)
    );
    assert_eq!(traction_loss.clear_delay, VescSeconds::from_seconds(0.2));
    assert_eq!(
        traction_loss.raw_duty_clear,
        vescpkg_rs::prelude::Ratio::from_ratio_const(0.85)
    );
}
