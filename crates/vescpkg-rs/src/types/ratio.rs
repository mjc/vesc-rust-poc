//! Controller ratio semantic wrappers.

use crate::units::Ratio;

macro_rules! ratio_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Ratio);

        impl $name {
            /// Wrap a generic ratio with VESC-domain meaning.
            pub const fn new(ratio: Ratio) -> Self {
                Self(ratio)
            }

            /// Return the typed ratio without erasing it to a primitive.
            pub const fn ratio(self) -> Ratio {
                self.0
            }
        }
    };
}

ratio_type!(DutyCycle, "Controller duty-cycle command ratio.");
ratio_type!(CurrentRelative, "Relative motor-current command ratio.");
ratio_type!(
    BrakeCurrentRelative,
    "Relative brake-current command ratio."
);
ratio_type!(HandbrakeRelative, "Relative handbrake command ratio.");
ratio_type!(PpmInput, "Decoded PPM input ratio.");
ratio_type!(JoystickX, "Joystick X input ratio.");
ratio_type!(JoystickY, "Joystick Y input ratio.");
