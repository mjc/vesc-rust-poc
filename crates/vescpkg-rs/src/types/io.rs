//! IO and scheduler semantic tokens.

/// VESC thread priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ThreadPriority(i8);

impl ThreadPriority {
    /// Lowest accepted VESC thread priority.
    pub const MIN: i8 = -5;

    /// Highest accepted VESC thread priority.
    pub const MAX: i8 = 5;

    /// Create a checked thread priority.
    ///
    /// # Errors
    ///
    /// Returns [`ThreadPriorityError`] when `priority` is outside the supported range.
    pub const fn try_new(priority: i8) -> Result<Self, ThreadPriorityError> {
        if priority >= Self::MIN && priority <= Self::MAX {
            Ok(Self(priority))
        } else {
            Err(ThreadPriorityError { value: priority })
        }
    }

    /// Encode the priority for the firmware boundary.
    #[must_use]
    pub const fn as_i8(self) -> i8 {
        self.0
    }
}

/// Error returned when a thread priority is outside -5..=5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadPriorityError {
    value: i8,
}

impl ThreadPriorityError {
    /// Return the rejected priority.
    #[must_use]
    pub const fn value(self) -> i8 {
        self.value
    }
}

impl core::fmt::Display for ThreadPriorityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "thread priority {} is outside -5..=5", self.value)
    }
}

impl core::error::Error for ThreadPriorityError {}

macro_rules! nonzero_u32_token {
    ($name:ident, $error:ident, $doc:literal, $error_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(u32);

        impl $name {
            /// Create a checked non-zero token.
            ///
            /// # Errors
            ///
            /// Returns an error when `value` is zero.
            pub const fn try_new(value: u32) -> Result<Self, $error> {
                if value == 0 {
                    Err($error { value })
                } else {
                    Ok(Self(value))
                }
            }

            /// Encode the token for the firmware boundary.
            pub const fn as_u32(self) -> u32 {
                self.0
            }
        }

        #[doc = $error_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $error {
            value: u32,
        }

        impl $error {
            /// Return the rejected value.
            pub const fn value(self) -> u32 {
                self.value
            }
        }

        impl core::fmt::Display for $error {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{} must be non-zero", stringify!($name))
            }
        }

        impl core::error::Error for $error {}
    };
}

nonzero_u32_token!(
    BaudRate,
    BaudRateError,
    "Serial baud rate.",
    "Error returned when a baud rate is zero."
);
nonzero_u32_token!(
    PacketLength,
    PacketLengthError,
    "Packet length.",
    "Error returned when a packet length is zero."
);
