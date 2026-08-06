//! Provisional Float Out Boy/Refloat WS2812 peripheral driver.
//!
//! This module preserves Refloat v1.2.1's exact B6, B7, and C9 timer/DMA map.
//! It is deliberately named for Float Out Boy: it is not yet the generic VESC
//! WS2812 API. VESC's official `vesc_pkg` WS2812 library at commit
//! `10825f313fd35a798db5ec1f5c9aef2b41f947d3` supports TIM3/TIM4 channels 1/2
//! (normally B6, B7, C6, and C7), while Refloat additionally requires
//! C9/TIM3 channel 4 and different lifecycle behavior.
//!
//! The low-level entrypoints are unsafe because the SDK cannot lease these raw
//! STM32 peripherals from the firmware. Float Out Boy's internal LED driver is
//! the current safe owner: it keeps the DMA source buffer live, serializes CPU
//! mutation behind quiescence, and retains state when teardown cannot stop DMA.

/// Refloat v1.2.1's selectable WS2812 output pin.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pin {
    /// STM32 pin B6, TIM4 channel 1, DMA1 stream 0/channel 2.
    B6 = 0,
    /// STM32 pin B7, TIM4 channel 2, DMA1 stream 3/channel 2.
    B7 = 1,
    /// STM32 pin C9, TIM3 channel 4, DMA1 stream 2/channel 5.
    C9 = 2,
}

impl Pin {
    /// Return the Refloat v1.2.1 configuration ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the Refloat configuration value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Pin {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::B6),
            1 => Ok(Self::B7),
            2 => Ok(Self::C9),
            _ => Err(value),
        }
    }
}

/// Refloat v1.2.1's WS2812 output pull-up configuration.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PinConfig {
    /// Configure an open-drain output for an external 5 V pull-up.
    PullupTo5v = 0,
    /// Configure the alternate-function output without open drain.
    NoPullup = 1,
}

impl PinConfig {
    /// Return the Refloat v1.2.1 configuration ID.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "the repr(u8) discriminant is the Refloat configuration value"
    )]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for PinConfig {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::PullupTo5v),
            1 => Ok(Self::NoPullup),
            _ => Err(value),
        }
    }
}

#[cfg(all(feature = "alloc", target_arch = "arm"))]
pub use super::DmaHalfWordBuffer as PulseBuffer;

#[cfg(any(test, target_arch = "arm"))]
const DMA1_BASE: usize = 0x4002_6000;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCMR1: usize = 0x18;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCMR2: usize = 0x1c;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR1: usize = 0x34;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR2: usize = 0x38;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR4: usize = 0x40;
#[cfg(target_arch = "arm")]
const TIM_PERIOD: u32 = 104;
#[cfg(target_arch = "arm")]
const PAL_MODE_INPUT: u32 = 0;
#[cfg(target_arch = "arm")]
const PAL_MODE_ALTERNATE_2_MID_SPEED: u32 = 2 | (2 << 7) | (1 << 3);
#[cfg(target_arch = "arm")]
const PAL_OPEN_DRAIN: u32 = 1 << 2;

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PinHardware {
    gpio: usize,
    gpio_pin: u32,
    timer: usize,
    timer_ccr: usize,
    timer_ccmr: usize,
    timer_ccmr_shift: u32,
    timer_ccer_shift: u32,
    timer_dma_source: u32,
    rcc_apb1_peripheral: u32,
    dma_stream: usize,
    dma_channel: u32,
    dma_flag_shift: u32,
}

#[cfg(any(test, target_arch = "arm"))]
impl PinHardware {
    const fn for_pin(pin: Pin) -> Self {
        match pin {
            Pin::B6 => Self {
                gpio: 0x4002_0400,
                gpio_pin: 6,
                timer: 0x4000_0800,
                timer_ccr: TIM_CCR1,
                timer_ccmr: TIM_CCMR1,
                timer_ccmr_shift: 0,
                timer_ccer_shift: 0,
                timer_dma_source: 0x0200,
                rcc_apb1_peripheral: 0x0000_0004,
                dma_stream: DMA1_BASE + 0x10,
                dma_channel: 0x0400_0000,
                dma_flag_shift: 0,
            },
            Pin::B7 => Self {
                gpio: 0x4002_0400,
                gpio_pin: 7,
                timer: 0x4000_0800,
                timer_ccr: TIM_CCR2,
                timer_ccmr: TIM_CCMR1,
                timer_ccmr_shift: 8,
                timer_ccer_shift: 4,
                timer_dma_source: 0x0400,
                rcc_apb1_peripheral: 0x0000_0004,
                dma_stream: DMA1_BASE + 0x58,
                dma_channel: 0x0400_0000,
                dma_flag_shift: 22,
            },
            Pin::C9 => Self {
                gpio: 0x4002_0800,
                gpio_pin: 9,
                timer: 0x4000_0400,
                timer_ccr: TIM_CCR4,
                timer_ccmr: TIM_CCMR2,
                timer_ccmr_shift: 8,
                timer_ccer_shift: 12,
                timer_dma_source: 0x1000,
                rcc_apb1_peripheral: 0x0000_0002,
                dma_stream: DMA1_BASE + 0x40,
                dma_channel: 0x0a00_0000,
                dma_flag_shift: 16,
            },
        }
    }

    #[cfg(target_arch = "arm")]
    const fn stream(self) -> super::Stm32F4CircularDmaPwm {
        super::Stm32F4CircularDmaPwm::new(super::Stm32F4CircularDmaPwmConfig {
            gpio: self.gpio,
            gpio_pin: self.gpio_pin,
            timer: self.timer,
            timer_ccr: self.timer_ccr,
            timer_ccmr: self.timer_ccmr,
            timer_ccmr_shift: self.timer_ccmr_shift,
            timer_ccer_shift: self.timer_ccer_shift,
            timer_dma_source: self.timer_dma_source,
            rcc_apb1_peripheral: self.rcc_apb1_peripheral,
            dma_stream: self.dma_stream,
            dma_channel: self.dma_channel,
            dma_flag_shift: self.dma_flag_shift,
        })
    }
}

/// Configure and start Refloat's circular WS2812 timer/DMA stream.
///
/// # Safety
///
/// The caller must exclusively own the selected GPIO pad, timer channel, and
/// DMA stream. `pulses` must remain allocated at the same address and must not
/// be mutated until [`quiesce`] or a successful [`teardown`] stops DMA.
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn setup(pin: Pin, pin_config: PinConfig, pulses: &mut [u16]) -> bool {
    let pin_mode = PAL_MODE_ALTERNATE_2_MID_SPEED
        | if matches!(pin_config, PinConfig::PullupTo5v) {
            PAL_OPEN_DRAIN
        } else {
            0
        };
    // SAFETY: the caller owns the Refloat source-mapped tuple and buffer for
    // the complete DMA lifetime, exactly as required by the generic stream.
    unsafe {
        PinHardware::for_pin(pin)
            .stream()
            .setup(pin_mode, PAL_MODE_INPUT, TIM_PERIOD, pulses)
    }
}

/// Stop DMA so the caller may mutate or replace the pulse buffer.
///
/// # Safety
///
/// The caller must own a live stream previously created by [`setup`] for
/// `pin`, and must keep its source buffer allocated even when this returns
/// `false`.
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn quiesce(pin: Pin) -> bool {
    // SAFETY: the caller retains exclusive ownership and the live source
    // buffer under this function's documented contract.
    unsafe { PinHardware::for_pin(pin).stream().quiesce() }
}

/// Restart DMA with an updated stable pulse buffer.
///
/// # Safety
///
/// The caller must exclusively own the stream, must have successfully
/// quiesced it, and must keep `pulses` live and immutable until the next
/// successful [`quiesce`] or [`teardown`].
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn restart(pin: Pin, pulses: &[u16]) -> bool {
    // SAFETY: the caller has quiesced this exclusively owned tuple and keeps
    // `pulses` stable for the restarted DMA lifetime.
    unsafe { PinHardware::for_pin(pin).stream().restart(pulses) }
}

/// Stop and reset Refloat's WS2812 peripherals and return the pad to input.
///
/// # Safety
///
/// The caller must exclusively own the stream created by [`setup`]. When this
/// returns `false`, the DMA source buffer and owning state must remain live.
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn teardown(pin: Pin) -> bool {
    // SAFETY: the caller owns the live tuple and retains the source buffer if
    // the generic stream cannot confirm DMA shutdown.
    unsafe { PinHardware::for_pin(pin).stream().teardown(PAL_MODE_INPUT) }
}

#[cfg(test)]
mod tests {
    use super::{DMA1_BASE, Pin, PinConfig, PinHardware, TIM_CCR1, TIM_CCR2, TIM_CCR4};

    #[test]
    fn refloat_pin_ids_round_trip_and_reject_unknown_values() {
        for (id, pin) in [Pin::B6, Pin::B7, Pin::C9].into_iter().enumerate() {
            let id = u8::try_from(id).expect("three pins fit in u8");
            assert_eq!(pin.id(), id);
            assert_eq!(Pin::try_from(id), Ok(pin));
        }
        assert_eq!(Pin::try_from(3), Err(3));
        assert_eq!(Pin::try_from(u8::MAX), Err(u8::MAX));
    }

    #[test]
    fn refloat_pin_config_ids_round_trip_and_reject_unknown_values() {
        assert_eq!(PinConfig::PullupTo5v.id(), 0);
        assert_eq!(PinConfig::NoPullup.id(), 1);
        assert_eq!(PinConfig::try_from(0), Ok(PinConfig::PullupTo5v));
        assert_eq!(PinConfig::try_from(1), Ok(PinConfig::NoPullup));
        assert_eq!(PinConfig::try_from(2), Err(2));
    }

    #[test]
    fn refloat_pin_map_keeps_b6_b7_and_c9_timer_dma_tuples() {
        let b6 = PinHardware::for_pin(Pin::B6);
        assert_eq!(
            (b6.gpio, b6.gpio_pin, b6.timer, b6.timer_ccr, b6.dma_stream),
            (0x4002_0400, 6, 0x4000_0800, TIM_CCR1, DMA1_BASE + 0x10)
        );

        let b7 = PinHardware::for_pin(Pin::B7);
        assert_eq!(
            (b7.gpio, b7.gpio_pin, b7.timer, b7.timer_ccr, b7.dma_stream),
            (0x4002_0400, 7, 0x4000_0800, TIM_CCR2, DMA1_BASE + 0x58)
        );

        let c9 = PinHardware::for_pin(Pin::C9);
        assert_eq!(
            (c9.gpio, c9.gpio_pin, c9.timer, c9.timer_ccr, c9.dma_stream),
            (0x4002_0800, 9, 0x4000_0400, TIM_CCR4, DMA1_BASE + 0x40)
        );
        assert_eq!(c9.timer_dma_source, 0x1000);
        assert_eq!(c9.dma_channel, 0x0a00_0000);
        assert_eq!(c9.dma_flag_shift, 16);
    }
}
