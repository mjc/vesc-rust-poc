//! POC-specific `LispBM` extensions for the BLE loopback package.

use vescpkg_rs::{ExtensionDescriptor, ExtensionName, LbmExtension, LispArgs, LispValue};

/// `LispBM` extension name registered on device (`ext-rust-probe-diag-v4`).
const EXT_RUST_PROBE_DIAG_NAME: ExtensionName =
    vescpkg_rs::extension_name!("ext-rust-probe-diag-v4");

const PACKAGE_EXTENSION_COUNT: usize = 1;

/// Extension names exported by this loopback example package.
pub const PACKAGE_EXTENSION_NAMES: [ExtensionName; PACKAGE_EXTENSION_COUNT] =
    [EXT_RUST_PROBE_DIAG_NAME];

const _: () = assert!(PACKAGE_EXTENSION_COUNT == 1);

struct RustProbeDiag;

impl LbmExtension for RustProbeDiag {
    fn call(_args: LispArgs<'_>) -> LispValue {
        LispValue::try_from(42).expect("42 fits the LispBM immediate integer")
    }
}

/// Returns extension descriptors registered by the loopback example package.
#[must_use]
pub fn package_extension_descriptors() -> [ExtensionDescriptor; PACKAGE_EXTENSION_COUNT] {
    [ExtensionDescriptor::typed::<RustProbeDiag>(
        EXT_RUST_PROBE_DIAG_NAME,
    )]
}

/// Returns the diagnostic probe extension descriptor used by tests and fixtures.
#[must_use]
pub fn rust_probe_diag_descriptor() -> ExtensionDescriptor {
    package_extension_descriptors()[0]
}

#[cfg(test)]
pub(crate) fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(all(test, feature = "test-support"))]
fn rust_add_extension_value() -> LispValue {
    LispValue::try_from(rust_add(20, 22)).expect("42 fits the LispBM immediate integer")
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{
        EXT_RUST_PROBE_DIAG_NAME, LbmExtension, LispArgs, LispValue, PACKAGE_EXTENSION_NAMES,
        RustProbeDiag, package_extension_descriptors, rust_add_extension_value,
    };
    use vescpkg_rs::test_support::{LoaderInfo, TestExtensionRegistry};

    #[test]
    fn package_extension_table_lists_the_device_probe_descriptor() {
        let [descriptor] = package_extension_descriptors();

        assert_eq!(descriptor.name(), EXT_RUST_PROBE_DIAG_NAME);
        assert_eq!(PACKAGE_EXTENSION_NAMES[0], EXT_RUST_PROBE_DIAG_NAME);
    }

    #[test]
    fn package_start_registers_extension_descriptor_table() {
        let registry = TestExtensionRegistry::accepting();
        let mut info = LoaderInfo::new();
        let [descriptor] = package_extension_descriptors();
        let mut start = vescpkg_rs::test_support::package_start(&mut info);
        start.install_stop_hook().unwrap();

        assert!(
            registry
                .register(&mut start, [descriptor])
                .is_ok_and(vescpkg_rs::ExtensionRegistration::is_complete)
        );
        assert_eq!(registry.registration_count(), 1);
    }

    #[test]
    fn package_extension_table_lists_every_rust_owned_extension() {
        assert_eq!(PACKAGE_EXTENSION_NAMES, [EXT_RUST_PROBE_DIAG_NAME]);
        assert!(
            PACKAGE_EXTENSION_NAMES
                .iter()
                .all(|name| name.as_str().starts_with("ext-"))
        );
    }

    #[test]
    fn rust_add_extension_returns_a_constant_encoded_probe_value() {
        assert_eq!(rust_add_extension_value(), LispValue::try_from(42).unwrap());
    }

    #[test]
    fn rust_probe_diag_ignores_lisp_arguments() {
        assert_eq!(
            RustProbeDiag::call(LispArgs::empty()),
            LispValue::try_from(42).unwrap()
        );
    }
}
