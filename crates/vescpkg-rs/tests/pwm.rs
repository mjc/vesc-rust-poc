#![cfg(feature = "test-support")]
//! Integration coverage for typed PWM callback ownership.

use vescpkg_rs::{PwmCallbackHandler, PwmCallbackLease};

struct Handler;

impl PwmCallbackHandler for Handler {
    fn on_pwm() {}
}

#[test]
fn typed_pwm_registration_is_exclusive_and_released_on_drop() {
    let _firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let first = PwmCallbackLease::register_typed::<Handler>().expect("typed PWM callback");
    assert!(PwmCallbackLease::register_typed::<Handler>().is_err());
    drop(first);
    PwmCallbackLease::register_typed::<Handler>().expect("released typed PWM callback");
}
