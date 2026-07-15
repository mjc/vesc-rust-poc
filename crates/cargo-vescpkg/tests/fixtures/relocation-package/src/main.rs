#![no_main]
#![no_std]

use core::panic::PanicInfo;

static REFERENT: u8 = 7;
#[used]
#[unsafe(no_mangle)]
pub static POINTER_BEARING_STATIC: &u8 = &REFERENT;

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    let pointer = core::ptr::addr_of!(POINTER_BEARING_STATIC);
    // Volatile access keeps the pointer-bearing static in the linked image.
    unsafe {
        core::ptr::read_volatile(pointer);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
