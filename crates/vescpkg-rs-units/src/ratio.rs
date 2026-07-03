//! Unitless ratio and percent newtypes.

use crate::bounded_unit;

bounded_unit!(
    Ratio,
    from_ratio,
    from_ratio_const,
    as_ratio,
    0.0,
    1.0,
    "normalized ratio"
);
bounded_unit!(
    SignedRatio,
    from_ratio,
    from_ratio_const,
    as_ratio,
    -1.0,
    1.0,
    "signed normalized ratio"
);
bounded_unit!(
    Percent,
    from_percent,
    from_percent_const,
    as_percent,
    0.0,
    100.0,
    "percent"
);
