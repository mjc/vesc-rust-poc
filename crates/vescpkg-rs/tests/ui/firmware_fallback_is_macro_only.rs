use core::ptr::NonNull;
use vescpkg_rs::{PackageStateAccess, PackageStateStore};

struct State;

unsafe fn firmware_state() -> Option<NonNull<State>> {
    None
}

fn main() {
    let runtime = PackageStateStore::new();
    let _ = unsafe { PackageStateAccess::with_firmware_fallback(&runtime, firmware_state) };
}
