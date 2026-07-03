//! Compile-fail tests for package-author semantic type boundaries.

#[test]
fn semantic_type_compile_fail_ui() {
    // trybuild stderr fixtures are intentionally checked under the flake-pinned
    // Rust toolchain; update them with TRYBUILD=overwrite only with that toolchain.
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
