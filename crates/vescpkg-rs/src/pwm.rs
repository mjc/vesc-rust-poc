//! Exclusive low-level PWM callback registration.

use core::marker::PhantomData;
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
static PWM_CALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Safe behavior for a typed PWM callback.
pub trait PwmCallbackHandler {
    /// Run from the firmware PWM callback context.
    fn on_pwm();
}

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
        register_impl(callback).map(|()| Self { _private: () })
    }

    /// Register a callback whose late provider calls fail closed after drop.
    pub fn register_typed<H: PwmCallbackHandler>()
    -> Result<TypedPwmCallbackLease<H>, PwmCallbackError> {
        register_impl(typed_callback::<H>)?;
        Ok(TypedPwmCallbackLease {
            _handler: PhantomData,
        })
    }
}

impl Drop for PwmCallbackLease {
    fn drop(&mut self) {
        PWM_CALLBACK_ACTIVE.store(false, Ordering::Release);
        // Retain ownership if the callback cannot be cleared from firmware.
        if unsafe { crate::ffi::mc_set_pwm_callback(None) } {
            PWM_CALLBACK_REGISTERED.store(false, Ordering::Release);
        }
    }
}

/// Exclusive ownership of a typed PWM callback.
pub struct TypedPwmCallbackLease<H: PwmCallbackHandler> {
    _handler: PhantomData<fn() -> H>,
}

impl<H: PwmCallbackHandler> Drop for TypedPwmCallbackLease<H> {
    fn drop(&mut self) {
        PWM_CALLBACK_ACTIVE.store(false, Ordering::Release);
        if unsafe { crate::ffi::mc_set_pwm_callback(None) } {
            PWM_CALLBACK_REGISTERED.store(false, Ordering::Release);
        }
    }
}

fn register_impl(callback: PwmCallback) -> Result<(), PwmCallbackError> {
    if PWM_CALLBACK_REGISTERED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(PwmCallbackError::AlreadyRegistered);
    }
    if unsafe { crate::ffi::mc_set_pwm_callback(Some(callback)) } {
        PWM_CALLBACK_ACTIVE.store(true, Ordering::Release);
        Ok(())
    } else {
        PWM_CALLBACK_REGISTERED.store(false, Ordering::Release);
        Err(PwmCallbackError::Unavailable)
    }
}

unsafe extern "C" fn typed_callback<H: PwmCallbackHandler>() {
    if PWM_CALLBACK_ACTIVE.load(Ordering::Acquire) {
        H::on_pwm();
    }
}

#[cfg(test)]
mod tests {
    use super::{PWM_CALLBACK_ACTIVE, PwmCallbackHandler, typed_callback};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    struct Handler;

    impl PwmCallbackHandler for Handler {
        fn on_pwm() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn typed_pwm_callback_fails_closed_after_drop() {
        CALLS.store(0, Ordering::Relaxed);
        PWM_CALLBACK_ACTIVE.store(true, Ordering::Release);
        unsafe { typed_callback::<Handler>() };
        PWM_CALLBACK_ACTIVE.store(false, Ordering::Release);
        unsafe { typed_callback::<Handler>() };
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    }
}
