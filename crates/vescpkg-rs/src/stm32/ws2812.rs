//! VESC STM32F4 WS2812 peripheral driver.
//!
//! This module supports the official VESC package outputs on B6, B7, C6, and
//! C7, plus Refloat's C9 extension. Each pin resolves to its complete GPIO,
//! timer-channel, and DMA-stream tuple.
//!
//! The low-level entrypoints are unsafe because the SDK cannot lease these raw
//! STM32 peripherals from the firmware. A safe owner must keep the DMA source
//! buffer live, serialize CPU mutation behind quiescence, and retain state when
//! teardown cannot stop DMA.

/// Supported STM32F4 WS2812 output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPin {
    /// STM32 pin B6, TIM4 channel 1, DMA1 stream 0/channel 2.
    B6,
    /// STM32 pin B7, TIM4 channel 2, DMA1 stream 3/channel 2.
    B7,
    /// STM32 pin C6, TIM3 channel 1, DMA1 stream 4/channel 5.
    C6,
    /// STM32 pin C7, TIM3 channel 2, DMA1 stream 5/channel 5.
    C7,
    /// STM32 pin C9, TIM3 channel 4, DMA1 stream 2/channel 5.
    C9,
}

/// WS2812 output-driver electrical mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputDrive {
    /// Configure an open-drain output for an external pull-up.
    OpenDrain,
    /// Configure a push-pull alternate-function output.
    PushPull,
}

const WS2812_ZERO: u16 = 31;
const WS2812_ONE: u16 = 72;
#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
const WS2812_RESET: u16 = 0;

/// Encode one byte with the VESC package WS2812 PWM duty values.
#[must_use]
pub fn encode_byte(pulses: &mut [u16], index: &mut usize, byte: u8) -> bool {
    let Some(output) = pulses.get_mut(*index..) else {
        return false;
    };
    let mut written = 0_usize;
    for (pulse, bit) in output.iter_mut().zip((0..8).rev()) {
        *pulse = if byte & 1_u8.wrapping_shl(bit) == 0 {
            WS2812_ZERO
        } else {
            WS2812_ONE
        };
        written = written.saturating_add(1);
    }
    *index = index.saturating_add(written);
    written == 8
}

/// DMA-backed WS2812 pulse buffer with explicit quiesce and teardown ownership.
#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub struct Ws2812DmaBuffer<const N: usize> {
    storage: super::DmaHalfWordStorage<N>,
    initialized: bool,
    operational: bool,
}

#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
impl<const N: usize> Ws2812DmaBuffer<N> {
    /// Build an unallocated, inactive pulse buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            storage: super::DmaHalfWordStorage::new(),
            initialized: false,
            operational: false,
        }
    }

    /// Prepare zero pixels plus the trailing reset pulse and start DMA.
    #[must_use]
    pub fn setup(
        &mut self,
        pulse_count: usize,
        setup: impl FnOnce(&mut [u16]) -> bool,
        rollback: impl FnOnce(),
    ) -> bool {
        if !self.storage.prepare(pulse_count) {
            return false;
        }
        let Some((reset, data)) = self.storage.as_mut_slice().and_then(<[_]>::split_last_mut)
        else {
            self.storage.release();
            return false;
        };
        data.fill(WS2812_ZERO);
        *reset = WS2812_RESET;
        self.operational = setup(self.storage.as_mut_slice().unwrap_or_default());
        self.initialized = self.operational;
        if !self.operational {
            rollback();
            self.storage.release();
        }
        self.operational
    }

    /// Quiesce DMA, replace pixel pulses, and restart the stable source buffer.
    #[must_use]
    pub fn update(
        &mut self,
        encode: impl FnOnce(&mut [u16]) -> bool,
        quiesce: impl FnOnce() -> bool,
        restart: impl FnOnce(&[u16]) -> bool,
    ) -> bool {
        if !self.operational || !quiesce() {
            self.operational = false;
            return false;
        }
        let encoded = self
            .storage
            .as_mut_slice()
            .and_then(<[_]>::split_last_mut)
            .is_some_and(|(_, data)| encode(data));
        if !encoded {
            self.operational = false;
            return false;
        }
        self.operational = self.storage.as_slice().is_some_and(restart);
        self.operational
    }

    /// Stop DMA and release storage only after teardown succeeds.
    #[must_use]
    pub fn teardown(&mut self, teardown: impl FnOnce() -> bool) -> bool {
        if !self.initialized {
            return true;
        }
        self.operational = false;
        if !teardown() {
            return false;
        }
        self.initialized = false;
        self.storage.release();
        true
    }

    /// Return whether the stream can currently accept an update.
    #[must_use]
    pub const fn is_operational(&self) -> bool {
        self.operational
    }

    /// Borrow host fixture pulses for package-driver characterization.
    #[cfg(all(not(target_arch = "arm"), any(test, feature = "test-support")))]
    #[must_use]
    pub fn pulses_for_test(&self) -> &[u16] {
        self.storage.as_slice().unwrap_or_default()
    }
}

#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
impl<const N: usize> Default for Ws2812DmaBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

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
#[cfg(any(test, target_arch = "arm"))]
const PAL_MODE_ALTERNATE_2_MID_SPEED: u32 = 2 | (2 << 7) | (1 << 3);
#[cfg(any(test, target_arch = "arm"))]
const PAL_OPEN_DRAIN: u32 = 1 << 2;

#[cfg(any(test, target_arch = "arm"))]
const fn output_mode(drive: OutputDrive) -> u32 {
    PAL_MODE_ALTERNATE_2_MID_SPEED
        | if matches!(drive, OutputDrive::OpenDrain) {
            PAL_OPEN_DRAIN
        } else {
            0
        }
}

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
    const fn for_pin(pin: OutputPin) -> Self {
        match pin {
            OutputPin::B6 => Self {
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
            OutputPin::B7 => Self {
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
            OutputPin::C6 => Self {
                gpio: 0x4002_0800,
                gpio_pin: 6,
                timer: 0x4000_0400,
                timer_ccr: TIM_CCR1,
                timer_ccmr: TIM_CCMR1,
                timer_ccmr_shift: 0,
                timer_ccer_shift: 0,
                timer_dma_source: 0x0200,
                rcc_apb1_peripheral: 0x0000_0002,
                dma_stream: DMA1_BASE + 0x70,
                dma_channel: 0x0a00_0000,
                dma_flag_shift: 0,
            },
            OutputPin::C7 => Self {
                gpio: 0x4002_0800,
                gpio_pin: 7,
                timer: 0x4000_0400,
                timer_ccr: TIM_CCR2,
                timer_ccmr: TIM_CCMR1,
                timer_ccmr_shift: 8,
                timer_ccer_shift: 4,
                timer_dma_source: 0x0400,
                rcc_apb1_peripheral: 0x0000_0002,
                dma_stream: DMA1_BASE + 0x88,
                dma_channel: 0x0a00_0000,
                dma_flag_shift: 6,
            },
            OutputPin::C9 => Self {
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

/// Configure and start a circular WS2812 timer/DMA stream.
///
/// # Safety
///
/// The caller must exclusively own the selected GPIO pad, timer channel, and
/// DMA stream. `pulses` must remain allocated at the same address and must not
/// be mutated until [`quiesce`] or a successful [`teardown`] stops DMA.
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn setup(pin: OutputPin, drive: OutputDrive, pulses: &mut [u16]) -> bool {
    // SAFETY: the caller owns the source-mapped tuple and buffer for
    // the complete DMA lifetime, exactly as required by the generic stream.
    unsafe {
        PinHardware::for_pin(pin).stream().setup(
            output_mode(drive),
            PAL_MODE_INPUT,
            TIM_PERIOD,
            pulses,
        )
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
pub unsafe fn quiesce(pin: OutputPin) -> bool {
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
pub unsafe fn restart(pin: OutputPin, pulses: &[u16]) -> bool {
    // SAFETY: the caller has quiesced this exclusively owned tuple and keeps
    // `pulses` stable for the restarted DMA lifetime.
    unsafe { PinHardware::for_pin(pin).stream().restart(pulses) }
}

/// Stop and reset the WS2812 peripherals and return the pad to input.
///
/// # Safety
///
/// The caller must exclusively own the stream created by [`setup`]. When this
/// returns `false`, the DMA source buffer and owning state must remain live.
#[cfg(target_arch = "arm")]
#[must_use]
pub unsafe fn teardown(pin: OutputPin) -> bool {
    // SAFETY: the caller owns the live tuple and retains the source buffer if
    // the generic stream cannot confirm DMA shutdown.
    unsafe { PinHardware::for_pin(pin).stream().teardown(PAL_MODE_INPUT) }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::{
        DMA1_BASE, OutputDrive, OutputPin, PAL_MODE_ALTERNATE_2_MID_SPEED, PAL_OPEN_DRAIN,
        PinHardware, TIM_CCR1, TIM_CCR2, TIM_CCR4, Ws2812DmaBuffer, encode_byte, output_mode,
    };

    #[test]
    fn every_byte_maps_to_the_vesc_ws2812_pulses() {
        let masks = [0x80_u8, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

        for byte in 0_u8..=u8::MAX {
            let mut pulses = [0_u16; 8];
            let mut index = 0;
            assert!(encode_byte(&mut pulses, &mut index, byte));
            assert_eq!(index, 8);
            assert_eq!(
                pulses,
                masks.map(|mask| if byte & mask == 0 { 31 } else { 72 }),
                "wrong pulses for byte {byte:#04x}"
            );
        }

        let mut short = [0_u16; 7];
        let mut index = 0;
        assert!(!encode_byte(&mut short, &mut index, u8::MAX));
        assert_eq!(index, short.len());
        assert_eq!(short, [72; 7]);
    }

    #[test]
    fn official_vesc_pin_map_keeps_its_timer_dma_tuples() {
        let b6 = PinHardware::for_pin(OutputPin::B6);
        assert_eq!(
            (b6.gpio, b6.gpio_pin, b6.timer, b6.timer_ccr, b6.dma_stream),
            (0x4002_0400, 6, 0x4000_0800, TIM_CCR1, DMA1_BASE + 0x10)
        );

        let b7 = PinHardware::for_pin(OutputPin::B7);
        assert_eq!(
            (b7.gpio, b7.gpio_pin, b7.timer, b7.timer_ccr, b7.dma_stream),
            (0x4002_0400, 7, 0x4000_0800, TIM_CCR2, DMA1_BASE + 0x58)
        );

        let c6 = PinHardware::for_pin(OutputPin::C6);
        assert_eq!(
            (c6.gpio, c6.gpio_pin, c6.timer, c6.timer_ccr, c6.dma_stream),
            (0x4002_0800, 6, 0x4000_0400, TIM_CCR1, DMA1_BASE + 0x70)
        );
        assert_eq!(c6.timer_dma_source, 0x0200);
        assert_eq!(c6.dma_channel, 0x0a00_0000);
        assert_eq!(c6.dma_flag_shift, 0);

        let c7 = PinHardware::for_pin(OutputPin::C7);
        assert_eq!(
            (c7.gpio, c7.gpio_pin, c7.timer, c7.timer_ccr, c7.dma_stream),
            (0x4002_0800, 7, 0x4000_0400, TIM_CCR2, DMA1_BASE + 0x88)
        );
        assert_eq!(c7.timer_dma_source, 0x0400);
        assert_eq!(c7.dma_channel, 0x0a00_0000);
        assert_eq!(c7.dma_flag_shift, 6);
    }

    #[test]
    fn refloat_c9_extension_keeps_its_timer_dma_tuple() {
        let c9 = PinHardware::for_pin(OutputPin::C9);
        assert_eq!(
            (c9.gpio, c9.gpio_pin, c9.timer, c9.timer_ccr, c9.dma_stream),
            (0x4002_0800, 9, 0x4000_0400, TIM_CCR4, DMA1_BASE + 0x40)
        );
        assert_eq!(c9.timer_dma_source, 0x1000);
        assert_eq!(c9.dma_channel, 0x0a00_0000);
        assert_eq!(c9.dma_flag_shift, 16);
    }

    #[test]
    fn output_drive_selects_push_pull_or_open_drain() {
        assert_eq!(
            output_mode(OutputDrive::PushPull),
            PAL_MODE_ALTERNATE_2_MID_SPEED
        );
        assert_eq!(
            output_mode(OutputDrive::OpenDrain),
            PAL_MODE_ALTERNATE_2_MID_SPEED | PAL_OPEN_DRAIN
        );
    }

    #[test]
    fn dma_buffer_owns_one_complete_setup_update_and_teardown_interval() {
        let mut buffer = Ws2812DmaBuffer::<9>::new();
        assert!(buffer.setup(
            9,
            |pulses| {
                assert_eq!(pulses, [31, 31, 31, 31, 31, 31, 31, 31, 0]);
                true
            },
            || panic!("successful setup rolled back")
        ));
        assert!(buffer.is_operational());

        assert!(buffer.update(
            |data| {
                assert_eq!(data.len(), 8);
                data.fill(72);
                true
            },
            || true,
            |pulses| {
                assert_eq!(pulses, [72, 72, 72, 72, 72, 72, 72, 72, 0]);
                true
            },
        ));
        assert!(buffer.teardown(|| true));
        assert!(!buffer.is_operational());
        assert!(buffer.storage.is_empty());
    }

    #[test]
    fn dma_buffer_rolls_back_failed_setup_and_retains_storage_until_safe_teardown() {
        let rollbacks = Cell::new(0);
        let mut buffer = Ws2812DmaBuffer::<9>::new();
        assert!(!buffer.setup(9, |_| false, || rollbacks.set(rollbacks.get() + 1)));
        assert_eq!(rollbacks.get(), 1);
        assert!(buffer.storage.is_empty());

        assert!(buffer.setup(9, |_| true, || panic!("successful setup rolled back")));
        assert!(!buffer.update(
            |_| true,
            || false,
            |_| panic!("restart followed failed quiesce")
        ));
        assert!(!buffer.is_operational());
        assert!(!buffer.storage.is_empty());
        assert!(!buffer.teardown(|| false));
        assert!(!buffer.storage.is_empty());
        assert!(buffer.teardown(|| true));
        assert!(buffer.storage.is_empty());
    }
}
