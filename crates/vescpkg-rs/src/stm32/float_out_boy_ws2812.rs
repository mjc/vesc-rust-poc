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

#[cfg(all(feature = "alloc", target_arch = "arm"))]
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};
#[cfg(target_arch = "arm")]
use core::{ffi::c_void, ptr};

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

/// Firmware-owned pulse storage for the provisional Float Out Boy driver.
///
/// This type is available only for ARM packages that enable `vescpkg-rs`'s
/// `alloc` feature. The owner must call [`Self::release`] only after the DMA
/// stream has stopped successfully.
#[cfg(all(feature = "alloc", target_arch = "arm"))]
#[derive(Debug, PartialEq)]
pub struct PulseBuffer {
    pointer: NonNull<u8>,
    layout: Layout,
    capacity: usize,
}

// SAFETY: ownership moves with the allocation. The package-specific driver
// serializes access and quiesces DMA before CPU mutation or release.
#[cfg(all(feature = "alloc", target_arch = "arm"))]
unsafe impl Send for PulseBuffer {}

#[cfg(all(feature = "alloc", target_arch = "arm"))]
impl PulseBuffer {
    /// Allocate and zero `count` WS2812 timer pulses.
    #[must_use]
    pub fn new(count: usize) -> Option<Self> {
        let layout = Layout::array::<u16>(count).ok()?;
        // SAFETY: `layout` is valid and null reports allocation failure.
        let pointer = unsafe { crate::VescAllocator.alloc_zeroed(layout) };
        Some(Self {
            pointer: NonNull::new(pointer)?,
            layout,
            capacity: count,
        })
    }

    /// Borrow at most `count` initialized pulses for encoding.
    #[must_use]
    pub fn as_mut_slice(&mut self, count: usize) -> Option<&mut [u16]> {
        (count <= self.capacity).then(|| {
            // SAFETY: this value exclusively owns `layout.size()` initialized bytes.
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.layout.size())
            };
            // SAFETY: the allocation was requested with `u16` size and alignment.
            let (prefix, pulses, suffix) = unsafe { bytes.align_to_mut::<u16>() };
            debug_assert!(prefix.is_empty() && suffix.is_empty());
            pulses.get_mut(..count).unwrap_or_default()
        })
    }

    /// Borrow at most `count` initialized pulses for DMA restart.
    #[must_use]
    pub fn as_slice(&self, count: usize) -> Option<&[u16]> {
        (count <= self.capacity).then(|| {
            // SAFETY: the allocation remains live for `layout.size()` initialized bytes.
            let bytes =
                unsafe { core::slice::from_raw_parts(self.pointer.as_ptr(), self.layout.size()) };
            // SAFETY: the allocation was requested with `u16` size and alignment.
            let (prefix, pulses, suffix) = unsafe { bytes.align_to::<u16>() };
            debug_assert!(prefix.is_empty() && suffix.is_empty());
            pulses.get(..count).unwrap_or_default()
        })
    }

    /// Release this allocation after the owning DMA stream has stopped.
    pub fn release(self) {
        // SAFETY: `pointer` came from this allocator with exactly `layout`;
        // consuming ownership prevents a second release.
        unsafe { crate::VescAllocator.dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

#[cfg(target_arch = "arm")]
const RCC_BASE: usize = 0x4002_3800;
#[cfg(target_arch = "arm")]
const RCC_APB1RSTR: usize = RCC_BASE + 0x20;
#[cfg(target_arch = "arm")]
const RCC_AHB1ENR: usize = RCC_BASE + 0x30;
#[cfg(target_arch = "arm")]
const RCC_APB1ENR: usize = RCC_BASE + 0x40;
#[cfg(target_arch = "arm")]
const RCC_AHB1_PERIPH_DMA1: u32 = 0x0020_0000;
#[cfg(any(test, target_arch = "arm"))]
const DMA1_BASE: usize = 0x4002_6000;
#[cfg(target_arch = "arm")]
const DMA1_LISR: usize = DMA1_BASE;
#[cfg(target_arch = "arm")]
const DMA1_LIFCR: usize = DMA1_BASE + 0x08;
#[cfg(target_arch = "arm")]
const DMA_STREAM_CR: usize = 0;
#[cfg(target_arch = "arm")]
const DMA_STREAM_NDTR: usize = 0x04;
#[cfg(target_arch = "arm")]
const DMA_STREAM_PAR: usize = 0x08;
#[cfg(target_arch = "arm")]
const DMA_STREAM_M0AR: usize = 0x0c;
#[cfg(target_arch = "arm")]
const DMA_STREAM_M1AR: usize = 0x10;
#[cfg(target_arch = "arm")]
const DMA_STREAM_FCR: usize = 0x14;
#[cfg(target_arch = "arm")]
const DMA_ENABLE: u32 = 1;
#[cfg(target_arch = "arm")]
const DMA_ERROR_FLAGS: u32 = 0x0d;
#[cfg(target_arch = "arm")]
const DMA_ALL_FLAGS: u32 = 0x3d;
#[cfg(target_arch = "arm")]
const DMA_STREAM_CONFIG: u32 = 0x0002_2c40;
#[cfg(target_arch = "arm")]
const DMA_FIFO_CONFIG: u32 = 0x23;
#[cfg(target_arch = "arm")]
const TIM_CR1: usize = 0;
#[cfg(target_arch = "arm")]
const TIM_DIER: usize = 0x0c;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCMR1: usize = 0x18;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCMR2: usize = 0x1c;
#[cfg(target_arch = "arm")]
const TIM_CCER: usize = 0x20;
#[cfg(target_arch = "arm")]
const TIM_ARR: usize = 0x2c;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR1: usize = 0x34;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR2: usize = 0x38;
#[cfg(any(test, target_arch = "arm"))]
const TIM_CCR4: usize = 0x40;
#[cfg(target_arch = "arm")]
const TIM_PERIOD: u32 = 104;
#[cfg(target_arch = "arm")]
const TIM_PWM1: u32 = 0x60;
#[cfg(target_arch = "arm")]
const TIM_OUTPUT_ENABLE: u32 = 1;
#[cfg(target_arch = "arm")]
const TIM_PRELOAD_ENABLE: u32 = 8;
#[cfg(target_arch = "arm")]
const TIM_AUTO_RELOAD_PRELOAD_ENABLE: u32 = 0x80;
#[cfg(target_arch = "arm")]
const TIM_COUNTER_ENABLE: u32 = 1;
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
    fn pad(self) -> Option<super::Stm32Pad> {
        let gpio = ptr::without_provenance_mut::<c_void>(self.gpio);
        // SAFETY: all three mappings are copied from Refloat v1.2.1's
        // `pin_hw_configs`; the caller owns the selected timer/DMA/pad tuple.
        unsafe { super::Stm32Pad::from_raw_parts(gpio, self.gpio_pin) }
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
    let hardware = PinHardware::for_pin(pin);
    let Some(pad) = hardware.pad() else {
        return false;
    };
    let pin_mode = PAL_MODE_ALTERNATE_2_MID_SPEED
        | if matches!(pin_config, PinConfig::PullupTo5v) {
            PAL_OPEN_DRAIN
        } else {
            0
        };
    let Some(buffer_address) = u32::try_from(pulses.as_mut_ptr().addr()).ok() else {
        return false;
    };
    let Some(pulse_count) = u32::try_from(pulses.len()).ok() else {
        return false;
    };
    let Some(peripheral_address) =
        u32::try_from(hardware.timer.saturating_add(hardware.timer_ccr)).ok()
    else {
        return false;
    };

    set_pad_mode(pad, pin_mode);
    reset_timer(hardware);
    modify(RCC_AHB1ENR, |value| value | RCC_AHB1_PERIPH_DMA1);
    modify(RCC_APB1ENR, |value| value | hardware.rcc_apb1_peripheral);
    if !disable_dma_stream(hardware) {
        set_pad_mode(pad, PAL_MODE_INPUT);
        return false;
    }
    write(register(hardware.dma_stream, DMA_STREAM_M1AR), 0);
    clear_dma_flags(hardware, DMA_ALL_FLAGS);
    write(
        register(hardware.dma_stream, DMA_STREAM_CR),
        hardware.dma_channel | DMA_STREAM_CONFIG,
    );
    write(
        register(hardware.dma_stream, DMA_STREAM_FCR),
        DMA_FIFO_CONFIG,
    );
    write(
        register(hardware.dma_stream, DMA_STREAM_M0AR),
        buffer_address,
    );
    write(register(hardware.dma_stream, DMA_STREAM_NDTR), pulse_count);
    write(
        register(hardware.dma_stream, DMA_STREAM_PAR),
        peripheral_address,
    );
    modify(register(hardware.dma_stream, DMA_STREAM_CR), |value| {
        value | DMA_ENABLE
    });
    initialize_timer(hardware);
    modify(register(hardware.timer, TIM_DIER), |value| {
        value | hardware.timer_dma_source
    });
    true
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
    let hardware = PinHardware::for_pin(pin);
    modify(register(hardware.timer, TIM_DIER), |value| {
        value & !hardware.timer_dma_source
    });
    if !disable_dma_stream(hardware) {
        return false;
    }
    let flags = read(DMA1_LISR) >> hardware.dma_flag_shift;
    clear_dma_flags(hardware, DMA_ALL_FLAGS);
    flags & DMA_ERROR_FLAGS == 0
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
    let hardware = PinHardware::for_pin(pin);
    let Some(buffer_address) = u32::try_from(pulses.as_ptr().addr()).ok() else {
        return false;
    };
    let Some(pulse_count) = u32::try_from(pulses.len()).ok() else {
        return false;
    };
    write(
        register(hardware.dma_stream, DMA_STREAM_M0AR),
        buffer_address,
    );
    write(register(hardware.dma_stream, DMA_STREAM_NDTR), pulse_count);
    modify(register(hardware.dma_stream, DMA_STREAM_CR), |value| {
        value | DMA_ENABLE
    });
    modify(register(hardware.timer, TIM_DIER), |value| {
        value | hardware.timer_dma_source
    });
    true
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
    let hardware = PinHardware::for_pin(pin);
    modify(register(hardware.timer, TIM_DIER), |value| {
        value & !hardware.timer_dma_source
    });
    let stopped = disable_dma_stream(hardware);
    reset_timer(hardware);
    if let Some(pad) = hardware.pad() {
        set_pad_mode(pad, PAL_MODE_INPUT);
    }
    stopped
}

#[cfg(target_arch = "arm")]
fn initialize_timer(hardware: PinHardware) {
    write(register(hardware.timer, TIM_ARR), TIM_PERIOD);
    modify(register(hardware.timer, hardware.timer_ccmr), |value| {
        value
            | (TIM_PWM1 << hardware.timer_ccmr_shift)
            | (TIM_PRELOAD_ENABLE << hardware.timer_ccmr_shift)
    });
    modify(register(hardware.timer, TIM_CCER), |value| {
        value | (TIM_OUTPUT_ENABLE << hardware.timer_ccer_shift)
    });
    modify(register(hardware.timer, TIM_CR1), |value| {
        value | TIM_AUTO_RELOAD_PRELOAD_ENABLE | TIM_COUNTER_ENABLE
    });
}

#[cfg(target_arch = "arm")]
fn reset_timer(hardware: PinHardware) {
    modify(RCC_APB1RSTR, |value| value | hardware.rcc_apb1_peripheral);
    modify(RCC_APB1RSTR, |value| value & !hardware.rcc_apb1_peripheral);
}

#[cfg(target_arch = "arm")]
fn disable_dma_stream(hardware: PinHardware) -> bool {
    let control = register(hardware.dma_stream, DMA_STREAM_CR);
    modify(control, |value| value & !DMA_ENABLE);
    for _ in 0..1_024 {
        if read(control) & DMA_ENABLE == 0 {
            return true;
        }
    }
    false
}

#[cfg(target_arch = "arm")]
fn clear_dma_flags(hardware: PinHardware, flags: u32) {
    write(DMA1_LIFCR, flags << hardware.dma_flag_shift);
}

#[cfg(target_arch = "arm")]
fn read(address: usize) -> u32 {
    // SAFETY: callers provide aligned registers from the fixed Refloat v1.2.1
    // STM32F4 map while the module's unsafe public boundary owns the tuple.
    unsafe { ptr::read_volatile(ptr::without_provenance(address)) }
}

#[cfg(target_arch = "arm")]
fn write(address: usize, value: u32) {
    // SAFETY: callers provide aligned registers from the fixed Refloat v1.2.1
    // STM32F4 map while the module's unsafe public boundary owns the tuple.
    unsafe { ptr::write_volatile(ptr::without_provenance_mut(address), value) };
}

#[cfg(target_arch = "arm")]
fn modify(address: usize, update: impl FnOnce(u32) -> u32) {
    write(address, update(read(address)));
}

#[cfg(any(test, target_arch = "arm"))]
const fn register(base: usize, offset: usize) -> usize {
    base.saturating_add(offset)
}

#[cfg(target_arch = "arm")]
fn set_pad_mode(pad: super::Stm32Pad, mode: u32) {
    // SAFETY: `PinHardware::pad` supplies the source-mapped pad and callers use
    // only Refloat's alternate-output or inert-input modes while owning it.
    unsafe { pad.set_mode(mode) };
}

#[cfg(test)]
mod tests {
    use super::{DMA1_BASE, Pin, PinConfig, PinHardware, TIM_CCR1, TIM_CCR2, TIM_CCR4, register};

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

    #[test]
    fn register_offsets_use_the_fixed_peripheral_base() {
        assert_eq!(register(DMA1_BASE, 0x58), 0x4002_6058);
    }
}
