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
pub(super) fn teardown(_pin: FloatOutBoyLedPin) {}

#[cfg(target_arch = "arm")]
use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    ptr::{self, NonNull},
};

#[cfg(target_arch = "arm")]
use super::FloatOutBoyInternalLedRuntime;
#[cfg(target_arch = "arm")]
use crate::leds::{FloatOutBoyLedPin, FloatOutBoyLedPinConfig};

#[cfg(target_arch = "arm")]
#[derive(Debug, PartialEq)]
pub(in crate::package::state) struct RuntimeAllocation {
    address: usize,
}

#[cfg(target_arch = "arm")]
impl RuntimeAllocation {
    pub(super) fn allocate() -> Option<Self> {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        // SAFETY: `layout` is valid and null reports allocation failure.
        let pointer = unsafe { vescpkg_rs::VescAllocator.alloc(layout) };
        let pointer = NonNull::new(pointer)?;
        // SAFETY: the allocation has this runtime's exact size and alignment.
        let bytes = unsafe { core::slice::from_raw_parts_mut(pointer.as_ptr(), layout.size()) };
        // SAFETY: the allocation was requested with the runtime's size and alignment.
        let (prefix, slots, suffix) = unsafe {
            bytes.align_to_mut::<core::mem::MaybeUninit<FloatOutBoyInternalLedRuntime>>()
        };
        if !prefix.is_empty() || !suffix.is_empty() || slots.len() != 1 {
            // SAFETY: nothing was initialized and `pointer` still has `layout`.
            unsafe { vescpkg_rs::VescAllocator.dealloc(pointer.as_ptr(), layout) };
            return None;
        }
        if slots.first_mut().is_none() {
            // SAFETY: nothing was initialized and `pointer` still has `layout`.
            unsafe { vescpkg_rs::VescAllocator.dealloc(pointer.as_ptr(), layout) };
            return None;
        }
        Some(Self {
            address: pointer.as_ptr().addr(),
        })
    }

    pub(super) fn initialize(self, runtime: FloatOutBoyInternalLedRuntime) -> Self {
        let pointer = ptr::without_provenance_mut::<u8>(self.address);
        // SAFETY: this value owns one uninitialized allocation of the exact size.
        let bytes = unsafe {
            core::slice::from_raw_parts_mut(
                pointer,
                core::mem::size_of::<FloatOutBoyInternalLedRuntime>(),
            )
        };
        // SAFETY: `allocate` validated this type's exact size and alignment.
        let (_, slots, _) = unsafe {
            bytes.align_to_mut::<core::mem::MaybeUninit<FloatOutBoyInternalLedRuntime>>()
        };
        if let Some(slot) = slots.first_mut() {
            slot.write(runtime);
        }
        self
    }

    pub(super) fn release_uninitialized(self) {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        let pointer = ptr::without_provenance_mut::<u8>(self.address);
        // SAFETY: no runtime was written and this allocation used exactly `layout`.
        unsafe { vescpkg_rs::VescAllocator.dealloc(pointer, layout) };
    }

    pub(super) fn runtime_mut(&mut self) -> Option<&mut FloatOutBoyInternalLedRuntime> {
        let pointer = ptr::without_provenance_mut::<u8>(self.address);
        // SAFETY: this value exclusively owns one initialized runtime allocation.
        let bytes = unsafe {
            core::slice::from_raw_parts_mut(
                pointer,
                core::mem::size_of::<FloatOutBoyInternalLedRuntime>(),
            )
        };
        // SAFETY: construction used this exact type's size and alignment.
        let (prefix, runtime, suffix) =
            unsafe { bytes.align_to_mut::<FloatOutBoyInternalLedRuntime>() };
        (prefix.is_empty() && suffix.is_empty())
            .then(|| runtime.first_mut())
            .flatten()
    }

    pub(super) fn runtime(&self) -> Option<&FloatOutBoyInternalLedRuntime> {
        let pointer = ptr::without_provenance::<u8>(self.address);
        // SAFETY: this value owns one live initialized runtime allocation.
        let bytes = unsafe {
            core::slice::from_raw_parts(
                pointer,
                core::mem::size_of::<FloatOutBoyInternalLedRuntime>(),
            )
        };
        // SAFETY: construction used this exact type's size and alignment.
        let (prefix, runtime, suffix) =
            unsafe { bytes.align_to::<FloatOutBoyInternalLedRuntime>() };
        (prefix.is_empty() && suffix.is_empty())
            .then(|| runtime.first())
            .flatten()
    }

    pub(super) fn release(mut self) {
        let layout = Layout::new::<FloatOutBoyInternalLedRuntime>();
        if let Some(runtime) = self.runtime_mut() {
            // SAFETY: this consumes the sole owner of the initialized runtime.
            unsafe { ptr::drop_in_place(runtime) };
        }
        let pointer = ptr::without_provenance_mut::<u8>(self.address);
        // SAFETY: the runtime is no longer live and this allocation used `layout`.
        unsafe { vescpkg_rs::VescAllocator.dealloc(pointer, layout) };
    }
}

#[cfg(target_arch = "arm")]
#[derive(Debug, PartialEq)]
pub(super) struct PulseAllocation {
    pointer: NonNull<u8>,
    layout: Layout,
    capacity: usize,
}

#[cfg(target_arch = "arm")]
// SAFETY: ownership moves with the allocation, while all access is serialized
// through `PackageStateStore`; DMA is quiesced before CPU mutation or release.
unsafe impl Send for PulseAllocation {}

#[cfg(target_arch = "arm")]
impl PulseAllocation {
    pub(super) fn new(count: usize) -> Option<Self> {
        let layout = Layout::array::<u16>(count).ok()?;
        // SAFETY: `layout` is valid and a null allocation is handled as setup failure.
        let pointer = unsafe { vescpkg_rs::VescAllocator.alloc_zeroed(layout) };
        Some(Self {
            pointer: NonNull::new(pointer)?,
            layout,
            capacity: count,
        })
    }

    pub(super) fn as_mut_slice(&mut self, count: usize) -> Option<&mut [u16]> {
        (count <= self.capacity()).then(|| {
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

    pub(super) fn as_slice(&self, count: usize) -> Option<&[u16]> {
        (count <= self.capacity()).then(|| {
            // SAFETY: the allocation remains live for `layout.size()` initialized bytes.
            let bytes =
                unsafe { core::slice::from_raw_parts(self.pointer.as_ptr(), self.layout.size()) };
            // SAFETY: the allocation was requested with `u16` size and alignment.
            let (prefix, pulses, suffix) = unsafe { bytes.align_to::<u16>() };
            debug_assert!(prefix.is_empty() && suffix.is_empty());
            pulses.get(..count).unwrap_or_default()
        })
    }

    pub(super) fn release(self) {
        // SAFETY: `pointer` came from this allocator with exactly `layout`;
        // consuming ownership prevents a second release.
        unsafe {
            vescpkg_rs::VescAllocator.dealloc(self.pointer.as_ptr(), self.layout);
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
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
#[cfg(target_arch = "arm")]
const TIM_CCMR1: usize = 0x18;
#[cfg(target_arch = "arm")]
const TIM_CCMR2: usize = 0x1c;
#[cfg(target_arch = "arm")]
const TIM_CCER: usize = 0x20;
#[cfg(target_arch = "arm")]
const TIM_ARR: usize = 0x2c;
#[cfg(target_arch = "arm")]
const TIM_CCR1: usize = 0x34;
#[cfg(target_arch = "arm")]
const TIM_CCR2: usize = 0x38;
#[cfg(target_arch = "arm")]
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

#[cfg(target_arch = "arm")]
#[derive(Debug, Clone, Copy)]
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

#[cfg(target_arch = "arm")]
impl PinHardware {
    const fn for_pin(pin: FloatOutBoyLedPin) -> Self {
        match pin {
            FloatOutBoyLedPin::B6 => Self {
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
            FloatOutBoyLedPin::B7 => Self {
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
            FloatOutBoyLedPin::C9 => Self {
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

    fn pad(self) -> Option<vescpkg_rs::stm32::Stm32Pad> {
        let gpio = ptr::without_provenance_mut::<c_void>(self.gpio);
        // SAFETY: all three mappings are copied from Refloat v1.2.1's
        // `pin_hw_configs`; the driver owns the selected timer/DMA/pad tuple.
        unsafe { vescpkg_rs::stm32::Stm32Pad::from_raw_parts(gpio, self.gpio_pin) }
    }
}

#[cfg(target_arch = "arm")]
pub(super) fn setup(
    pin: FloatOutBoyLedPin,
    pin_config: FloatOutBoyLedPinConfig,
    pulses: &mut [u16],
) -> bool {
    let hardware = PinHardware::for_pin(pin);
    let Some(pad) = hardware.pad() else {
        return false;
    };
    let pin_mode = PAL_MODE_ALTERNATE_2_MID_SPEED
        | if matches!(pin_config, FloatOutBoyLedPinConfig::PullupTo5v) {
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

#[cfg(target_arch = "arm")]
pub(super) fn quiesce(pin: FloatOutBoyLedPin) -> bool {
    let hardware = PinHardware::for_pin(pin);
    modify(register(hardware.timer, TIM_DIER), |value| {
        value & !hardware.timer_dma_source
    });
    if !disable_dma_stream(hardware) {
        return false;
    }
    let flags = read(DMA1_LISR) >> hardware.dma_flag_shift;
    clear_dma_flags(hardware, DMA_ALL_FLAGS);
    if flags & DMA_ERROR_FLAGS != 0 {
        return false;
    }
    true
}

#[cfg(target_arch = "arm")]
pub(super) fn restart(pin: FloatOutBoyLedPin, pulses: &[u16]) -> bool {
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

#[cfg(target_arch = "arm")]
pub(super) fn teardown(pin: FloatOutBoyLedPin) {
    let hardware = PinHardware::for_pin(pin);
    modify(register(hardware.timer, TIM_DIER), |value| {
        value & !hardware.timer_dma_source
    });
    let _ = disable_dma_stream(hardware);
    reset_timer(hardware);
    if let Some(pad) = hardware.pad() {
        set_pad_mode(pad, PAL_MODE_INPUT);
    }
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
    // SAFETY: every caller passes an aligned register from the fixed Refloat
    // v1.2.1 STM32F4 peripheral map exclusively owned by this module.
    unsafe { ptr::read_volatile(ptr::without_provenance(address)) }
}

#[cfg(target_arch = "arm")]
fn write(address: usize, value: u32) {
    // SAFETY: every caller passes an aligned register from the fixed Refloat
    // v1.2.1 STM32F4 peripheral map exclusively owned by this module.
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
fn set_pad_mode(pad: vescpkg_rs::stm32::Stm32Pad, mode: u32) {
    // SAFETY: `PinHardware::pad` provides the exclusive source-mapped pad and
    // callers use only Refloat's alternate-output or inert-input modes.
    unsafe { pad.set_mode(mode) };
}
