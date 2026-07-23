//! GNSS-domain semantic wrappers.

use crate::units::{Distance, Height, Latitude, Longitude, Speed};

macro_rules! latitude_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Latitude);

        impl $name {
            /// Wrap a generic latitude with GNSS-domain meaning.
            pub const fn new(latitude: Latitude) -> Self {
                Self(latitude)
            }

            /// Return the typed latitude without erasing it to a primitive.
            pub const fn latitude(self) -> Latitude {
                self.0
            }
        }
    };
}

macro_rules! longitude_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Longitude);

        impl $name {
            /// Wrap a generic longitude with GNSS-domain meaning.
            pub const fn new(longitude: Longitude) -> Self {
                Self(longitude)
            }

            /// Return the typed longitude without erasing it to a primitive.
            pub const fn longitude(self) -> Longitude {
                self.0
            }
        }
    };
}

macro_rules! altitude_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Height);

        impl $name {
            /// Wrap a generic height with GNSS-domain meaning.
            pub const fn new(altitude: Height) -> Self {
                Self(altitude)
            }

            /// Return the typed altitude without erasing it to a primitive.
            pub const fn altitude(self) -> Height {
                self.0
            }
        }
    };
}

macro_rules! speed_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Speed);

        impl $name {
            /// Wrap a generic speed with GNSS-domain meaning.
            pub const fn new(speed: Speed) -> Self {
                Self(speed)
            }

            /// Return the typed speed without erasing it to a primitive.
            pub const fn speed(self) -> Speed {
                self.0
            }
        }
    };
}

latitude_type!(GnssLatitude, "Latitude reported by GNSS.");
longitude_type!(GnssLongitude, "Longitude reported by GNSS.");
altitude_type!(GnssAltitude, "Altitude reported by GNSS.");
speed_type!(GnssSpeed, "Ground speed reported by GNSS.");

/// Horizontal dilution of precision reported by GNSS.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GnssHdop(f32);

impl GnssHdop {
    /// Wrap the unitless HDOP value reported by firmware.
    #[must_use]
    pub const fn from_unitless(value: f32) -> Self {
        Self(value)
    }

    /// Return the unitless HDOP value for explicit firmware/API boundaries.
    #[must_use]
    pub const fn as_unitless(self) -> f32 {
        self.0
    }
}

/// Position accuracy reported by GNSS.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GnssAccuracy(Distance);

impl GnssAccuracy {
    /// Wrap generic distance with GNSS accuracy meaning.
    #[must_use]
    pub const fn new(distance: Distance) -> Self {
        Self(distance)
    }

    /// Return the accuracy distance without erasing it to a primitive.
    #[must_use]
    pub const fn distance(self) -> Distance {
        self.0
    }
}
