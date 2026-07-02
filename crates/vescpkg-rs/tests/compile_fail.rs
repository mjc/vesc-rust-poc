//! Compile-fail tests for package-author semantic type boundaries.

#[test]
fn semantic_type_compile_fail_ui() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
