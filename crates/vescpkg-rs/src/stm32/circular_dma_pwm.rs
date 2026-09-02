//! Explicitly unsafe STM32F4 circular timer/DMA PWM stream.

#[cfg(all(feature = "alloc", target_arch = "arm"))]
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};
#[cfg(target_arch = "arm")]
use core::{ffi::c_void, ptr};

/// Bounded half-word storage prepared for one DMA ownership interval.
///
/// Host builds retain inline storage for deterministic tests. ARM builds
/// acquire exactly the requested firmware allocation.
#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub struct DmaHalfWordStorage<const N: usize> {
    #[cfg(not(target_arch = "arm"))]
    words: [u16; N],
    #[cfg(all(feature = "alloc", target_arch = "arm"))]
    words: Option<DmaHalfWordBuffer>,
    len: usize,
}

#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
impl<const N: usize> DmaHalfWordStorage<N> {
    /// Build empty storage without allocating firmware memory.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "arm"))]
            words: [0; N],
            #[cfg(all(feature = "alloc", target_arch = "arm"))]
            words: None,
            len: 0,
        }
    }

    /// Prepare exactly `len` half-words for one exclusive DMA owner.
    ///
    /// Preparation fails for zero, values beyond `N`, or storage that has not
    /// been explicitly released from its previous ownership interval.
    #[must_use]
    pub fn prepare(&mut self, len: usize) -> bool {
        if len == 0 || len > N || !self.is_empty() {
            return false;
        }
        #[cfg(not(target_arch = "arm"))]
        self.words.get_mut(..len).unwrap_or_default().fill(0);
        #[cfg(target_arch = "arm")]
        {
            self.words = DmaHalfWordBuffer::new(len);
            if self.words.is_none() {
                return false;
            }
        }
        self.len = len;
        true
    }

    /// Return the prepared half-word count.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether no DMA storage is currently prepared.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrow the prepared words for exclusive CPU mutation.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> Option<&mut [u16]> {
        let len = self.len;
        if len == 0 {
            return None;
        }
        #[cfg(not(target_arch = "arm"))]
        let words = self.words.get_mut(..len);
        #[cfg(target_arch = "arm")]
        let words = self.words.as_mut()?.as_mut_slice(len);
        words
    }

    /// Borrow the prepared words for DMA.
    #[must_use]
    pub fn as_slice(&self) -> Option<&[u16]> {
        if self.len == 0 {
            return None;
        }
        #[cfg(not(target_arch = "arm"))]
        let words = self.words.get(..self.len);
        #[cfg(target_arch = "arm")]
        let words = self.words.as_ref()?.as_slice(self.len);
        words
    }

    /// Release the current ownership interval and its ARM allocation.
    pub fn release(&mut self) {
        #[cfg(not(target_arch = "arm"))]
        self.words.fill(0);
        #[cfg(all(feature = "alloc", target_arch = "arm"))]
        if let Some(words) = self.words.take() {
            words.release();
        }
        self.len = 0;
    }
}

#[cfg(any(not(target_arch = "arm"), feature = "alloc"))]
impl<const N: usize> Default for DmaHalfWordStorage<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Firmware-allocated half-word storage whose release is controlled by a DMA owner.
#[cfg(all(feature = "alloc", target_arch = "arm"))]
#[derive(Debug, PartialEq)]
pub struct DmaHalfWordBuffer {
    pointer: NonNull<u8>,
    layout: Layout,
    capacity: usize,
}

// SAFETY: ownership moves with the allocation. Its external DMA owner must
// serialize access and quiesce DMA before CPU mutation or release.
#[cfg(all(feature = "alloc", target_arch = "arm"))]
unsafe impl Send for DmaHalfWordBuffer {}

#[cfg(all(feature = "alloc", target_arch = "arm"))]
impl DmaHalfWordBuffer {
    /// Allocate and zero `count` DMA half-words.
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

    /// Borrow at most `count` initialized half-words for CPU mutation.
    #[must_use]
    pub fn as_mut_slice(&mut self, count: usize) -> Option<&mut [u16]> {
        (count <= self.capacity).then(|| {
            // SAFETY: this value exclusively owns `layout.size()` initialized bytes.
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.layout.size())
            };
            // SAFETY: the allocation was requested with `u16` size and alignment.
            let (prefix, words, suffix) = unsafe { bytes.align_to_mut::<u16>() };
            debug_assert!(prefix.is_empty() && suffix.is_empty());
            words.get_mut(..count).unwrap_or_default()
        })
    }

    /// Borrow at most `count` initialized half-words for DMA.
    #[must_use]
    pub fn as_slice(&self, count: usize) -> Option<&[u16]> {
        (count <= self.capacity).then(|| {
            // SAFETY: the allocation remains live for `layout.size()` initialized bytes.
            let bytes =
                unsafe { core::slice::from_raw_parts(self.pointer.as_ptr(), self.layout.size()) };
            // SAFETY: the allocation was requested with `u16` size and alignment.
            let (prefix, words, suffix) = unsafe { bytes.align_to::<u16>() };
            debug_assert!(prefix.is_empty() && suffix.is_empty());
            words.get(..count).unwrap_or_default()
        })
    }

    /// Release this allocation after the owning DMA stream has stopped.
    pub fn release(self) {
        // SAFETY: `pointer` came from this allocator with exactly `layout`;
        // consuming ownership prevents a second release.
        unsafe { crate::VescAllocator.dealloc(self.pointer.as_ptr(), self.layout) };
    }
}

/// Source-backed register map for one STM32F4 timer channel and DMA1 stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stm32F4CircularDmaPwmConfig {
    /// GPIO peripheral base address.
    pub gpio: usize,
    /// GPIO pad number.
    pub gpio_pin: u32,
    /// Timer peripheral base address.
    pub timer: usize,
    /// Timer capture/compare register offset.
    pub timer_ccr: usize,
    /// Timer capture/compare mode register offset.
    pub timer_ccmr: usize,
    /// Bit shift for this channel in its mode register.
    pub timer_ccmr_shift: u32,
    /// Bit shift for this channel in the capture/compare enable register.
    pub timer_ccer_shift: u32,
    /// Timer DMA request-enable bit.
    pub timer_dma_source: u32,
    /// Timer reset/clock-enable bit in RCC APB1.
    pub rcc_apb1_peripheral: u32,
    /// DMA1 stream register base address.
    pub dma_stream: usize,
    /// DMA channel selection bits.
    pub dma_channel: u32,
    /// Bit shift for this stream's DMA1 low interrupt flags.
    pub dma_flag_shift: u32,
}

/// Explicitly unsafe owner facade for an STM32F4 circular timer/DMA PWM stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stm32F4CircularDmaPwm(Stm32F4CircularDmaPwmConfig);

impl Stm32F4CircularDmaPwm {
    /// Retain a source-backed peripheral map without touching hardware.
    #[must_use]
    pub const fn new(config: Stm32F4CircularDmaPwmConfig) -> Self {
        Self(config)
    }

    /// Return the retained peripheral map.
    #[must_use]
    pub const fn config(self) -> Stm32F4CircularDmaPwmConfig {
        self.0
    }

    /// Configure and start a circular half-word timer/DMA stream.
    ///
    /// # Safety
    ///
    /// The config must describe live, aligned STM32F4 GPIO, timer, RCC, and
    /// DMA1 registers. The caller must exclusively own them. `words` must stay
    /// allocated at the same address and must not be mutated until
    /// [`Self::quiesce`] or a successful [`Self::teardown`] stops DMA.
    #[cfg(target_arch = "arm")]
    #[must_use]
    pub unsafe fn setup(
        self,
        active_pad_mode: u32,
        inactive_pad_mode: u32,
        timer_period: u32,
        words: &mut [u16],
    ) -> bool {
        let Some(pad) = self.pad() else {
            return false;
        };
        let Some(buffer_address) = u32::try_from(words.as_mut_ptr().addr()).ok() else {
            return false;
        };
        let Some(word_count) = u32::try_from(words.len()).ok() else {
            return false;
        };
        let config = self.0;
        let Some(peripheral_address) =
            u32::try_from(config.timer.saturating_add(config.timer_ccr)).ok()
        else {
            return false;
        };

        set_pad_mode(pad, active_pad_mode);
        self.reset_timer();
        modify(RCC_AHB1ENR, |value| value | RCC_AHB1_PERIPH_DMA1);
        modify(RCC_APB1ENR, |value| value | config.rcc_apb1_peripheral);
        if !self.disable_dma_stream() {
            set_pad_mode(pad, inactive_pad_mode);
            return false;
        }
        write(register(config.dma_stream, DMA_STREAM_M1AR), 0);
        self.clear_dma_flags(DMA_ALL_FLAGS);
        write(
            register(config.dma_stream, DMA_STREAM_CR),
            config.dma_channel | DMA_STREAM_CONFIG,
        );
        write(register(config.dma_stream, DMA_STREAM_FCR), DMA_FIFO_CONFIG);
        write(register(config.dma_stream, DMA_STREAM_M0AR), buffer_address);
        write(register(config.dma_stream, DMA_STREAM_NDTR), word_count);
        write(
            register(config.dma_stream, DMA_STREAM_PAR),
            peripheral_address,
        );
        modify(register(config.dma_stream, DMA_STREAM_CR), |value| {
            value | DMA_ENABLE
        });
        self.initialize_timer(timer_period);
        modify(register(config.timer, TIM_DIER), |value| {
            value | config.timer_dma_source
        });
        true
    }

    /// Stop DMA so the caller may mutate or replace its source buffer.
    ///
    /// # Safety
    ///
    /// The config must remain valid and exclusively owned, and the caller must
    /// keep the source buffer allocated even when this returns `false`.
    #[cfg(target_arch = "arm")]
    #[must_use]
    pub unsafe fn quiesce(self) -> bool {
        let config = self.0;
        modify(register(config.timer, TIM_DIER), |value| {
            value & !config.timer_dma_source
        });
        if !self.disable_dma_stream() {
            return false;
        }
        let flags = read(DMA1_LISR) >> config.dma_flag_shift;
        self.clear_dma_flags(DMA_ALL_FLAGS);
        flags & DMA_ERROR_FLAGS == 0
    }

    /// Restart DMA with an updated stable half-word buffer.
    ///
    /// # Safety
    ///
    /// The config must remain valid and exclusively owned. The caller must
    /// have quiesced the stream and keep `words` live and immutable until the
    /// next successful quiesce or teardown.
    #[cfg(target_arch = "arm")]
    #[must_use]
    pub unsafe fn restart(self, words: &[u16]) -> bool {
        let Some(buffer_address) = u32::try_from(words.as_ptr().addr()).ok() else {
            return false;
        };
        let Some(word_count) = u32::try_from(words.len()).ok() else {
            return false;
        };
        let config = self.0;
        write(register(config.dma_stream, DMA_STREAM_M0AR), buffer_address);
        write(register(config.dma_stream, DMA_STREAM_NDTR), word_count);
        modify(register(config.dma_stream, DMA_STREAM_CR), |value| {
            value | DMA_ENABLE
        });
        modify(register(config.timer, TIM_DIER), |value| {
            value | config.timer_dma_source
        });
        true
    }

    /// Stop/reset the stream and return its pad to `inactive_pad_mode`.
    ///
    /// # Safety
    ///
    /// The config must describe the live, exclusively owned stream created by
    /// [`Self::setup`]. When this returns `false`, its source buffer must remain
    /// live because DMA shutdown could not be confirmed.
    #[cfg(target_arch = "arm")]
    #[must_use]
    pub unsafe fn teardown(self, inactive_pad_mode: u32) -> bool {
        let config = self.0;
        modify(register(config.timer, TIM_DIER), |value| {
            value & !config.timer_dma_source
        });
        let stopped = self.disable_dma_stream();
        self.reset_timer();
        if let Some(pad) = self.pad() {
            set_pad_mode(pad, inactive_pad_mode);
        }
        stopped
    }

    #[cfg(target_arch = "arm")]
    fn pad(self) -> Option<super::Stm32Pad> {
        let config = self.0;
        let gpio = ptr::without_provenance_mut::<c_void>(config.gpio);
        // SAFETY: the public hardware methods require a valid, exclusively
        // owned GPIO peripheral and pad in the retained source-backed config.
        unsafe { super::Stm32Pad::from_raw_parts(gpio, config.gpio_pin) }
    }

    #[cfg(target_arch = "arm")]
    fn initialize_timer(self, period: u32) {
        let config = self.0;
        write(register(config.timer, TIM_ARR), period);
        modify(register(config.timer, config.timer_ccmr), |value| {
            value
                | (TIM_PWM1 << config.timer_ccmr_shift)
                | (TIM_PRELOAD_ENABLE << config.timer_ccmr_shift)
        });
        modify(register(config.timer, TIM_CCER), |value| {
            value | (TIM_OUTPUT_ENABLE << config.timer_ccer_shift)
        });
        modify(register(config.timer, TIM_CR1), |value| {
            value | TIM_AUTO_RELOAD_PRELOAD_ENABLE | TIM_COUNTER_ENABLE
        });
    }

    #[cfg(target_arch = "arm")]
    fn reset_timer(self) {
        let peripheral = self.0.rcc_apb1_peripheral;
        modify(RCC_APB1RSTR, |value| value | peripheral);
        modify(RCC_APB1RSTR, |value| value & !peripheral);
    }

    #[cfg(target_arch = "arm")]
    fn disable_dma_stream(self) -> bool {
        let control = register(self.0.dma_stream, DMA_STREAM_CR);
        modify(control, |value| value & !DMA_ENABLE);
        for _ in 0..1_024 {
            if read(control) & DMA_ENABLE == 0 {
                return true;
            }
        }
        false
    }

    #[cfg(target_arch = "arm")]
    fn clear_dma_flags(self, flags: u32) {
        write(DMA1_LIFCR, flags << self.0.dma_flag_shift);
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
#[cfg(target_arch = "arm")]
const DMA1_LISR: usize = 0x4002_6000;
#[cfg(target_arch = "arm")]
const DMA1_LIFCR: usize = DMA1_LISR + 0x08;
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
#[cfg(target_arch = "arm")]
const TIM_CCER: usize = 0x20;
#[cfg(target_arch = "arm")]
const TIM_ARR: usize = 0x2c;
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
fn read(address: usize) -> u32 {
    // SAFETY: callers supply aligned registers through an unsafe public method
    // whose contract requires a valid STM32F4 peripheral map and ownership.
    unsafe { ptr::read_volatile(ptr::without_provenance(address)) }
}

#[cfg(target_arch = "arm")]
fn write(address: usize, value: u32) {
    // SAFETY: callers supply aligned registers through an unsafe public method
    // whose contract requires a valid STM32F4 peripheral map and ownership.
    unsafe { ptr::write_volatile(ptr::without_provenance_mut(address), value) };
}

#[cfg(target_arch = "arm")]
fn modify(address: usize, update: impl FnOnce(u32) -> u32) {
    write(address, update(read(address)));
}

#[cfg(target_arch = "arm")]
const fn register(base: usize, offset: usize) -> usize {
    base.saturating_add(offset)
}

#[cfg(target_arch = "arm")]
fn set_pad_mode(pad: super::Stm32Pad, mode: u32) {
    // SAFETY: the unsafe public method supplies a source-mapped, exclusively
    // owned pad and a caller-selected valid STM32 mode.
    unsafe { pad.set_mode(mode) };
}
