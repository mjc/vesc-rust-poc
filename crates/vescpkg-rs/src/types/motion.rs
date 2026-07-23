//! Motion-domain semantic wrappers.

use crate::units::{AngleDegrees, Distance, Rpm, Speed, TachometerSteps as UnitTachometerSteps};

/// Select whether a tachometer read preserves or resets firmware state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TachometerReset {
    /// Read the current value without changing the firmware counter.
    Preserve,
    /// Read the current value and reset the firmware counter.
    Reset,
}

impl TachometerReset {
    /// Return the ABI reset flag for this semantic choice.
    pub const fn resets(self) -> bool {
        matches!(self, Self::Reset)
    }
}

/// Select whether a PID-position offset update remains live-only or is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidPositionOffsetPersistence {
    /// Update the live value without persisting it to firmware configuration.
    Volatile,
    /// Update the live value and ask firmware to persist it.
    Persistent,
}

impl PidPositionOffsetPersistence {
    /// Return the ABI persistence flag for this semantic choice.
    pub const fn stores(self) -> bool {
        matches!(self, Self::Persistent)
    }
}

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
        pub struct $name(Rpm);

        impl $name {
            /// Wrap generic RPM with VESC-domain mechanical-speed meaning.
            pub const fn new(rpm: Rpm) -> Self {
                Self(rpm)
            }

            /// Return the generic RPM without erasing it to a primitive.
            pub const fn rpm(self) -> Rpm {
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
        pub struct $name(Rpm);

        impl $name {
            /// Wrap generic RPM with VESC-domain electrical-speed meaning.
            pub const fn new(rpm: Rpm) -> Self {
                Self(rpm)
            }

            /// Return the generic RPM without erasing it to a primitive.
            pub const fn rpm(self) -> Rpm {
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

macro_rules! angle_degrees_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(AngleDegrees);

        impl $name {
            /// Wrap generic degrees with VESC-domain meaning.
            pub const fn new(angle: AngleDegrees) -> Self {
                Self(angle)
            }

            /// Return the typed angle without erasing it to a primitive.
            pub const fn angle(self) -> AngleDegrees {
                self.0
            }
        }
    };
}

speed_type!(VehicleSpeed, "Estimated vehicle speed.");
speed_type!(AverageVehicleSpeed, "Average vehicle speed statistic.");
speed_type!(PeakVehicleSpeed, "Peak vehicle speed statistic.");
distance_type!(TripDistance, "Trip distance travelled by the vehicle.");
distance_type!(
    SignedTripDistance,
    "Signed trip distance travelled by the vehicle."
);
mechanical_rpm_type!(MechanicalSpeed, "Mechanical motor speed.");
electrical_rpm_type!(ElectricalSpeed, "Electrical motor speed.");
tachometer_type!(TachometerSteps, "Relative tachometer position.");
tachometer_type!(AbsoluteTachometerSteps, "Absolute tachometer position.");
angle_degrees_type!(PidPosition, "PID motor position command in degrees.");
angle_degrees_type!(OpenLoopPhase, "FOC open-loop phase command in degrees.");
