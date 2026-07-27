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
    pulses: Option<super::hardware::PulseAllocation>,
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
        if !quiesce(self.hardware.pin()) {
            self.operational = false;
            return false;
        }
        if !self.encode(renderer) {
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

    pub(super) fn destroy(&mut self, teardown: impl FnOnce(FloatOutBoyLedPin)) {
        if !self.initialized {
            return;
        }
        teardown(self.hardware.pin());
        self.initialized = false;
        self.operational = false;
        self.pulse_count = 0;
        self.release_pulses();
    }

    fn required_pulse_count(&self) -> Option<usize> {
        if !self.hardware.uses_internal_leds() {
            return None;
        }
        let layout = self.hardware.internal_layout().ok()?;
        let bits = layout.roles().iter().try_fold(0_usize, |bits, role| {
            let strip = self.strip(*role);
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
        self.pulses = super::hardware::PulseAllocation::new(pulse_count);
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

    const fn strip(&self, role: FloatOutBoyLedStripRole) -> FloatOutBoyLedStripConfig {
        match role {
            FloatOutBoyLedStripRole::Status => self.hardware.status_strip(),
            FloatOutBoyLedStripRole::Front => self.hardware.front_strip(),
            FloatOutBoyLedStripRole::Rear => self.hardware.rear_strip(),
        }
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
        let Some(mask) = 1_u8.checked_shl(bit) else {
            return false;
        };
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
mod tests {
    use core::cell::Cell;

    use crate::{
        FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyRunState,
        lcm::{FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode},
        leds::{
            FloatOutBoyLedAnimationMode, FloatOutBoyLedAnimationSpeed, FloatOutBoyLedBarConfig,
            FloatOutBoyLedColor, FloatOutBoyLedColorOrder, FloatOutBoyLedFrameUpdate,
            FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedRenderer,
            FloatOutBoyLedStatusUpdate, FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
            FloatOutBoyLedUpdate, FloatOutBoyLedsConfig, FloatOutBoyStatusBarConfig,
            FloatOutBoyStatusBarIdleTimeout,
        },
    };
    use vescpkg_rs::Ratio;

    use super::{FloatOutBoyInternalLedDriver, WS2812_ONE, WS2812_RESET, WS2812_ZERO};

    fn solid_bar(color: FloatOutBoyLedColor) -> FloatOutBoyLedBarConfig {
        FloatOutBoyLedBarConfig::new(
            Ratio::from_ratio_const(1.0),
            color,
            FloatOutBoyLedColor::Black,
            FloatOutBoyLedAnimationMode::Solid,
            FloatOutBoyLedAnimationSpeed::from_units(1.0),
        )
    }

    fn enabled_config() -> FloatOutBoyLedsConfig {
        let black = solid_bar(FloatOutBoyLedColor::Black);
        FloatOutBoyLedsConfig::new(
            black,
            black,
            solid_bar(FloatOutBoyLedColor::Red),
            black,
            FloatOutBoyStatusBarConfig::new(
                FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
                Ratio::from_ratio_const(0.5),
                Ratio::from_ratio_const(0.2),
                Ratio::from_ratio_const(0.2),
                Ratio::from_ratio_const(0.5),
            ),
            black,
        )
        .enabled()
    }

    #[test]
    fn setup_builds_source_sized_zero_pulse_buffer() {
        let strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            2,
            FloatOutBoyLedColorOrder::Grbw,
        );
        let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_pin(FloatOutBoyLedPin::C9)
            .with_pin_config(FloatOutBoyLedPinConfig::NoPullup)
            .with_front_strip(strip)
            .with_status_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ))
            .with_rear_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ));
        let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
        let mut setup = None;

        assert!(driver.setup(
            |pin, pin_config, pulses| {
                setup = Some((pin, pin_config, pulses.len()));
                true
            },
            |_| panic!("successful setup rolled back"),
        ));

        assert_eq!(
            setup,
            Some((FloatOutBoyLedPin::C9, FloatOutBoyLedPinConfig::NoPullup, 65,))
        );
        assert_eq!(driver.pulses()[..64], [WS2812_ZERO; 64]);
        assert_eq!(driver.pulses()[64], WS2812_RESET);
        assert!(driver.is_operational());
    }

    #[test]
    fn setup_rejects_zero_and_out_of_range_strip_counts_without_touching_hardware() {
        let zero = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_status_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ))
            .with_front_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ))
            .with_rear_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ));
        let too_many = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_status_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::First,
                31,
                FloatOutBoyLedColorOrder::Grb,
            ));

        for hardware in [zero, too_many] {
            let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
            assert!(!driver.setup(
                |_, _, _| panic!("invalid layout reached hardware"),
                |_| panic!("hardware teardown ran without setup"),
            ));
            assert!(!driver.is_operational());
        }
    }

    #[test]
    fn paint_encodes_gamma_ordered_renderer_pixels_and_restarts_once() {
        let strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            1,
            FloatOutBoyLedColorOrder::Grb,
        );
        let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_front_strip(strip)
            .with_status_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ))
            .with_rear_strip(FloatOutBoyLedStripConfig::new(
                FloatOutBoyLedStripOrder::None,
                0,
                FloatOutBoyLedColorOrder::Grb,
            ));
        let config = enabled_config();
        let mut renderer = FloatOutBoyLedRenderer::new(hardware, config, 0.0);
        let frame = FloatOutBoyLedFrameUpdate::new(
            FloatOutBoyLedUpdate {
                run_state: FloatOutBoyRunState::Ready,
                mode: FloatOutBoyMode::Normal,
                darkride: false,
                footpad: FloatOutBoyFootpadState::None,
                pitch_degrees: 0.0,
                distance: 0.0,
            },
            FloatOutBoyLedStatusUpdate {
                battery_level: 1.0,
                duty_cycle: 0.0,
                moving: false,
            },
        );
        for _ in 0..10 {
            renderer.update(config, frame, 0.0);
        }
        let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
        assert!(driver.setup(|_, _, _| true, |_| panic!("successful setup rolled back")));
        let mut restarted = 0;

        assert!(driver.paint(
            &renderer,
            |_| true,
            |_, pulses| {
                restarted += 1;
                assert_eq!(pulses[..8], [WS2812_ZERO; 8]);
                assert_eq!(pulses[8..16], [WS2812_ONE; 8]);
                assert_eq!(pulses[16..24], [WS2812_ZERO; 8]);
                assert_eq!(pulses[24], WS2812_RESET);
                true
            }
        ));

        assert_eq!(restarted, 1);
    }

    #[test]
    fn failed_setup_and_successful_destroy_teardown_exactly_once() {
        let strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            1,
            FloatOutBoyLedColorOrder::Grb,
        );
        let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_front_strip(strip);
        let mut failed = FloatOutBoyInternalLedDriver::new(hardware);
        let mut failed_teardowns = 0;

        assert!(!failed.setup(|_, _, _| false, |_| failed_teardowns += 1,));
        failed.destroy(|_| failed_teardowns += 1);
        assert_eq!(failed_teardowns, 1);

        let mut active = FloatOutBoyInternalLedDriver::new(hardware);
        assert!(active.setup(|_, _, _| true, |_| {}));
        let mut active_teardowns = 0;
        active.destroy(|_| active_teardowns += 1);
        active.destroy(|_| active_teardowns += 1);
        assert_eq!(active_teardowns, 1);
        assert!(!active.is_operational());
    }

    #[test]
    fn paint_quiesces_before_mutating_pulses_and_faults_still_teardown() {
        let strip = FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            1,
            FloatOutBoyLedColorOrder::Grb,
        );
        let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
            .with_front_strip(strip);
        let config = enabled_config();
        let renderer = FloatOutBoyLedRenderer::new(hardware, config, 0.0);
        let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
        assert!(driver.setup(|_, _, _| true, |_| {}));
        let quiesced = Cell::new(false);

        assert!(!driver.paint(
            &renderer,
            |_| {
                quiesced.set(true);
                false
            },
            |_, _| panic!("failed quiesce restarted DMA")
        ));
        assert!(quiesced.get());
        assert!(!driver.is_operational());

        let mut teardowns = 0;
        driver.destroy(|_| teardowns += 1);
        driver.destroy(|_| teardowns += 1);
        assert_eq!(teardowns, 1);
    }
}
