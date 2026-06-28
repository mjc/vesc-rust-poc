use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::cargo_vescpkg_command::DEFAULT_PACKAGE_VERSION;
use crate::native_lib_link::native_lib_link_plan_for_native_binary;
use crate::native_lib_materialize::materialize_native_lib_binary_unlocked;
use crate::native_lib_toolchain::RealNativeLibToolchain;
use crate::package_conversion::{
    PackageBinaryConversionCommand, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageProvenance};

pub struct RealPackageRunner;

static NATIVE_LIB_BUILD: Mutex<()> = Mutex::new(());
static REPO_NATIVE_LIB: OnceLock<()> = OnceLock::new();

pub fn ensure_native_lib_artifacts(plan: &PackageBinaryConversionPlan) {
    let _guard = NATIVE_LIB_BUILD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    REPO_NATIVE_LIB.get_or_init(|| {
        let plan = PackageBinaryConversionPlan::new(
            root,
            BLE_LOOPBACK_PACKAGE_NAME,
            DEFAULT_PACKAGE_VERSION,
        );
        ensure_native_lib_artifacts(&plan);
    });
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

#[cfg(test)]
mod tests {
    use super::ensure_native_lib_artifacts;
    use crate::BLE_LOOPBACK_PACKAGE_NAME;
    use crate::hygiene::repo_root;
    use crate::package_conversion::PackageBinaryConversionPlan;
    use std::fs;

    #[test]
    fn native_lib_materialization_is_idempotent() {
        let root = repo_root();
        let plan = PackageBinaryConversionPlan::new(&root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");

        ensure_native_lib_artifacts(&plan);
        let expected_native = fs::read(plan.native_binary_path()).expect("baseline native bin");
        let expected_package = fs::read(plan.package_binary_path()).expect("baseline package bin");

        ensure_native_lib_artifacts(&plan);

        assert_eq!(
            fs::read(plan.native_binary_path()).expect("second native bin"),
            expected_native,
            "conversion plan should keep native payload bytes stable"
        );
        assert_eq!(
            fs::read(plan.package_binary_path()).expect("second package bin"),
            expected_package,
            "conversion plan should keep package payload bytes stable"
        );
    }
}
