use crate::ffi;
use crate::package_lifecycle;

#[used]
#[no_mangle]
#[link_section = ".program_ptr"]
static mut prog_ptr: u32 = 0;

#[no_mangle]
#[link_section = ".init_fun"]
pub extern "C" fn init(info: *mut ffi::LibInfo) -> bool {
    if !crate::package_lib_init(info) {
        return false;
    }

    unsafe {
        let image = ffi::NativeImage::from_info(&*info);
        let descriptor = package_lifecycle::rust_probe_diag_descriptor();
        let handler_addr = image.rebase_addr(descriptor.handler() as *const () as usize);
        let handler = core::mem::transmute(handler_addr);
        let _registered = ffi::raw::lbm_add_extension(descriptor.name().as_ptr(), handler);
    }

    true
}
