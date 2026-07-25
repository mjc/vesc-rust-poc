//! Owned GNSS snapshots copied from the optional firmware record.

use crate::{
    GnssAltitude, GnssHdop, GnssLatitude, GnssLongitude, GnssSpeed, Height, Latitude, Longitude,
    Speed, TimestampTicks,
};

/// Failure returned by the GNSS capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GnssError {
    /// The firmware does not expose a GNSS record slot or current record.
    Unavailable,
}

/// Owned copy of one firmware GNSS record.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GnssSnapshot {
    latitude: GnssLatitude,
    longitude: GnssLongitude,
    altitude: GnssAltitude,
    speed: GnssSpeed,
    hdop: GnssHdop,
    last_update: TimestampTicks,
    milliseconds_today: i32,
    year: i8,
    month: i8,
    day: i8,
}

impl GnssSnapshot {
    pub(crate) fn from_raw(data: vescpkg_rs_sys::raw::GnssData) -> Self {
        Self {
            latitude: GnssLatitude::new(Latitude::from_degrees(data.lat)),
            longitude: GnssLongitude::new(Longitude::from_degrees(data.lon)),
            altitude: GnssAltitude::new(Height::from_meters(data.height)),
            speed: GnssSpeed::new(Speed::from_meters_per_second(data.speed)),
            hdop: GnssHdop::from_unitless(data.hdop),
            last_update: TimestampTicks::from_ticks(data.last_update),
            milliseconds_today: data.ms_today,
            year: data.yy,
            month: data.mo,
            day: data.dd,
        }
    }

    /// Return typed latitude.
    pub const fn latitude(self) -> GnssLatitude {
        self.latitude
    }
    /// Return typed longitude.
    pub const fn longitude(self) -> GnssLongitude {
        self.longitude
    }
    /// Return typed altitude.
    pub const fn altitude(self) -> GnssAltitude {
        self.altitude
    }
    /// Return typed ground speed.
    pub const fn speed(self) -> GnssSpeed {
        self.speed
    }
    /// Return horizontal dilution of precision.
    pub const fn hdop(self) -> GnssHdop {
        self.hdop
    }
    /// Return the firmware update timestamp.
    pub const fn last_update(self) -> TimestampTicks {
        self.last_update
    }
    /// Return milliseconds since midnight in the firmware date record.
    pub const fn milliseconds_today(self) -> i32 {
        self.milliseconds_today
    }
    /// Return the firmware GNSS year field.
    pub const fn year(self) -> i8 {
        self.year
    }
    /// Return the firmware GNSS month field.
    pub const fn month(self) -> i8 {
        self.month
    }
    /// Return the firmware GNSS day field.
    pub const fn day(self) -> i8 {
        self.day
    }
}

/// Optional GNSS capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gnss;

impl Gnss {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Copy the current firmware-owned GNSS record.
    pub fn snapshot(&self) -> Result<GnssSnapshot, GnssError> {
        unsafe { crate::ffi::gnss_snapshot() }
            .map(GnssSnapshot::from_raw)
            .ok_or(GnssError::Unavailable)
    }
}

impl crate::Firmware {
    /// Return the optional GNSS capability handle.
    pub fn gnss(&self) -> Gnss {
        Gnss::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional GNSS capability handle.
    pub fn gnss(&self) -> Gnss {
        Gnss::new()
    }
}
