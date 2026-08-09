use super::FloatOutBoyTuneNibble;
use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent, PidScale, RateCurrentGain};

#[test]
fn tune_nibble_keeps_exact_endpoints_without_primitive_conversions() {
    assert_eq!(FloatOutBoyTuneNibble::low(0xF0), FloatOutBoyTuneNibble(0));
    assert_eq!(FloatOutBoyTuneNibble::high(0xF0), FloatOutBoyTuneNibble(15));
    assert_eq!(
        FloatOutBoyTuneNibble(0).angle_from(AngleDegrees::from_degrees(5.0)),
        AngleDegrees::from_degrees(5.0),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(15).angle_from(AngleDegrees::from_degrees(5.0)),
        AngleDegrees::from_degrees(20.0),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(0).booster_current(),
        MotorCurrent::new(Current::ZERO),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(15).booster_current(),
        MotorCurrent::new(Current::from_amps(38.0)),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(9).divided(10.0, 0.0, RateCurrentGain::new),
        RateCurrentGain::new(9.0 / 10.0),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(9).torque_tilt_strength(),
        PidScale::new((9.0 / 10.0) * 0.3),
    );
    assert_eq!(
        FloatOutBoyTuneNibble(6).brake_gain(),
        PidScale::new((6.0 + 1.0) / 10.0),
    );
}
