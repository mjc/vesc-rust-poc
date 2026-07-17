#![allow(missing_docs)]

fn main() {
    vescpkg_build_support::build_package(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}
