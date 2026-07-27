#![cfg(feature = "test-support")]
//! Integration coverage for the explicitly unsafe STM32 pad surface.

use vescpkg_rs::{DigitalPin, stm32::Stm32Pad};

#[test]
fn stm32_pad_resolution_and_mutation_are_explicitly_unsafe() {
    let pad = unsafe { Stm32Pad::from_pin(DigitalPin::HW_1) }.expect("pinned pad resolves");
    assert_eq!(pad.pin(), 13);

    unsafe {
        pad.set_mode(3);
        pad.set();
        pad.clear();
    }
}

#[test]
fn stm32_pad_raw_parts_reject_null_and_preserve_physical_pin() {
    let gpio = core::ptr::NonNull::<u32>::dangling().as_ptr().cast();

    assert!(unsafe { Stm32Pad::from_raw_parts(core::ptr::null_mut(), 7) }.is_none());
    assert_eq!(
        unsafe { Stm32Pad::from_raw_parts(gpio, 9) }
            .expect("non-null raw GPIO")
            .pin(),
        9
    );
}
