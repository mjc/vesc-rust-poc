use crate::{
    lcm::FloatOutBoyHardwareLedsConfig,
    leds::{
        FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
        FloatOutBoyLedRenderer, FloatOutBoyLedStripConfig, FloatOutBoyLedStripFrame,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyLedStripRole {
    Status,
    Front,
    Rear,
}

fn ordered_strips(
    hardware: FloatOutBoyHardwareLedsConfig,
) -> impl Iterator<Item = (FloatOutBoyLedStripRole, FloatOutBoyLedStripConfig)> {
    let strips = [
        (FloatOutBoyLedStripRole::Status, hardware.status),
        (FloatOutBoyLedStripRole::Front, hardware.front),
        (FloatOutBoyLedStripRole::Rear, hardware.rear),
    ];
    [
        crate::leds::FloatOutBoyLedStripOrder::First,
        crate::leds::FloatOutBoyLedStripOrder::Second,
        crate::leds::FloatOutBoyLedStripOrder::Third,
    ]
    .into_iter()
    .filter_map(move |order| {
        strips
            .into_iter()
            .find(|(_, strip)| strip.order == order && strip.count > 0)
    })
}

const WS2812_ZERO: u16 = 31;
const WS2812_ONE: u16 = 72;
const WS2812_RESET: u16 = 0;
const MAX_STRIP_PIXELS: usize = 30;
const MAX_PULSES: usize = MAX_STRIP_PIXELS * 3 * 32 + 1;

#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub(super) struct FloatOutBoyInternalLedDriver {
    hardware: FloatOutBoyHardwareLedsConfig,
    pulses: vescpkg_rs::stm32::DmaHalfWordStorage<MAX_PULSES>,
    initialized: bool,
    operational: bool,
}

impl FloatOutBoyInternalLedDriver {
    pub(super) const fn new(hardware: FloatOutBoyHardwareLedsConfig) -> Self {
        Self {
            hardware,
            pulses: vescpkg_rs::stm32::DmaHalfWordStorage::new(),
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
        if !self.pulses.prepare(pulse_count) {
            return false;
        }
        let hardware = self.hardware;
        let Some(pulses) = self.pulses.as_mut_slice() else {
            self.pulses.release();
            return false;
        };
        let Some((reset, data)) = pulses.split_last_mut() else {
            self.pulses.release();
            return false;
        };
        data.fill(WS2812_ZERO);
        *reset = WS2812_RESET;
        self.operational = setup(hardware.pin, hardware.pin_config, pulses);
        self.initialized = self.operational;
        if !self.operational {
            rollback(self.hardware.pin);
            self.pulses.release();
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
        if !quiesce(self.hardware.pin) || !self.encode(renderer) {
            self.operational = false;
            return false;
        }
        let Some(pulses) = self.pulses.as_slice() else {
            self.operational = false;
            return false;
        };
        self.operational = restart(self.hardware.pin, pulses);
        self.operational
    }

    pub(super) fn destroy(&mut self, teardown: impl FnOnce(FloatOutBoyLedPin) -> bool) -> bool {
        if !self.initialized {
            return true;
        }
        self.operational = false;
        if !teardown(self.hardware.pin) {
            return false;
        }
        self.initialized = false;
        self.pulses.release();
        true
    }

    fn required_pulse_count(&self) -> Option<usize> {
        if !matches!(
            self.hardware.mode,
            crate::lcm::FloatOutBoyLedMode::Internal | crate::lcm::FloatOutBoyLedMode::Both
        ) {
            return None;
        }
        let bits = ordered_strips(self.hardware).try_fold(0_usize, |bits, (_, strip)| {
            (usize::from(strip.count) <= MAX_STRIP_PIXELS)
                .then_some(strip)
                .and_then(|strip| {
                    bits.checked_add(
                        usize::from(strip.count).checked_mul(bits_per_pixel(strip.color_order))?,
                    )
                })
        })?;
        (bits > 0).then(|| bits.saturating_add(1))
    }

    fn encode(&mut self, renderer: &FloatOutBoyLedRenderer) -> bool {
        let Some(data_pulse_count) = self.pulses.len().checked_sub(1) else {
            return false;
        };
        let hardware = self.hardware;
        let Some(pulses) = self
            .pulses
            .as_mut_slice()
            .and_then(|pulses| pulses.get_mut(..data_pulse_count))
        else {
            return false;
        };
        let mut pulse_index = 0;
        for (role, strip) in ordered_strips(hardware) {
            let frame = frame_for(renderer, role);
            for pixel_index in 0..usize::from(strip.count) {
                let Some(pixel) = frame.physical_pixel(pixel_index) else {
                    return false;
                };
                let (channels, channel_count) = pixel.physical_channels(strip.color_order);
                for channel in channels.into_iter().take(channel_count) {
                    if !encode_byte(pulses, &mut pulse_index, channel) {
                        return false;
                    }
                }
            }
        }
        pulse_index == data_pulse_count
    }

    #[cfg(test)]
    fn pulses(&self) -> &[u16] {
        self.pulses.as_slice().unwrap_or_default()
    }
}

fn frame_for(
    renderer: &FloatOutBoyLedRenderer,
    role: FloatOutBoyLedStripRole,
) -> &FloatOutBoyLedStripFrame {
    match role {
        FloatOutBoyLedStripRole::Status => &renderer.status,
        FloatOutBoyLedStripRole::Front => &renderer.front,
        FloatOutBoyLedStripRole::Rear => &renderer.rear,
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
