use vescpkg_rs::{Current, MotorCurrent};

#[test]
fn missing_generated_fields_use_inert_defaults() {
    assert_eq!(super::generated_field::<u16>(None), 0);
    assert!(!super::generated_field::<bool>(None));
    assert_eq!(
        super::generated_field::<MotorCurrent>(None).current(),
        Current::from_amps(0.0)
    );
}
