use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::cargo_vescpkg_command::DEFAULT_PACKAGE_VERSION;
use crate::native_lib_link::native_lib_link_plan_for_native_binary;
use crate::native_lib_materialize::materialize_native_lib_binary_unlocked;
use crate::native_lib_toolchain::RealNativeLibToolchain;
use crate::package_conversion::{
    PackageBinaryConversionCommand, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::{PackageProvenance, BLE_LOOPBACK_PACKAGE_NAME};

pub struct RealPackageRunner;

static NATIVE_LIB_BUILD: Mutex<()> = Mutex::new(());

pub fn ensure_native_lib_artifacts(plan: &PackageBinaryConversionPlan) {
    let _guard = NATIVE_LIB_BUILD.lock().expect("native-lib build mutex");
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

#[cfg(test)]
mod tests {
    use super::{ensure_native_lib_artifacts, RealPackageRunner};
    use crate::package_conversion::PackageBinaryConversionPlan;
    use crate::test_support::repo_root;
    use crate::BLE_LOOPBACK_PACKAGE_NAME;
    use std::fs;

    #[test]
    fn native_lib_materialization_matches_package_runner_plan() {
        let root = repo_root();
        let plan = PackageBinaryConversionPlan::new(&root, BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");

        ensure_native_lib_artifacts(&plan);
        let expected_native = fs::read(plan.native_binary_path()).expect("baseline native bin");
        let expected_package = fs::read(plan.package_binary_path()).expect("baseline package bin");

        fs::remove_file(plan.native_binary_path()).expect("remove native bin for rebuild");
        fs::remove_file(plan.package_binary_path()).expect("remove package bin for rebuild");
        fs::remove_file(root.join("target/native-lib-baseline/native_lib.elf"))
            .expect("remove native elf for rebuild");

        plan.run_with(&RealPackageRunner)
            .expect("rebuild native-lib through conversion plan");

        assert_eq!(
            fs::read(plan.native_binary_path()).expect("rebuilt native bin"),
            expected_native,
            "conversion plan should reproduce native payload bytes"
        );
        assert_eq!(
            fs::read(plan.package_binary_path()).expect("rebuilt package bin"),
            expected_package,
            "conversion plan should reproduce package payload bytes"
        );
    }
}
