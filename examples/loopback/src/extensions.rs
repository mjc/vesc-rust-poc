//! POC-specific LispBM extensions for the BLE loopback package.

use core::ffi::CStr;

use vesc_sdk::ffi;
use vesc_sdk::lbm::encode_lbm_i32_raw;

#[cfg(all(test, feature = "test-support"))]
use vesc_sdk::ffi::{LbmApi, LbmBindings, LbmCount, LbmValue};

/// LispBM extension name registered on device (`ext-rust-probe-diag-v4`).
const EXT_RUST_PROBE_DIAG_NAME: &CStr = c"ext-rust-probe-diag-v4";

const PACKAGE_EXTENSION_COUNT: usize = 1;

/// Extension names exported by this loopback example package.
pub const PACKAGE_EXTENSION_NAMES: [&CStr; PACKAGE_EXTENSION_COUNT] = [EXT_RUST_PROBE_DIAG_NAME];

const _: () = assert!(PACKAGE_EXTENSION_COUNT == 1);

#[unsafe(no_mangle)]
/// Device probe: returns encoded LispBM integer 42 without calling firmware `lbm_enc_i`.
///
/// # Safety
///
/// `args` is ignored; callers must satisfy the LispBM extension ABI.
pub unsafe extern "C" fn ext_rust_probe_diag_v4(_args: *mut u32, _argn: u32) -> u32 {
    encode_lbm_i32_raw(42)
}

/// Returns extension descriptors registered by the loopback example package.
pub fn package_extension_descriptors() -> [ffi::ExtensionDescriptor; PACKAGE_EXTENSION_COUNT] {
    [ffi::ExtensionDescriptor::new(
        EXT_RUST_PROBE_DIAG_NAME,
        ext_rust_probe_diag_v4,
    )]
}

/// Returns the diagnostic probe extension descriptor used by tests and fixtures.
pub fn rust_probe_diag_descriptor() -> ffi::ExtensionDescriptor {
    package_extension_descriptors()[0]
}

#[cfg(test)]
pub(crate) fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(all(test, feature = "test-support"))]
fn rust_add_extension_value<B: LbmBindings>(
    api: &LbmApi<B>,
    _args: *mut LbmValue,
    _argn: LbmCount,
) -> LbmValue {
    api.encode_i32(rust_add(20, 22))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{
        EXT_RUST_PROBE_DIAG_NAME, LbmApi, LbmCount, LbmValue, PACKAGE_EXTENSION_NAMES,
        ext_rust_probe_diag_v4, package_extension_descriptors, rust_add_extension_value,
    };
    use vesc_sdk::ffi::test_support::FakeBindings;
    use vesc_sdk::ffi::{self, PackageLifecycle};
    use vesc_sdk::lbm::encode_lbm_i32_raw;
    use vesc_sdk::lifecycle::register_extension_from_image;

    #[test]
    fn probe_returns_the_device_encoded_value() {
        assert_eq!(
            unsafe { ext_rust_probe_diag_v4(core::ptr::null_mut(), 0) },
            encode_lbm_i32_raw(42)
        );
    }

    #[test]
    fn package_extension_table_lists_the_device_probe_descriptor() {
        let [descriptor] = package_extension_descriptors();

        assert_eq!(descriptor.name(), EXT_RUST_PROBE_DIAG_NAME);
        assert_eq!(PACKAGE_EXTENSION_NAMES[0], EXT_RUST_PROBE_DIAG_NAME);
    }

    #[test]
    fn register_package_extension_from_image_uses_the_descriptor_table() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let [descriptor] = package_extension_descriptors();

        assert_eq!(
            unsafe { register_extension_from_image(&info, &lifecycle, descriptor) },
            Ok(())
        );
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    }

    #[test]
    fn package_extension_table_lists_every_rust_owned_extension() {
        assert_eq!(PACKAGE_EXTENSION_NAMES, [EXT_RUST_PROBE_DIAG_NAME]);
        assert!(
            PACKAGE_EXTENSION_NAMES
                .iter()
                .all(|name| name.to_bytes().starts_with(b"ext-"))
        );
    }

    #[test]
    fn rust_add_extension_returns_a_constant_encoded_probe_value() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20), LbmValue(22)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(2)),
            LbmValue(42)
        );
    }

    #[test]
    fn rust_add_extension_does_not_depend_on_live_argument_shape() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(1)),
            LbmValue(42)
        );
        assert_eq!(
            rust_add_extension_value(&api, core::ptr::null_mut(), LbmCount(2)),
            LbmValue(42)
        );
    }
}
