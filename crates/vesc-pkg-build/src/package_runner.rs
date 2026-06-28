use std::fs;
use std::path::Path;

use crate::cargo_vescpkg_command::DEFAULT_PACKAGE_VERSION;
use crate::native_lib_link::native_lib_link_plan_for_native_binary;
use crate::native_lib_materialize::materialize_native_lib_binary_unlocked;
use crate::native_lib_toolchain::RealNativeLibToolchain;
use crate::package_conversion::{
    PackageBinaryConversionCommand, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::{PackageProvenance, BLE_LOOPBACK_PACKAGE_NAME};

pub struct RealPackageRunner;

pub fn ensure_native_lib_artifacts(plan: &PackageBinaryConversionPlan) {
    RealPackageRunner
        .run(&plan.command())
        .unwrap_or_else(|error| {
            panic!(
                "native-lib package conversion failed: {} -> {}: {error}",
                plan.native_binary_path().display(),
                plan.package_binary_path().display()
            )
        });
}

pub fn ensure_repo_native_lib_artifacts(root: &Path) {
    let plan =
        PackageBinaryConversionPlan::new(root, BLE_LOOPBACK_PACKAGE_NAME, DEFAULT_PACKAGE_VERSION);
    ensure_native_lib_artifacts(&plan);
}

impl PackageBinaryConversionRunner for RealPackageRunner {
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
        let plan = native_lib_link_plan_for_native_binary(command.native_binary_path());
        materialize_native_lib_binary_unlocked(
            &plan,
            command.native_binary_path(),
            &RealNativeLibToolchain,
        )?;

        if let Some(parent) = command.package_binary_path().parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        fs::copy(command.native_binary_path(), command.package_binary_path())
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

pub fn package_provenance_from_env() -> PackageProvenance {
    PackageProvenance::new(
        std::env::var("VESC_PKG_GIT_COMMIT").ok(),
        std::env::var("VESC_PKG_BUILD_DATE").ok(),
    )
}
