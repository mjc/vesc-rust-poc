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

#[cfg(test)]
const WS2812_ZERO: u16 = 31;
#[cfg(test)]
const WS2812_ONE: u16 = 72;
#[cfg(test)]
const WS2812_RESET: u16 = 0;
const MAX_STRIP_PIXELS: usize = 30;
const MAX_PULSES: usize = MAX_STRIP_PIXELS * 3 * 32 + 1;

#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub(super) struct FloatOutBoyInternalLedDriver {
    hardware: FloatOutBoyHardwareLedsConfig,
    buffer: vescpkg_rs::stm32::ws2812::Ws2812DmaBuffer<MAX_PULSES>,
}

impl FloatOutBoyInternalLedDriver {
    pub(super) const fn new(hardware: FloatOutBoyHardwareLedsConfig) -> Self {
        Self {
            hardware,
            buffer: vescpkg_rs::stm32::ws2812::Ws2812DmaBuffer::new(),
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
        let hardware = self.hardware;
        self.buffer.setup(
            pulse_count,
            |pulses| setup(hardware.pin, hardware.pin_config, pulses),
            || rollback(hardware.pin),
        )
    }

    pub(super) const fn is_operational(&self) -> bool {
        self.buffer.is_operational()
    }

    pub(super) fn paint(
        &mut self,
        renderer: &FloatOutBoyLedRenderer,
        quiesce: impl FnOnce(FloatOutBoyLedPin) -> bool,
        restart: impl FnOnce(FloatOutBoyLedPin, &[u16]) -> bool,
    ) -> bool {
        let hardware = self.hardware;
        self.buffer.update(
            |pulses| encode(hardware, renderer, pulses),
            || quiesce(hardware.pin),
            |pulses| restart(hardware.pin, pulses),
        )
    }

    pub(super) fn destroy(&mut self, teardown: impl FnOnce(FloatOutBoyLedPin) -> bool) -> bool {
        self.buffer.teardown(|| teardown(self.hardware.pin))
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

    #[cfg(test)]
    fn pulses(&self) -> &[u16] {
        self.buffer.pulses_for_test()
    }
}

fn encode(
    hardware: FloatOutBoyHardwareLedsConfig,
    renderer: &FloatOutBoyLedRenderer,
    pulses: &mut [u16],
) -> bool {
    let mut pulse_index = 0;
    for (role, strip) in ordered_strips(hardware) {
        let frame = frame_for(renderer, role);
        for pixel_index in 0..usize::from(strip.count) {
            let Some(pixel) = frame.physical_pixel(pixel_index) else {
                return false;
            };
            let (channels, channel_count) = pixel.physical_channels(strip.color_order);
            for channel in channels.into_iter().take(channel_count) {
                if !vescpkg_rs::stm32::ws2812::encode_byte(pulses, &mut pulse_index, channel) {
                    return false;
                }
            }
        }
    }
    pulse_index == pulses.len()
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

const fn bits_per_pixel(order: FloatOutBoyLedColorOrder) -> usize {
    match order {
        FloatOutBoyLedColorOrder::Grb | FloatOutBoyLedColorOrder::Rgb => 24,
        FloatOutBoyLedColorOrder::Grbw | FloatOutBoyLedColorOrder::Wrgb => 32,
    }
}

#[cfg(test)]
mod tests;
