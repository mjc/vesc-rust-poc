//! `LispBM` extensions required by Float Out Boy's package loader.
//!
//! Float Out Boy `v1.2.1` (`0ef6e99d8701`) defines `ext_set_fw_version` in
//! `third_party/float-out-boy/src/main.c:2305-2313`, `ext_bms` in
//! `third_party/float-out-boy/src/main.c:2315-2331`, and registers both names in
//! `third_party/float-out-boy/src/main.c:2458-2459`. The Lisp loader calls them immediately
//! after native load in `third_party/float-out-boy/lisp/package.lisp:4-17`.

use crate::bms::ExtBms;
use crate::package::FloatOutBoyPackageState;

use vescpkg_rs::{ExtensionDescriptor, FirmwareVersion, LispArgs, LispValue};

/// Called from Float Out Boy's Lisp loader to pass firmware version components.
///
/// Upstream stores these components into `Data` at `third_party/float-out-boy/src/main.c:2305-2311`.
/// The loader-only Rust candidate has no upstream `Data` allocation/`ARG`
/// install from `third_party/float-out-boy/src/main.c:2419-2432`, so it stores only this narrow state.
struct ExtSetFwVersion;

impl vescpkg_rs::StatefulLbmExtension for ExtSetFwVersion {
    type State = FloatOutBoyPackageState;

    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
        if args.len() > 2 {
            let mut values = args.iter();
            if let (Some(major), Some(minor), Some(beta)) = (
                values.next().and_then(LispValue::decode_number_as_i32),
                values.next().and_then(LispValue::decode_number_as_i32),
                values.next().and_then(LispValue::decode_number_as_i32),
            ) {
                record_float_out_boy_firmware_version(state, &[major, minor, beta]);
            }
        }
        LispValue::true_value()
    }
}

fn record_float_out_boy_firmware_version(
    state: &mut FloatOutBoyPackageState,
    args: &[i32],
) -> bool {
    // Float Out Boy v1.2.1 only updates version state when `argn > 2` at
    // `third_party/float-out-boy/src/main.c:2306-2310`; shorter calls still
    // return true at `third_party/float-out-boy/src/main.c:2311`.
    args.get(..3)
        .and_then(|values| <&[i32; 3]>::try_from(values).ok())
        .is_some_and(|&[major, minor, beta]| {
            state.record_firmware_version(FirmwareVersion::new(major, minor, beta));
            true
        })
}

/// Return the native extension descriptors required by upstream `package.lisp`.
fn package_extension_descriptors() -> [ExtensionDescriptor; 2] {
    [
        ExtensionDescriptor::stateful::<ExtSetFwVersion>(vescpkg_rs::extension_name!(
            "ext-set-fw-version"
        )),
        ExtensionDescriptor::stateful::<ExtBms>(vescpkg_rs::extension_name!("ext-bms")),
    ]
}

/// Register Float Out Boy's loader extensions with runtime names and handlers.
///
/// Upstream reaches this after custom config and app-data setup in
/// `third_party/float-out-boy/src/main.c:2456-2459`; Rust package init
/// reaches this after state install and runtime thread startup.
///
/// # Errors
///
/// Returns [`vescpkg_rs::PackageStartError`] if the firmware cannot register
/// the package's extension descriptors.
///
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_float_out_boy_loader_extensions(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    // C map: Float Out Boy registers loader extensions from the loaded package image at
    // `third_party/float-out-boy/src/main.c:2458-2459`; VESC stores that image base in loader
    // metadata before calling init at `third_party/vesc/lispBM/lispif_c_lib.c:1087-1100`.
    start
        .register_extensions(package_extension_descriptors())
        .map(|_| ())
}

#[cfg(test)]
mod tests;
