#[cfg(test)]
use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig};

#[cfg(test)]
pub(super) fn setup(
    _pin: FloatOutBoyLedPin,
    _pin_config: FloatOutBoyLedPinConfig,
    _pulses: &mut [u16],
) -> bool {
    true
}

#[cfg(test)]
pub(super) fn quiesce(_pin: FloatOutBoyLedPin) -> bool {
    true
}

#[cfg(test)]
pub(super) fn restart(_pin: FloatOutBoyLedPin, _pulses: &[u16]) -> bool {
    true
}

#[cfg(test)]
pub(super) fn teardown(_pin: FloatOutBoyLedPin) -> bool {
    true
}

#[cfg(target_arch = "arm")]
use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig};
#[cfg(target_arch = "arm")]
use vescpkg_rs::stm32::float_out_boy_ws2812;

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
