extern "C" fn handler(_: *mut u32, _: u32) -> u32 {
    0
}

fn main() {
    let _ = vescpkg_rs_sys::raw::lbm_add_extension(core::ptr::null(), handler);
}
