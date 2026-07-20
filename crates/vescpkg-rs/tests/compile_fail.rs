//! Compile-fail tests for package-author semantic type boundaries.

#[test]
fn semantic_type_compile_fail_ui() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        // cargo-llvm-cov's trybuild_no_target mode leaves dependency paths
        // relative; preserve the canonical paths in the checked-in fixtures.
        unsafe {
            std::env::remove_var("CARGO_LLVM_COV");
            std::env::set_var(
                "CARGO_ENCODED_RUSTFLAGS",
                "--remap-path-prefix=src=<BASE_DIR>/crates/vescpkg-rs/src",
            );
        }
    }

    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
