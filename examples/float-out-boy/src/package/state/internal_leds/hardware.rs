#[cfg(not(target_arch = "arm"))]
use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig};

#[cfg(not(target_arch = "arm"))]
pub(super) fn setup(
    _pin: FloatOutBoyLedPin,
    _pin_config: FloatOutBoyLedPinConfig,
    _pulses: &mut [u16],
) -> bool {
    true
}

#[cfg(not(target_arch = "arm"))]
pub(super) fn quiesce(_pin: FloatOutBoyLedPin) -> bool {
    true
}

#[cfg(not(target_arch = "arm"))]
pub(super) fn restart(_pin: FloatOutBoyLedPin, _pulses: &[u16]) -> bool {
    true
}

#[cfg(not(target_arch = "arm"))]
pub(super) fn teardown(_pin: FloatOutBoyLedPin) -> bool {
    true
}

#[cfg(target_arch = "arm")]
use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

#[cfg(target_arch = "arm")]
use super::FloatOutBoyInternalLedRuntime;
#[cfg(target_arch = "arm")]
use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig};
#[cfg(target_arch = "arm")]
use vescpkg_rs::stm32::float_out_boy_ws2812;

#[cfg(target_arch = "arm")]
#[derive(Debug, PartialEq)]
pub(in crate::package::state) struct RuntimeAllocation {
    pointer: NonNull<MaybeUninit<FloatOutBoyInternalLedRuntime>>,
}

// SAFETY: package state serializes access and this owner is the sole handle to its allocation.
#[cfg(target_arch = "arm")]
unsafe impl Send for RuntimeAllocation {}

#[cfg(target_arch = "arm")]
impl RuntimeAllocation {
    pub(super) fn allocate() -> Option<Self> {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        // SAFETY: `layout` is valid and null reports allocation failure.
        let pointer = unsafe { vescpkg_rs::VescAllocator.alloc(layout) };
        let pointer = NonNull::new(pointer)?;
        Some(Self {
            pointer: pointer.cast(),
        })
    }

    pub(super) fn initialize(mut self, runtime: FloatOutBoyInternalLedRuntime) -> Self {
        // SAFETY: this owner has exclusive access to one correctly aligned uninitialized slot.
        unsafe { self.pointer.as_mut().write(runtime) };
        self
    }

    pub(super) fn release_uninitialized(self) {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        // SAFETY: no runtime was written and this allocation used exactly `layout`.
        unsafe {
            vescpkg_rs::VescAllocator.dealloc(self.pointer.cast::<u8>().as_ptr(), layout);
        }
    }

    pub(super) fn runtime_mut(&mut self) -> &mut FloatOutBoyInternalLedRuntime {
        // SAFETY: this value exclusively owns the allocation.
        let slot = unsafe { self.pointer.as_mut() };
        // SAFETY: callers only receive an initialized `RuntimeAllocation`.
        unsafe { slot.assume_init_mut() }
    }

    pub(super) fn runtime(&self) -> &FloatOutBoyInternalLedRuntime {
        // SAFETY: this value exclusively owns the allocation.
        let slot = unsafe { self.pointer.as_ref() };
        // SAFETY: callers only receive an initialized `RuntimeAllocation`.
        unsafe { slot.assume_init_ref() }
    }

    pub(super) fn release(mut self) {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        let runtime = self.runtime_mut();
        // SAFETY: this consumes the sole owner of the initialized runtime.
        unsafe { ptr::drop_in_place(runtime) };
        // SAFETY: the runtime is no longer live and this allocation used `layout`.
        unsafe {
            vescpkg_rs::VescAllocator.dealloc(self.pointer.cast::<u8>().as_ptr(), layout);
        }
    }
}

#[cfg(target_arch = "arm")]
pub(super) fn setup(
    pin: FloatOutBoyLedPin,
    pin_config: FloatOutBoyLedPinConfig,
    pulses: &mut [u16],
) -> bool {
    // SAFETY: the package's internal LED driver owns this pin's timer/DMA tuple
    // and keeps `pulses` allocated and immutable until quiesce or teardown.
    unsafe { float_out_boy_ws2812::setup(pin, pin_config, pulses) }
}

#[cfg(target_arch = "arm")]
pub(super) fn quiesce(pin: FloatOutBoyLedPin) -> bool {
    // SAFETY: only the owning driver calls this for its initialized stream; it
    // retains the pulse allocation when DMA cannot be stopped cleanly.
    unsafe { float_out_boy_ws2812::quiesce(pin) }
}

#[cfg(target_arch = "arm")]
pub(super) fn restart(pin: FloatOutBoyLedPin, pulses: &[u16]) -> bool {
    // SAFETY: the driver restarts only after successful quiescence and keeps
    // its pulse allocation at a stable address until the next stop attempt.
    unsafe { float_out_boy_ws2812::restart(pin, pulses) }
}

#[cfg(target_arch = "arm")]
pub(super) fn teardown(pin: FloatOutBoyLedPin) -> bool {
    // SAFETY: the sole owning driver retains all state when teardown reports
    // that DMA did not stop, so the source allocation cannot be freed early.
    unsafe { float_out_boy_ws2812::teardown(pin) }
}
