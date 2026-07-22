//! Float Out Boy typed motor command.

use vescpkg_rs::prelude::MotorCurrent;

/// Float Out Boy motor-current request.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct FloatOutBoyMotorCommand {
    requested_current: MotorCurrent,
}

impl FloatOutBoyMotorCommand {
    /// Build a motor command from typed requested current.
    pub const fn new(requested_current: MotorCurrent) -> Self {
        Self { requested_current }
    }

    /// Return the typed requested current.
    pub const fn requested_current(self) -> MotorCurrent {
        self.requested_current
    }
}
