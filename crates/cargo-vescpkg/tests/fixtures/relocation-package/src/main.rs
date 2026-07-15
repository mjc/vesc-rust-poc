#![no_std]

use core::panic::PanicInfo;

#[repr(C)]
pub struct LoaderInfo {
    stop_fun: u32,
    arg: u32,
    base_addr: u32,
}

#[cfg(not(any(
    feature = "marked-image-offset",
    feature = "unmarked-image-offset"
)))]
mod writable_static {
    static REFERENT: u8 = 7;

    #[used]
    #[unsafe(no_mangle)]
    pub static POINTER_BEARING_STATIC: &u8 = &REFERENT;
}

#[cfg(any(
    feature = "marked-image-offset",
    feature = "unmarked-image-offset"
))]
#[unsafe(no_mangle)]
extern "C" fn callback() {}

#[cfg(feature = "marked-image-offset")]
core::arch::global_asm!(
    ".global __vescpkg_image_offset_callback",
    ".set __vescpkg_image_offset_callback, callback",
);

#[unsafe(no_mangle)]
pub extern "C" fn init(info: *const LoaderInfo) -> bool {
    #[cfg(not(any(
        feature = "marked-image-offset",
        feature = "unmarked-image-offset"
    )))]
    {
        let pointer = core::ptr::addr_of!(writable_static::POINTER_BEARING_STATIC);
        // Volatile access keeps the pointer-bearing static in the linked image.
        unsafe {
            core::ptr::read_volatile(pointer);
        }
    }

    #[cfg(any(
        feature = "marked-image-offset",
        feature = "unmarked-image-offset"
    ))]
    {
        let image_offset = core::hint::black_box(callback as *const () as usize);
        // SAFETY: VESC's native loader supplies `LoaderInfo` for the loaded image.
        let loaded_address = unsafe { (*info).base_addr as usize }.wrapping_add(image_offset);
        // SAFETY: The base plus this marked image offset addresses `callback`.
        let callback = unsafe { core::mem::transmute::<usize, extern "C" fn()>(loaded_address) };
        callback();
    }

    true
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
