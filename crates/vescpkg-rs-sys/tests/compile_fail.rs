//! Compile-fail tests for unsafe and no-std ABI boundaries.

#[test]
#[cfg(not(coverage))]
fn compile_fail_ui() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
