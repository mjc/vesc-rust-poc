//! GNSS-adjacent generic unit newtypes.

use crate::scalar_unit;

scalar_unit!(Latitude, from_degrees, as_degrees, "degrees latitude");
scalar_unit!(Longitude, from_degrees, as_degrees, "degrees longitude");
scalar_unit!(Height, from_meters, as_meters, "meters height");
scalar_unit!(Hdop, from_unitless, as_unitless, "HDOP");
scalar_unit!(GnssAccuracy, from_meters, as_meters, "meters accuracy");
