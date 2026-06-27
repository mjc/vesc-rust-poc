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
        let lifecycle = package_lifecycle::PackageLifecycle::new(ffi::RealBindings);
        let _registered = lifecycle
            .register_extension_from_image(image, package_lifecycle::rust_probe_diag_descriptor());
    }

    true
}
