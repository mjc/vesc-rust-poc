//! Exclusive package-owned custom encoder callbacks.

use core::ffi::{CStr, c_char};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::AngleDegrees;

static ENCODER_OWNED: AtomicBool = AtomicBool::new(false);
static ENCODER_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn disable_callback_dispatch() {
    ENCODER_ACTIVE.store(false, Ordering::Release);
}

/// Failure returned by encoder callback registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncoderError {
    /// The encoder callback slot is unavailable.
    Unavailable,
    /// Another package currently owns custom encoder callbacks.
    Busy,
}

/// Safe callback behavior for one custom encoder provider.
pub trait EncoderHandler {
    /// Return the current encoder position in degrees.
    fn read_degrees() -> AngleDegrees;
    /// Return whether the encoder currently reports a fault.
    fn has_fault() -> bool;
    /// Return a static NUL-terminated encoder description.
    fn info() -> &'static CStr;
}

/// Optional custom encoder capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Encoder;

/// Exclusive custom encoder callback registration.
pub struct EncoderRegistration<H: EncoderHandler> {
    _handler: PhantomData<H>,
}

impl Encoder {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Install one package-owned custom encoder callback set.
    pub fn register<H: EncoderHandler>(&self) -> Result<EncoderRegistration<H>, EncoderError> {
        if ENCODER_OWNED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(EncoderError::Busy);
        }
        let installed =
            unsafe { crate::ffi::encoder_set_custom_callbacks(read::<H>, fault::<H>, info::<H>) };
        if !installed {
            ENCODER_OWNED.store(false, Ordering::Release);
            return Err(EncoderError::Unavailable);
        }
        ENCODER_ACTIVE.store(true, Ordering::Release);
        Ok(EncoderRegistration {
            _handler: PhantomData,
        })
    }
}

impl<H: EncoderHandler> Drop for EncoderRegistration<H> {
    fn drop(&mut self) {
        ENCODER_ACTIVE.store(false, Ordering::Release);
        // Do not admit a second provider when firmware rejected the disable set.
        let cleared = unsafe {
            crate::ffi::encoder_set_custom_callbacks(disabled_read, disabled_fault, disabled_info)
        };
        if cleared {
            ENCODER_OWNED.store(false, Ordering::Release);
        }
    }
}

unsafe extern "C" fn read<H: EncoderHandler>() -> f32 {
    if !ENCODER_ACTIVE.load(Ordering::Acquire) {
        return 0.0;
    }
    H::read_degrees().as_degrees()
}

unsafe extern "C" fn fault<H: EncoderHandler>() -> bool {
    if !ENCODER_ACTIVE.load(Ordering::Acquire) {
        return true;
    }
    H::has_fault()
}

unsafe extern "C" fn info<H: EncoderHandler>() -> *mut c_char {
    if !ENCODER_ACTIVE.load(Ordering::Acquire) {
        return unsafe { disabled_info() };
    }
    H::info().as_ptr().cast_mut()
}

unsafe extern "C" fn disabled_read() -> f32 {
    0.0
}
unsafe extern "C" fn disabled_fault() -> bool {
    true
}
unsafe extern "C" fn disabled_info() -> *mut c_char {
    c"".as_ptr().cast_mut()
}

impl crate::Firmware {
    /// Return the optional custom encoder capability handle.
    pub fn encoder(&self) -> Encoder {
        Encoder::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional custom encoder capability handle.
    pub fn encoder(&self) -> Encoder {
        Encoder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ENCODER_ACTIVE, EncoderHandler, fault, info, read};
    use crate::AngleDegrees;
    use core::ffi::CStr;
    use core::sync::atomic::Ordering;

    struct Handler;

    impl EncoderHandler for Handler {
        fn read_degrees() -> AngleDegrees {
            AngleDegrees::from_degrees(12.0)
        }

        fn has_fault() -> bool {
            false
        }

        fn info() -> &'static CStr {
            c"test-encoder"
        }
    }

    #[test]
    fn late_encoder_callbacks_after_drop_fail_closed() {
        ENCODER_ACTIVE.store(true, Ordering::Release);
        assert_eq!(unsafe { read::<Handler>() }, 12.0);
        assert!(!unsafe { fault::<Handler>() });
        ENCODER_ACTIVE.store(false, Ordering::Release);
        assert_eq!(unsafe { read::<Handler>() }, 0.0);
        assert!(unsafe { fault::<Handler>() });
        assert_eq!(unsafe { CStr::from_ptr(info::<Handler>()) }, c"");
    }
}
