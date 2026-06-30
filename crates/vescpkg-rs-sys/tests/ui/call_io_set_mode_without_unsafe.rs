fn main() {
    let _ = vescpkg_rs_sys::raw::io_set_mode(vescpkg_rs_sys::VescPin(0), vescpkg_rs_sys::VescPinMode(0));
}
