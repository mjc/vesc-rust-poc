//! LispBM extensions required by Refloat's package loader.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) defines `ext_set_fw_version` in
//! `third_party/refloat/src/main.c:2305-2313`, `ext_bms` in
//! `third_party/refloat/src/main.c:2315-2331`, and registers both names in
//! `third_party/refloat/src/main.c:2458-2459`. The Lisp loader calls them immediately
//! after native load in `third_party/refloat/lisp/package.lisp:4-17`.

#[cfg(any(test, target_arch = "arm"))]
use crate::bms::ExtBms;
#[cfg(any(test, target_arch = "arm"))]
use crate::package::RefloatPackageState;

#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{ExtensionDescriptor, LispArgs, LispValue};

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefloatLoaderExtension {
    SetFwVersion,
    Bms,
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatLoaderExtension {
    const ALL: [Self; 2] = [Self::SetFwVersion, Self::Bms];

    fn name(self) -> vescpkg_rs::ExtensionName {
        match self {
            Self::SetFwVersion => vescpkg_rs::extension_name!("ext-set-fw-version"),
            Self::Bms => vescpkg_rs::extension_name!("ext-bms"),
        }
    }

    fn descriptor(self) -> ExtensionDescriptor {
        match self {
            Self::SetFwVersion => ExtensionDescriptor::stateful::<ExtSetFwVersion>(self.name()),
            Self::Bms => ExtensionDescriptor::stateful::<ExtBms>(self.name()),
        }
    }
}

/// Called from Refloat's Lisp loader to pass firmware version components.
///
/// Upstream stores these components into `Data` at `third_party/refloat/src/main.c:2305-2311`.
/// The loader-only Rust candidate has no upstream `Data` allocation/`ARG`
/// install from `third_party/refloat/src/main.c:2419-2432`, so it stores only this narrow state.
#[cfg(any(test, target_arch = "arm"))]
struct ExtSetFwVersion;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::StatefulLbmExtension for ExtSetFwVersion {
    type State = RefloatPackageState;

    fn runtime_state() -> &'static vescpkg_rs::PackageStateStore<Self::State> {
        &crate::package::REFLOAT_RUNTIME_STATE
    }

    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
        if args.len() > 2
            && let (Some(major), Some(minor), Some(beta)) =
                (args.integer(0), args.integer(1), args.integer(2))
        {
            record_refloat_firmware_version(state, &[major, minor, beta]);
        }
        args.true_value()
    }
}

/// Firmware version captured from Refloat's loader extension call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatFirmwareVersion {
    major: i32,
    minor: i32,
    beta: i32,
}

impl RefloatFirmwareVersion {
    /// Create a captured firmware-version tuple.
    pub const fn new(major: i32, minor: i32, beta: i32) -> Self {
        Self { major, minor, beta }
    }
}

#[cfg(any(test, target_arch = "arm"))]
fn record_refloat_firmware_version(state: &mut RefloatPackageState, args: &[i32]) {
    // Refloat v1.2.1 only updates version state when `argn > 2` at
    // `third_party/refloat/src/main.c:2306-2310`; shorter calls still
    // return true at `third_party/refloat/src/main.c:2311`.
    let [major, minor, beta] = args else {
        return;
    };
    state.record_firmware_version(RefloatFirmwareVersion::new(*major, *minor, *beta));
}

/// Return the native extension descriptors required by upstream `package.lisp`.
#[cfg(any(test, target_arch = "arm"))]
fn package_extension_descriptors() -> [ExtensionDescriptor; RefloatLoaderExtension::ALL.len()] {
    RefloatLoaderExtension::ALL.map(RefloatLoaderExtension::descriptor)
}

/// Register Refloat's loader extensions with runtime names and handlers.
///
/// Upstream reaches this after custom config and app-data setup in
/// `third_party/refloat/src/main.c:2456-2459`; Rust package init
/// reaches this after state install and runtime thread startup.
///
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_loader_extensions(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::RegisterError> {
    // C map: Refloat registers loader extensions from the loaded package image at
    // `third_party/refloat/src/main.c:2458-2459`; VESC stores that image base in loader
    // metadata before calling init at `third_party/vesc/lispBM/lispif_c_lib.c:1087-1100`.
    start.register_extensions(package_extension_descriptors())
}

#[cfg(test)]
mod tests {
    use super::{
        RefloatFirmwareVersion, RefloatLoaderExtension, package_extension_descriptors,
        record_refloat_firmware_version,
    };
    use crate::package::RefloatPackageState;
    use crate::package::test_support::{lock_refloat_runtime_state, sample_all_data_payloads};
    use vescpkg_rs::LoaderInfo;
    use vescpkg_rs::test_support::TestExtensionRegistry;

    #[test]
    fn extension_table_lists_official_refloat_loader_extensions() {
        let mut descriptors = package_extension_descriptors().into_iter();
        let names = RefloatLoaderExtension::ALL.map(RefloatLoaderExtension::name);

        assert_eq!(
            names,
            [
                super::RefloatLoaderExtension::SetFwVersion.name(),
                super::RefloatLoaderExtension::Bms.name(),
            ]
        );
        assert_eq!(descriptors.len(), names.len());
        assert_eq!(
            descriptors.next().map(|descriptor| descriptor.name()),
            Some(names[0])
        );
        assert_eq!(
            descriptors.next().map(|descriptor| descriptor.name()),
            Some(names[1])
        );
        assert!(descriptors.next().is_none());
    }

    #[test]
    fn package_lifecycle_registers_official_refloat_loader_extensions() {
        let _runtime_state = lock_refloat_runtime_state();
        let registry = TestExtensionRegistry::accepting();
        let mut info = LoaderInfo::new();
        let mut start = vescpkg_rs::test_support::package_start(&mut info);
        let names = RefloatLoaderExtension::ALL.map(RefloatLoaderExtension::name);

        assert_eq!(
            start.install_runtime_state(RefloatPackageState::new(sample_all_data_payloads())),
            Ok(())
        );

        for (descriptor, name) in package_extension_descriptors().into_iter().zip(names) {
            assert_eq!(registry.register(&mut start, [descriptor]), Ok(()));
            assert_eq!(registry.last_registered_name(), Some(name.as_str()));
        }

        assert_eq!(
            registry.registration_count(),
            RefloatLoaderExtension::ALL.len()
        );
        assert!(start.finish_start(true));
        assert!(vescpkg_rs::test_support::stop_package(&mut info));
    }

    #[test]
    fn ext_set_fw_version_records_three_decoded_components() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());

        // Refloat v1.2.1 stores firmware version only when `argn > 2` at
        // `third_party/refloat/src/main.c:2306-2310`; shorter calls still return true at
        // `third_party/refloat/src/main.c:2311`.
        record_refloat_firmware_version(&mut state, &[6, 5]);
        assert_eq!(state.recorded_firmware_version(), None);

        record_refloat_firmware_version(&mut state, &[6, 2, 0]);
        assert_eq!(
            state.recorded_firmware_version(),
            Some(RefloatFirmwareVersion::new(6, 2, 0))
        );
    }
}
