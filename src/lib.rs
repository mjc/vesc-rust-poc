#![cfg_attr(not(test), no_std)]

pub mod ffi;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cargo_test_smoke() {
        assert_eq!(1 + 1, 2);
    }
}
