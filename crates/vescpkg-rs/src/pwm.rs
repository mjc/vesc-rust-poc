//! Exclusive low-level PWM callback registration.

use core::sync::atomic::{AtomicBool, Ordering};

/// Firmware-compatible PWM callback function.
pub type PwmCallback = unsafe extern "C" fn();

/// Failure returned while registering a PWM callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PwmCallbackError {
    /// The firmware table does not expose PWM callback registration.
    Unavailable,
    /// Another callback lease is still active.
    AlreadyRegistered,
}

static PWM_CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Exclusive ownership of the installed PWM callback.
pub struct PwmCallbackLease {
    _private: (),
}

impl PwmCallbackLease {
    /// Register one callback and return its exclusive lease.
    ///
    /// # Safety
    ///
    /// The callback must remain valid for the lease lifetime and obey the
    /// firmware's interrupt-context restrictions.
    pub unsafe fn register(callback: PwmCallback) -> Result<Self, PwmCallbackError> {
        if PWM_CALLBACK_REGISTERED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(PwmCallbackError::AlreadyRegistered);
        }

        if unsafe { crate::ffi::mc_set_pwm_callback(Some(callback)) } {
            Ok(Self { _private: () })
        } else {
            PWM_CALLBACK_REGISTERED.store(false, Ordering::Release);
            Err(PwmCallbackError::Unavailable)
        }
    }
}

impl Drop for PwmCallbackLease {
    fn drop(&mut self) {
        // Retain ownership if the callback cannot be cleared from firmware.
        if unsafe { crate::ffi::mc_set_pwm_callback(None) } {
            PWM_CALLBACK_REGISTERED.store(false, Ordering::Release);
        }
    }
}
