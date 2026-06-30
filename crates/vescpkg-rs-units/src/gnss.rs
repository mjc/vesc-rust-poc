//! GNSS-adjacent generic unit newtypes.

use crate::{scalar_unit, scalar_unit_f64};

scalar_unit_f64!(Latitude, from_degrees, as_degrees, "degrees latitude");
scalar_unit_f64!(Longitude, from_degrees, as_degrees, "degrees longitude");
scalar_unit!(Height, from_meters, as_meters, "meters height");
scalar_unit!(Hdop, from_unitless, as_unitless, "HDOP");
scalar_unit!(GnssAccuracy, from_meters, as_meters, "meters accuracy");
