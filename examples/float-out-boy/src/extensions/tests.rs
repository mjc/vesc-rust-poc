use super::{
    FirmwareVersion, FloatOutBoyLoaderExtension, package_extension_descriptors,
    record_float_out_boy_firmware_version,
};
use crate::package::FloatOutBoyPackageState;
use crate::package::test_support::{lock_float_out_boy_runtime_state, sample_all_data_payloads};
use vescpkg_rs::test_support::{LoaderInfo, TestExtensionRegistry};

#[test]
fn extension_table_lists_official_float_out_boy_loader_extensions() {
    let mut descriptors = package_extension_descriptors().into_iter();
    let names = FloatOutBoyLoaderExtension::ALL.map(FloatOutBoyLoaderExtension::name);

    assert_eq!(
        names,
        [
            super::FloatOutBoyLoaderExtension::SetFwVersion.name(),
            super::FloatOutBoyLoaderExtension::Bms.name(),
        ]
    );
    assert_eq!(descriptors.len(), names.len());
    assert_eq!(
        descriptors
            .next()
            .map(vescpkg_rs::ExtensionDescriptor::name),
        Some(names[0])
    );
    assert_eq!(
        descriptors
            .next()
            .map(vescpkg_rs::ExtensionDescriptor::name),
        Some(names[1])
    );
    assert!(descriptors.next().is_none());
}

#[test]
fn package_lifecycle_registers_official_float_out_boy_loader_extensions() {
    let _runtime_state = lock_float_out_boy_runtime_state();
    let registry = TestExtensionRegistry::accepting();
    let mut info = LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    let names = FloatOutBoyLoaderExtension::ALL.map(FloatOutBoyLoaderExtension::name);

    assert_eq!(
        start.install_runtime_state(FloatOutBoyPackageState::new(sample_all_data_payloads())),
        Ok(())
    );

    for (descriptor, name) in package_extension_descriptors().into_iter().zip(names) {
        assert_eq!(
            registry
                .register(&mut start, [descriptor])
                .map(|registration| (registration.registered(), registration.is_complete())),
            Ok((1, true))
        );
        assert_eq!(registry.last_registered_name(), Some(name.as_str()));
    }

    assert_eq!(
        registry.registration_count(),
        FloatOutBoyLoaderExtension::ALL.len()
    );
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));
}

#[test]
fn ext_set_fw_version_records_three_decoded_components() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    // Float Out Boy v1.2.1 stores firmware version only when `argn > 2` at
    // `third_party/float-out-boy/src/main.c:2306-2310`; shorter calls still return true at
    // `third_party/float-out-boy/src/main.c:2311`.
    record_float_out_boy_firmware_version(&mut state, &[6, 5]);
    assert_eq!(state.recorded_firmware_version(), None);

    record_float_out_boy_firmware_version(&mut state, &[6, 2, 0]);
    assert_eq!(
        state.recorded_firmware_version(),
        Some(FirmwareVersion::new(6, 2, 0))
    );
}
