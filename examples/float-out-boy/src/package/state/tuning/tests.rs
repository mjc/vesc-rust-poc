use super::{tune_angle_from, tune_booster_current, tune_brake_gain, tune_torque_tilt_strength};
use vescpkg_rs::prelude::{
    AngleDegrees, Current, MotorCurrent, PidScale, RateCurrentGain, WireByte,
};

#[test]
fn tune_nibble_keeps_exact_endpoints_without_primitive_conversions() {
    assert_eq!(WireByte::low_nibble(0xF0), WireByte::new(0));
    assert_eq!(WireByte::high_nibble(0xF0), WireByte::new(15));
    assert_eq!(
        tune_angle_from(WireByte::new(0), AngleDegrees::from_degrees(5.0)),
        AngleDegrees::from_degrees(5.0),
    );
    assert_eq!(
        tune_angle_from(WireByte::new(15), AngleDegrees::from_degrees(5.0)),
        AngleDegrees::from_degrees(20.0),
    );
    assert_eq!(
        tune_booster_current(WireByte::new(0)),
        MotorCurrent::new(Current::ZERO),
    );
    assert_eq!(
        tune_booster_current(WireByte::new(15)),
        MotorCurrent::new(Current::from_amps(38.0)),
    );
    assert_eq!(
        WireByte::new(9).divided(10.0, 0.0, RateCurrentGain::new),
        RateCurrentGain::new(9.0 / 10.0),
    );
    assert_eq!(
        tune_torque_tilt_strength(WireByte::new(9)),
        PidScale::new((9.0 / 10.0) * 0.3),
    );
    assert_eq!(
        tune_brake_gain(WireByte::new(6)),
        PidScale::new((6.0 + 1.0) / 10.0),
    );
}
