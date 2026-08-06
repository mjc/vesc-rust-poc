//! Float Out Boy mapping into shared VESC package motor control.

use crate::domain::FloatOutBoyRunState;
pub(crate) use vescpkg_rs::MotorControl as FloatOutBoyMotorControl;
use vescpkg_rs::MotorControlRunState;

impl From<FloatOutBoyRunState> for MotorControlRunState {
    fn from(state: FloatOutBoyRunState) -> Self {
        match state {
            FloatOutBoyRunState::Disabled => Self::Disabled,
            FloatOutBoyRunState::Running => Self::Running,
            FloatOutBoyRunState::Startup | FloatOutBoyRunState::Ready => Self::Idle,
        }
    }
}

#[cfg(test)]
mod tests;
