//! Declare the custom `coverage` cfg used by compile-fail tests.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");
}
