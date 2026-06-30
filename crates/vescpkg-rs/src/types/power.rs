//! Power semantic wrappers.

use crate::units::Power;

macro_rules! power_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Power);

        impl $name {
            /// Wrap a generic power value with VESC-domain meaning.
            pub const fn new(power: Power) -> Self {
                Self(power)
            }

            /// Return the typed power without erasing it to a primitive.
            pub const fn power(self) -> Power {
                self.0
            }
        }
    };
}

power_type!(AveragePower, "Average power statistic.");
power_type!(PeakPower, "Peak power statistic.");
