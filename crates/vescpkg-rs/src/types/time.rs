//! Time semantic wrappers.

use crate::units::{AbiSeconds, SystemTicks, TimestampTicks};

/// System timestamp captured in VESC 100 us ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SystemTimestamp(TimestampTicks);

impl SystemTimestamp {
    /// Wrap timestamp ticks with system timestamp meaning.
    pub const fn new(ticks: TimestampTicks) -> Self {
        Self(ticks)
    }

    /// Return the typed timestamp ticks without erasing them to a primitive.
    pub const fn ticks(self) -> TimestampTicks {
        self.0
    }
}

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
        pub struct $name(AbiSeconds);

        impl $name {
            /// Wrap package ABI seconds with semantic duration meaning.
            pub const fn new(seconds: AbiSeconds) -> Self {
                Self(seconds)
            }

            /// Return the typed seconds without erasing them to a primitive.
            pub const fn duration(self) -> AbiSeconds {
                self.0
            }
        }
    };
}

seconds_type!(TimeoutDuration, "Timeout duration in package ABI seconds.");
seconds_type!(
    RemoteAge,
    "Age of the latest remote input sample in package ABI seconds."
);
seconds_type!(
    PpmAge,
    "Age of the latest PPM input sample in package ABI seconds."
);
