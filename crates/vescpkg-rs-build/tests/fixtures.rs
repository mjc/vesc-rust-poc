//! Golden fixture smoke tests for package payload metadata.

use vescpkg_rs_build::{
    audit_baseline_fixture_layout, audit_device_proven_fixture, audit_vesc_c_if_abi_pins,
};

#[test]
fn fixtures() {
    audit_baseline_fixture_layout();
    audit_vesc_c_if_abi_pins();
    audit_device_proven_fixture();
}
