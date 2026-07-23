//! Explicitly unsafe STM32 pad access.
//!
//! This module is intentionally separate from the leased abstract GPIO API
//! and is not re-exported through [`crate::prelude`]. The firmware ABI exposes
//! a raw GPIO port pointer and numeric STM32 pad mode; callers must own the
//! hardware resource and provide source-backed mode values.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::DigitalPin;

/// A resolved STM32 GPIO port/pad pair.
#[derive(Debug, Clone, Copy)]
pub struct Stm32Pad {
    gpio: NonNull<c_void>,
    pin: u32,
}

impl Stm32Pad {
    /// Resolve an abstract VESC pin through the firmware's STM32 mapping.
    ///
    /// # Safety
    ///
    /// The caller must have exclusive ownership of the resolved hardware
    /// resource and must ensure no firmware subsystem concurrently changes it.
    pub unsafe fn from_pin(pin: DigitalPin) -> Option<Self> {
        let mut gpio = ptr::null_mut();
        let mut st_pin = 0;
        let resolved = unsafe { crate::ffi::io_get_st_pin(pin.raw(), &mut gpio, &mut st_pin) };
        resolved.then(|| NonNull::new(gpio).map(|gpio| Self { gpio, pin: st_pin }))?
    }

    /// Return the resolved STM32 pad number.
    #[must_use]
    pub const fn pin(self) -> u32 {
        self.pin
    }

    /// Configure the STM32 pad with a firmware-defined numeric mode.
    ///
    /// # Safety
    ///
    /// `mode` must be a valid mode for the target STM32 port and the caller
    /// must uphold the ownership and electrical-safety requirements of that
    /// mode.
    pub unsafe fn set_mode(self, mode: u32) {
        unsafe { crate::ffi::set_pad_mode(self.gpio.as_ptr(), self.pin, mode) };
    }

    /// Drive the resolved STM32 pad high.
    ///
    /// # Safety
    ///
    /// The caller must own the pad and ensure that driving it high is safe for
    /// the attached hardware.
    pub unsafe fn set(self) {
        unsafe { crate::ffi::set_pad(self.gpio.as_ptr(), self.pin) };
    }

    /// Drive the resolved STM32 pad low.
    ///
    /// # Safety
    ///
    /// The caller must own the pad and ensure that driving it low is safe for
    /// the attached hardware.
    pub unsafe fn clear(self) {
        unsafe { crate::ffi::clear_pad(self.gpio.as_ptr(), self.pin) };
    }
}
