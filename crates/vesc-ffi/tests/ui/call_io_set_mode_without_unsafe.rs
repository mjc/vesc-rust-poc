fn main() {
    let _ = vesc_ffi::raw::io_set_mode(vesc_ffi::VescPin(0), vesc_ffi::VescPinMode(0));
}
