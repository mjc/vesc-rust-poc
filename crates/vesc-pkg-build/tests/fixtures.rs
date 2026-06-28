use vesc_pkg_build::native_audit::audit_device_proven_fixture;
use vesc_pkg_build::native_lib_baseline::{
    audit_baseline_fixture_layout, audit_vesc_c_if_abi_pins,
};

#[test]
fn fixtures() {
    audit_baseline_fixture_layout();
    audit_vesc_c_if_abi_pins();
    audit_device_proven_fixture();
}
