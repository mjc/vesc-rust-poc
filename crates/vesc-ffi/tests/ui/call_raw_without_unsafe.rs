extern "C" fn handler(_: *mut u32, _: u32) -> u32 {
    0
}

fn main() {
    let _ = vesc_ffi::raw::lbm_add_extension(core::ptr::null(), handler);
}
