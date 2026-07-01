//! Configuration semantic wrappers.

use crate::units::{Distance, FluxLinkage, Inductance, Resistance};

macro_rules! positive_count_type {
    ($name:ident, $error:ident, $doc:literal, $error_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(u16);

        impl $name {
            /// Create a checked non-zero count.
            pub const fn try_new(count: u16) -> Result<Self, $error> {
                if count == 0 {
                    Err($error { value: count })
                } else {
                    Ok(Self(count))
                }
            }

            /// Explicitly extract the raw count.
            pub const fn get(self) -> u16 {
                self.0
            }
        }

        #[doc = $error_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $error {
            value: u16,
        }

        impl $error {
            /// Return the rejected count.
            pub const fn value(self) -> u16 {
                self.value
            }
        }
    };
}

macro_rules! unit_type {
    ($name:ident, $inner:ty, $new_arg:ident, $accessor:ident, $doc:literal, $accessor_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Wrap a generic unit with config meaning.
            pub const fn new($new_arg: $inner) -> Self {
                Self($new_arg)
            }

            #[doc = $accessor_doc]
            pub const fn $accessor(self) -> $inner {
                self.0
            }
        }
    };
}

/// Gear reduction ratio configured for speed/distance calculations.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GearRatio(f32);

impl GearRatio {
    /// Create a checked positive gear ratio.
    pub const fn try_new(ratio: f32) -> Result<Self, GearRatioError> {
        if ratio > 0.0 {
            Ok(Self(ratio))
        } else {
            Err(GearRatioError { value: ratio })
        }
    }

    /// Explicitly extract the raw gear ratio.
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Error returned when a gear ratio is zero, negative, or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GearRatioError {
    value: f32,
}

impl GearRatioError {
    /// Return the rejected ratio.
    pub const fn value(self) -> f32 {
        self.value
    }
}

positive_count_type!(
    MotorPoleCount,
    MotorPoleCountError,
    "Configured motor pole count.",
    "Error returned when the motor pole count is zero."
);
positive_count_type!(
    BatteryCellCount,
    BatteryCellCountError,
    "Configured battery cell count.",
    "Error returned when the battery cell count is zero."
);

unit_type!(
    WheelDiameter,
    Distance,
    distance,
    distance,
    "Configured wheel diameter.",
    "Return the typed wheel diameter without erasing it to a primitive."
);
unit_type!(
    FocMotorResistance,
    Resistance,
    resistance,
    resistance,
    "Configured FOC motor resistance.",
    "Return the typed motor resistance without erasing it to a primitive."
);
unit_type!(
    FocMotorInductance,
    Inductance,
    inductance,
    inductance,
    "Configured FOC motor inductance.",
    "Return the typed motor inductance without erasing it to a primitive."
);
unit_type!(
    FocMotorFluxLinkage,
    FluxLinkage,
    flux_linkage,
    flux_linkage,
    "Configured FOC motor flux linkage.",
    "Return the typed motor flux linkage without erasing it to a primitive."
);
