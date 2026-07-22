//! Explicitly unsafe open-loop FOC controls.

use crate::{DutyCycle, ElectricalSpeed, OpenLoopCurrent, OpenLoopPhase};

/// Failure returned when the loaded firmware does not expose an open-loop slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdvancedFocError {
    /// The requested FOC capability is absent from the loaded function table.
    Unavailable,
}

/// Low-level FOC controls whose physical effects cannot be made safe by Rust.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdvancedFoc;

impl AdvancedFoc {
    /// Construct the advanced FOC capability handle.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Apply an open-loop current command at an electrical speed.
    ///
    /// # Safety
    ///
    /// The caller must ensure the motor, current, and speed are physically
    /// safe for the connected hardware. This bypasses the ordinary closed-loop
    /// motor safety controls.
    pub unsafe fn set_open_loop_current(
        &self,
        current: OpenLoopCurrent,
        speed: ElectricalSpeed,
    ) -> Result<(), AdvancedFocError> {
        unsafe {
            crate::ffi::foc_set_openloop_current(
                current.current().as_amps(),
                speed.rpm().as_revolutions_per_minute(),
            )
        }
        .then_some(())
        .ok_or(AdvancedFocError::Unavailable)
    }

    /// Apply an open-loop current command at an electrical phase.
    ///
    /// # Safety
    ///
    /// The caller must ensure the motor, current, and phase are physically
    /// safe for the connected hardware. This bypasses ordinary closed-loop
    /// motor safety controls.
    pub unsafe fn set_open_loop_phase(
        &self,
        current: OpenLoopCurrent,
        phase: OpenLoopPhase,
    ) -> Result<(), AdvancedFocError> {
        unsafe {
            crate::ffi::foc_set_openloop_phase(
                current.current().as_amps(),
                phase.angle().as_degrees(),
            )
        }
        .then_some(())
        .ok_or(AdvancedFocError::Unavailable)
    }

    /// Apply an open-loop duty command at an electrical speed.
    ///
    /// # Safety
    ///
    /// The caller must ensure the motor, duty, and speed are physically safe
    /// for the connected hardware. This bypasses ordinary closed-loop motor
    /// safety controls.
    pub unsafe fn set_open_loop_duty(
        &self,
        duty: DutyCycle,
        speed: ElectricalSpeed,
    ) -> Result<(), AdvancedFocError> {
        unsafe {
            crate::ffi::foc_set_openloop_duty(
                duty.ratio().as_ratio(),
                speed.rpm().as_revolutions_per_minute(),
            )
        }
        .then_some(())
        .ok_or(AdvancedFocError::Unavailable)
    }

    /// Apply an open-loop duty command at an electrical phase.
    ///
    /// # Safety
    ///
    /// The caller must ensure the motor, duty, and phase are physically safe
    /// for the connected hardware. This bypasses ordinary closed-loop motor
    /// safety controls.
    pub unsafe fn set_open_loop_duty_phase(
        &self,
        duty: DutyCycle,
        phase: OpenLoopPhase,
    ) -> Result<(), AdvancedFocError> {
        unsafe {
            crate::ffi::foc_set_openloop_duty_phase(
                duty.ratio().as_ratio(),
                phase.angle().as_degrees(),
            )
        }
        .then_some(())
        .ok_or(AdvancedFocError::Unavailable)
    }
}

impl crate::Firmware {
    /// Return the explicitly unsafe advanced FOC control surface.
    pub fn advanced_foc(&self) -> AdvancedFoc {
        AdvancedFoc::new()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl crate::test_support::FirmwareTest {
    /// Return the explicitly unsafe advanced FOC control surface.
    pub fn advanced_foc(&self) -> AdvancedFoc {
        AdvancedFoc::new()
    }
}
