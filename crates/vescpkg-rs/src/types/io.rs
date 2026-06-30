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
    pub const fn try_new(priority: i8) -> Result<Self, ThreadPriorityError> {
        if priority >= Self::MIN && priority <= Self::MAX {
            Ok(Self(priority))
        } else {
            Err(ThreadPriorityError { value: priority })
        }
    }

    /// Explicitly extract the raw priority.
    pub const fn get(self) -> i8 {
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
    pub const fn value(self) -> i8 {
        self.value
    }
}
