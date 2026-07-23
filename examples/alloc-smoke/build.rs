//! Builds the allocator smoke-test VESC package metadata.

#![deny(warnings, clippy::all, clippy::pedantic)]

fn main() {
    vescpkg_build_support::build_package(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}
