//! Motion-domain semantic wrappers.

use crate::units::{
    Distance, ElectricalRpm, MechanicalRpm, Speed, TachometerSteps as UnitTachometerSteps,
};

macro_rules! speed_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Speed);

        impl $name {
            /// Wrap a generic speed with VESC-domain meaning.
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

macro_rules! distance_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Distance);

        impl $name {
            /// Wrap a generic distance with VESC-domain meaning.
            pub const fn new(distance: Distance) -> Self {
                Self(distance)
            }

            /// Return the typed distance without erasing it to a primitive.
            pub const fn distance(self) -> Distance {
                self.0
            }
        }
    };
}

macro_rules! mechanical_rpm_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(MechanicalRpm);

        impl $name {
            /// Wrap generic mechanical RPM with VESC-domain meaning.
            pub const fn new(rpm: MechanicalRpm) -> Self {
                Self(rpm)
            }

            /// Return the typed mechanical RPM without erasing it to a primitive.
            pub const fn rpm(self) -> MechanicalRpm {
                self.0
            }
        }
    };
}

macro_rules! electrical_rpm_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(ElectricalRpm);

        impl $name {
            /// Wrap generic electrical RPM with VESC-domain meaning.
            pub const fn new(rpm: ElectricalRpm) -> Self {
                Self(rpm)
            }

            /// Return the typed electrical RPM without erasing it to a primitive.
            pub const fn rpm(self) -> ElectricalRpm {
                self.0
            }
        }
    };
}

macro_rules! tachometer_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(UnitTachometerSteps);

        impl $name {
            /// Wrap generic tachometer steps with VESC-domain meaning.
            pub const fn new(steps: UnitTachometerSteps) -> Self {
                Self(steps)
            }

            /// Return the typed tachometer steps without erasing it to a primitive.
            pub const fn steps(self) -> UnitTachometerSteps {
                self.0
            }
        }
    };
}

speed_type!(VehicleSpeed, "Estimated vehicle speed.");
distance_type!(TripDistance, "Trip distance travelled by the vehicle.");
mechanical_rpm_type!(MechanicalSpeed, "Mechanical motor speed.");
electrical_rpm_type!(ElectricalSpeed, "Electrical motor speed.");
tachometer_type!(TachometerSteps, "Relative tachometer position.");
tachometer_type!(AbsoluteTachometerSteps, "Absolute tachometer position.");
