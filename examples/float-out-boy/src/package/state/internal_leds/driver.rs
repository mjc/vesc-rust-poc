use crate::{
    lcm::{FloatOutBoyHardwareLedsConfig, FloatOutBoyLedStripRole},
    leds::{
        FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
        FloatOutBoyLedRenderer, FloatOutBoyLedStripConfig, FloatOutBoyLedStripFrame,
    },
};
const WS2812_ZERO: u16 = 31;
const WS2812_ONE: u16 = 72;
const WS2812_RESET: u16 = 0;
const MAX_STRIP_PIXELS: usize = 30;
#[cfg(not(target_arch = "arm"))]
const MAX_PULSES: usize = MAX_STRIP_PIXELS * 3 * 32 + 1;

#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub(super) struct FloatOutBoyInternalLedDriver {
    hardware: FloatOutBoyHardwareLedsConfig,
    #[cfg(not(target_arch = "arm"))]
    pulses: [u16; MAX_PULSES],
    #[cfg(target_arch = "arm")]
    pulses: Option<vescpkg_rs::stm32::float_out_boy_ws2812::PulseBuffer>,
    pulse_count: usize,
    initialized: bool,
    operational: bool,
}

impl FloatOutBoyInternalLedDriver {
    pub(super) const fn new(hardware: FloatOutBoyHardwareLedsConfig) -> Self {
        Self {
            hardware,
            #[cfg(not(target_arch = "arm"))]
            pulses: [WS2812_RESET; MAX_PULSES],
            #[cfg(target_arch = "arm")]
            pulses: None,
            pulse_count: 0,
            initialized: false,
            operational: false,
        }
    }

    pub(super) fn setup(
        &mut self,
        setup: impl FnOnce(FloatOutBoyLedPin, FloatOutBoyLedPinConfig, &mut [u16]) -> bool,
        rollback: impl FnOnce(FloatOutBoyLedPin),
    ) -> bool {
        let Some(pulse_count) = self.required_pulse_count() else {
            return false;
        };
        if !self.prepare_pulses(pulse_count) {
            return false;
        }
        self.pulse_count = pulse_count;
        let hardware = self.hardware;
        let Some(pulses) = self.pulses_mut(pulse_count) else {
            self.pulse_count = 0;
            self.release_pulses();
            return false;
        };
        let Some((reset, data)) = pulses.split_last_mut() else {
            self.pulse_count = 0;
            self.release_pulses();
            return false;
        };
        data.fill(WS2812_ZERO);
        *reset = WS2812_RESET;
        self.operational = setup(hardware.pin(), hardware.pin_config(), pulses);
        self.initialized = self.operational;
        if !self.operational {
            rollback(self.hardware.pin());
            self.pulse_count = 0;
            self.release_pulses();
        }
        self.operational
    }

    pub(super) const fn is_operational(&self) -> bool {
        self.operational
    }

    pub(super) fn paint(
        &mut self,
        renderer: &FloatOutBoyLedRenderer,
        quiesce: impl FnOnce(FloatOutBoyLedPin) -> bool,
        restart: impl FnOnce(FloatOutBoyLedPin, &[u16]) -> bool,
    ) -> bool {
        if !self.operational {
            return false;
        }
        if !quiesce(self.hardware.pin()) || !self.encode(renderer) {
            self.operational = false;
            return false;
        }
        let Some(pulses) = self.pulse_slice(self.pulse_count) else {
            self.operational = false;
            return false;
        };
        self.operational = restart(self.hardware.pin(), pulses);
        self.operational
    }

    pub(super) fn destroy(&mut self, teardown: impl FnOnce(FloatOutBoyLedPin) -> bool) -> bool {
        if !self.initialized {
            return true;
        }
        self.operational = false;
        if !teardown(self.hardware.pin()) {
            return false;
        }
        self.initialized = false;
        self.pulse_count = 0;
        self.release_pulses();
        true
    }

    fn required_pulse_count(&self) -> Option<usize> {
        if !self.hardware.uses_internal_leds() {
            return None;
        }
        let layout = self.hardware.internal_layout().ok()?;
        let bits = layout.roles().iter().try_fold(0_usize, |bits, role| {
            let strip = strip_for(self.hardware, *role);
            (usize::from(strip.count()) <= MAX_STRIP_PIXELS)
                .then_some(strip)
                .and_then(|strip| {
                    bits.checked_add(
                        usize::from(strip.count())
                            .checked_mul(bits_per_pixel(strip.color_order()))?,
                    )
                })
        })?;
        (bits > 0).then(|| bits.saturating_add(1))
    }

    #[cfg(not(target_arch = "arm"))]
    const fn prepare_pulses(&mut self, pulse_count: usize) -> bool {
        pulse_count <= self.pulses.len()
    }

    #[cfg(target_arch = "arm")]
    fn prepare_pulses(&mut self, pulse_count: usize) -> bool {
        self.pulses = vescpkg_rs::stm32::float_out_boy_ws2812::PulseBuffer::new(pulse_count);
        self.pulses.is_some()
    }

    #[cfg(not(target_arch = "arm"))]
    fn release_pulses(&mut self) {
        self.pulses.fill(WS2812_RESET);
    }

    #[cfg(target_arch = "arm")]
    fn release_pulses(&mut self) {
        if let Some(pulses) = self.pulses.take() {
            pulses.release();
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn pulses_mut(&mut self, pulse_count: usize) -> Option<&mut [u16]> {
        self.pulses.get_mut(..pulse_count)
    }

    #[cfg(target_arch = "arm")]
    fn pulses_mut(&mut self, pulse_count: usize) -> Option<&mut [u16]> {
        self.pulses.as_mut()?.as_mut_slice(pulse_count)
    }

    #[cfg(not(target_arch = "arm"))]
    fn pulse_slice(&self, pulse_count: usize) -> Option<&[u16]> {
        self.pulses.get(..pulse_count)
    }

    #[cfg(target_arch = "arm")]
    fn pulse_slice(&self, pulse_count: usize) -> Option<&[u16]> {
        self.pulses.as_ref()?.as_slice(pulse_count)
    }

    fn encode(&mut self, renderer: &FloatOutBoyLedRenderer) -> bool {
        let Ok(layout) = self.hardware.internal_layout() else {
            return false;
        };
        let Some(data_pulse_count) = self.pulse_count.checked_sub(1) else {
            return false;
        };
        let hardware = self.hardware;
        let Some(pulses) = self.pulses_mut(data_pulse_count) else {
            return false;
        };
        let mut pulse_index = 0;
        for role in layout.roles() {
            let strip = strip_for(hardware, *role);
            let frame = frame_for(renderer, *role);
            for pixel_index in 0..usize::from(strip.count()) {
                let Some(pixel) = frame.physical_pixel(pixel_index) else {
                    return false;
                };
                for channel in pixel.physical_channels(strip.color_order()).as_slice() {
                    if !encode_byte(pulses, &mut pulse_index, *channel) {
                        return false;
                    }
                }
            }
        }
        pulse_index == data_pulse_count
    }

    #[cfg(test)]
    fn pulses(&self) -> &[u16] {
        self.pulse_slice(self.pulse_count).unwrap_or_default()
    }
}

const fn strip_for(
    hardware: FloatOutBoyHardwareLedsConfig,
    role: FloatOutBoyLedStripRole,
) -> FloatOutBoyLedStripConfig {
    match role {
        FloatOutBoyLedStripRole::Status => hardware.status_strip(),
        FloatOutBoyLedStripRole::Front => hardware.front_strip(),
        FloatOutBoyLedStripRole::Rear => hardware.rear_strip(),
    }
}

fn frame_for(
    renderer: &FloatOutBoyLedRenderer,
    role: FloatOutBoyLedStripRole,
) -> &FloatOutBoyLedStripFrame {
    match role {
        FloatOutBoyLedStripRole::Status => renderer.status(),
        FloatOutBoyLedStripRole::Front => renderer.front(),
        FloatOutBoyLedStripRole::Rear => renderer.rear(),
    }
}

fn encode_byte(pulses: &mut [u16], index: &mut usize, byte: u8) -> bool {
    for bit in (0..8).rev() {
        let mask = 1_u8.wrapping_shl(bit);
        let Some(pulse) = pulses.get_mut(*index) else {
            return false;
        };
        *pulse = if byte & mask == 0 {
            WS2812_ZERO
        } else {
            WS2812_ONE
        };
        *index = index.saturating_add(1);
    }
    true
}

const fn bits_per_pixel(order: FloatOutBoyLedColorOrder) -> usize {
    match order {
        FloatOutBoyLedColorOrder::Grb | FloatOutBoyLedColorOrder::Rgb => 24,
        FloatOutBoyLedColorOrder::Grbw | FloatOutBoyLedColorOrder::Wrgb => 32,
    }
}

#[cfg(test)]
mod tests;
