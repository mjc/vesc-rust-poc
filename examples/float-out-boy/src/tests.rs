mod package_author;

use vescpkg_rs::test_support::LoaderInfo;

#[test]
fn float_assertions_handle_non_finite_values_without_nan_arithmetic() {
    assert_f32_eq!(f32::INFINITY, f32::INFINITY);
    assert_f32_eq!(f32::NEG_INFINITY, f32::NEG_INFINITY);
    assert_f32_eq!(0.0, -0.0);
    assert_f32_ne!(f32::INFINITY, f32::NEG_INFINITY);
    assert_f32_ne!(f32::NAN, f32::NAN);
}

#[test]
fn package_lib_init_runs_float_out_boy_start() {
    let _runtime_state = crate::package::test_support::lock_float_out_boy_runtime_state();
    let mut info = LoaderInfo::new();

    assert!(super::package_lib_init(&raw mut info));
    assert!(info.argument().is_some());
    assert!(vescpkg_rs::test_support::stop_package(&mut info));
}
