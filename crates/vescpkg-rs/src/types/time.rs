//! Time semantic wrappers.

use crate::units::{SystemTicks, VescSeconds};

macro_rules! duration_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(SystemTicks);

        impl $name {
            /// Wrap system ticks with semantic duration meaning.
            pub const fn new(duration: SystemTicks) -> Self {
                Self(duration)
            }

            /// Return the typed duration without erasing it to a primitive.
            pub const fn duration(self) -> SystemTicks {
                self.0
            }
        }
    };
}

duration_type!(SystemDuration, "Elapsed system duration.");

macro_rules! seconds_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(VescSeconds);

        impl $name {
            /// Wrap VESC float seconds with semantic duration meaning.
            pub const fn new(seconds: VescSeconds) -> Self {
                Self(seconds)
            }

            /// Return the typed seconds without erasing them to a primitive.
            pub const fn duration(self) -> VescSeconds {
                self.0
            }
        }
    };
}

seconds_type!(TimeoutDuration, "Timeout duration in VESC seconds.");
seconds_type!(
    CurrentOffDelay,
    "Motor current off-delay duration in VESC seconds."
);
seconds_type!(
    RemoteAge,
    "Age of the latest remote input sample in VESC seconds."
);
seconds_type!(
    PpmAge,
    "Age of the latest PPM input sample in VESC seconds."
);
