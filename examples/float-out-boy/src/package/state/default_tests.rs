use pin_init::PtrPinWith;

#[test]
fn in_place_default_matches_movable_default() {
    let state = Box::pin_with(super::FloatOutBoyPackageState::default_in_place())
        .expect("infallible initializer");

    assert_eq!(
        state.as_ref().get_ref(),
        &super::FloatOutBoyPackageState::default()
    );
}
