fn main() {
    let _ = vescpkg_sys::raw::io_set_mode(vescpkg_sys::VescPin(0), vescpkg_sys::VescPinMode(0));
}
