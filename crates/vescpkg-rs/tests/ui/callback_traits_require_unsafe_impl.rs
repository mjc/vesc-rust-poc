use vescpkg_rs::__macro_support::PackageAppDataCallback;

struct Callback;

impl PackageAppDataCallback for Callback {
    fn image_address() -> usize {
        0
    }
}

fn main() {}
