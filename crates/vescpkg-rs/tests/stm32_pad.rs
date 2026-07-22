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
