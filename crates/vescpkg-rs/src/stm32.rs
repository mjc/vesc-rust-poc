//! Explicitly unsafe STM32 peripheral access.
//!
//! This module is intentionally separate from the leased abstract GPIO API
//! and is not re-exported through [`crate::prelude`]. The firmware ABI exposes
//! raw GPIO and timer/DMA register maps; callers must own the hardware
//! resources and provide source-backed configuration values.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::DigitalPin;

mod circular_dma_pwm;
#[cfg(all(feature = "alloc", target_arch = "arm"))]
pub use circular_dma_pwm::DmaHalfWordBuffer;
#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
pub use circular_dma_pwm::DmaHalfWordStorage;
pub use circular_dma_pwm::{Stm32F4CircularDmaPwm, Stm32F4CircularDmaPwmConfig};

/// Provisional Float Out Boy/Refloat WS2812 driver.
///
/// This is intentionally package-specific rather than a generic VESC LED API.
pub mod float_out_boy_ws2812;

#[cfg(test)]
mod circular_dma_pwm_tests {
    use super::{DmaHalfWordStorage, Stm32F4CircularDmaPwm, Stm32F4CircularDmaPwmConfig};

    #[test]
    fn dma_half_word_storage_tracks_exclusive_preparation_and_explicit_release() {
        let mut storage = DmaHalfWordStorage::<4>::new();
        assert!(storage.is_empty());
        assert!(storage.prepare(3));
        assert_eq!(storage.len(), 3);
        assert!(!storage.prepare(2));
        storage
            .as_mut_slice()
            .expect("prepared storage")
            .copy_from_slice(&[1, 2, 3]);
        assert_eq!(storage.as_slice(), Some([1, 2, 3].as_slice()));

        storage.release();
        assert!(storage.is_empty());
        assert_eq!(storage.as_slice(), None);
        assert!(!storage.prepare(5));
    }

    #[test]
    fn circular_dma_pwm_retains_the_source_backed_peripheral_map() {
        let config = Stm32F4CircularDmaPwmConfig {
            gpio: 0x4002_0400,
            gpio_pin: 6,
            timer: 0x4000_0800,
            timer_ccr: 0x34,
            timer_ccmr: 0x18,
            timer_ccmr_shift: 0,
            timer_ccer_shift: 0,
            timer_dma_source: 0x0200,
            rcc_apb1_peripheral: 0x0000_0004,
            dma_stream: 0x4002_6010,
            dma_channel: 0x0400_0000,
            dma_flag_shift: 0,
        };

        assert_eq!(Stm32F4CircularDmaPwm::new(config).config(), config);
    }
}

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
    #[must_use]
    pub unsafe fn from_pin(pin: DigitalPin) -> Option<Self> {
        let mut gpio = ptr::null_mut();
        let mut st_pin = 0;
        let resolved =
            unsafe { crate::ffi::io_get_st_pin(pin.raw(), &raw mut gpio, &raw mut st_pin) };
        resolved.then(|| NonNull::new(gpio).map(|gpio| Self { gpio, pin: st_pin }))?
    }

    /// Build a pad from source-backed STM32 GPIO and pin values.
    ///
    /// # Safety
    ///
    /// `gpio` must be the correct live GPIO peripheral address for the target,
    /// `pin` must belong to that port, and the caller must exclusively own it.
    pub unsafe fn from_raw_parts(gpio: *mut c_void, pin: u32) -> Option<Self> {
        NonNull::new(gpio).map(|gpio| Self { gpio, pin })
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
