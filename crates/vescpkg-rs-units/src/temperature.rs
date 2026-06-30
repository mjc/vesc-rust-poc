//! Temperature unit newtypes.

use crate::scalar_unit;

scalar_unit!(
    Temperature,
    from_degrees_celsius,
    as_degrees_celsius,
    "degrees Celsius"
);
