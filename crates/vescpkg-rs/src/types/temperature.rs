//! Temperature semantic wrappers.

use crate::units::Temperature;

macro_rules! temperature_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Temperature);

        impl $name {
            /// Wrap a generic temperature with VESC-domain meaning.
            pub const fn new(temperature: Temperature) -> Self {
                Self(temperature)
            }

            /// Return the typed temperature without erasing it to a primitive.
            pub const fn temperature(self) -> Temperature {
                self.0
            }
        }
    };
}

temperature_type!(FetTemperature, "FET temperature.");
temperature_type!(MotorTemperature, "Motor temperature.");
temperature_type!(MosfetTemperature, "MOSFET temperature.");
temperature_type!(
    AverageMosfetTemperature,
    "Average MOSFET temperature statistic."
);
temperature_type!(PeakMosfetTemperature, "Peak MOSFET temperature statistic.");
temperature_type!(
    AverageMotorTemperature,
    "Average motor temperature statistic."
);
temperature_type!(PeakMotorTemperature, "Peak motor temperature statistic.");
temperature_type!(TemperatureLimitStart, "Temperature limit start threshold.");
temperature_type!(TemperatureLimitEnd, "Temperature limit end threshold.");
