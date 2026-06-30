//! Motor-domain semantic wrappers.

use crate::units::{Current, Voltage};

macro_rules! current_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Current);

        impl $name {
            /// Wrap a generic current with VESC-domain meaning.
            pub const fn new(current: Current) -> Self {
                Self(current)
            }

            /// Return the typed current without erasing it to a primitive.
            pub const fn current(self) -> Current {
                self.0
            }
        }
    };
}

macro_rules! voltage_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Voltage);

        impl $name {
            /// Wrap a generic voltage with VESC-domain meaning.
            pub const fn new(voltage: Voltage) -> Self {
                Self(voltage)
            }

            /// Return the typed voltage without erasing it to a primitive.
            pub const fn voltage(self) -> Voltage {
                self.0
            }
        }
    };
}

current_type!(MotorCurrent, "Motor phase/current-control current.");
current_type!(BrakeCurrent, "Motor braking current.");
current_type!(HandbrakeCurrent, "Handbrake current command.");
current_type!(PhaseCurrent, "Measured motor phase current.");
current_type!(TotalMotorCurrent, "Total motor current.");
current_type!(DirectionalMotorCurrent, "Signed/directional motor current.");
current_type!(DCurrent, "FOC d-axis current.");
current_type!(QCurrent, "FOC q-axis current.");
current_type!(OpenLoopCurrent, "Open-loop motor current command.");
voltage_type!(DVoltage, "FOC d-axis voltage.");
voltage_type!(QVoltage, "FOC q-axis voltage.");
voltage_type!(AudioVoltage, "Audio/haptic voltage command.");
