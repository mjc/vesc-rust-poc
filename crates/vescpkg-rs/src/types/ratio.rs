//! Controller ratio semantic wrappers.

use crate::units::{Ratio, SignedRatio};

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

macro_rules! signed_ratio_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(SignedRatio);

        impl $name {
            /// Wrap a generic signed ratio with VESC-domain meaning.
            pub const fn new(ratio: SignedRatio) -> Self {
                Self(ratio)
            }

            /// Return the typed signed ratio without erasing it to a primitive.
            pub const fn ratio(self) -> SignedRatio {
                self.0
            }
        }
    };
}

signed_ratio_type!(
    DutyCycle,
    "Signed controller duty-cycle command ratio in `-1.0..=1.0`."
);
ratio_type!(Pwm, "Normalized PWM output command ratio in `0.0..=1.0`.");
signed_ratio_type!(
    CurrentRelative,
    "Signed relative motor-current command ratio."
);
ratio_type!(
    BrakeCurrentRelative,
    "Relative brake-current command ratio."
);
ratio_type!(HandbrakeRelative, "Relative handbrake command ratio.");
signed_ratio_type!(PpmInput, "Decoded PPM input ratio.");
signed_ratio_type!(JoystickX, "Joystick X input ratio.");
signed_ratio_type!(JoystickY, "Joystick Y input ratio.");
