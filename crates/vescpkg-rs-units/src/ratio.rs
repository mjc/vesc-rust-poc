//! Unitless ratio and percent newtypes.

use crate::bounded_unit;

bounded_unit!(DutyCycle, from_ratio, as_ratio, 0.0, 1.0, "normalized duty");
bounded_unit!(Ratio, from_ratio, as_ratio, 0.0, 1.0, "normalized ratio");
bounded_unit!(
    SignedRatio,
    from_ratio,
    as_ratio,
    -1.0,
    1.0,
    "signed normalized ratio"
);
bounded_unit!(Percent, from_percent, as_percent, 0.0, 100.0, "percent");
bounded_unit!(Pwm, from_ratio, as_ratio, 0.0, 1.0, "normalized PWM");
