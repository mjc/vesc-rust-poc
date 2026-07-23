#![cfg(feature = "test-support")]
//! Integration coverage for typed PWM callback ownership.

use vescpkg_rs::{
    PackageRuntimeState, PackageStateStore, PwmCallbackHandler, PwmCallbackLease,
    TypedPwmCallbackLease,
};

struct Handler;

impl PwmCallbackHandler for Handler {
    fn on_pwm() {}
}

struct PackageState {
    _lease: Option<TypedPwmCallbackLease<Handler>>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

#[test]
fn typed_pwm_registration_is_exclusive_and_released_on_drop() {
    let _firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let first = PwmCallbackLease::register_typed::<Handler>().expect("typed PWM callback");
    assert!(PwmCallbackLease::register_typed::<Handler>().is_err());
    drop(first);
    PwmCallbackLease::register_typed::<Handler>().expect("released typed PWM callback");
}

#[test]
fn package_stop_releases_typed_pwm_state_before_next_registration() {
    let _firmware = vescpkg_rs::test_support::FirmwareTest::new();
    let lease = PwmCallbackLease::register_typed::<Handler>().expect("typed PWM callback");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _lease: Some(lease),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));
    PwmCallbackLease::register_typed::<Handler>().expect("stop released typed PWM callback");
}
