use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::cargo_vescpkg_command::DEFAULT_PACKAGE_VERSION;
use crate::native_lib_link::NativeLibLinkPlan;
use crate::native_lib_materialize::materialize_native_lib_binary_unlocked;
use crate::native_lib_toolchain::RealNativeLibToolchain;
use crate::package_conversion::{
    PackageBinaryConversionCommand, PackageBinaryConversionPlan, PackageBinaryConversionRunner,
};
use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageProvenance};

/// Runner that materializes the native package binary using the real toolchain.
pub struct RealPackageRunner;

static NATIVE_LIB_BUILD: Mutex<()> = Mutex::new(());
static REPO_NATIVE_LIB: OnceLock<()> = OnceLock::new();

/// Ensure that the native library artifacts exist for one package plan.
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

/// Ensure the repository's loopback native artifacts exist.
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
    /// Run the native build and copy the resulting package binary into place.
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
        let source_root = source_root_from_conversion_script(command.script_path())?;
        let native_build_dir = command.native_binary_path().parent().ok_or_else(|| {
            format!(
                "native binary path must have a parent directory: {}",
                command.native_binary_path().display()
            )
        })?;
        let plan = NativeLibLinkPlan::for_example_with_native_build_dir(
            source_root,
            command.example(),
            native_build_dir,
        );
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

fn source_root_from_conversion_script(script_path: &Path) -> Result<&Path, String> {
    let scripts_dir = script_path.parent().ok_or_else(|| {
        format!(
            "conversion script path must live under <source-root>/scripts: {}",
            script_path.display()
        )
    })?;
    if scripts_dir.file_name().and_then(|name| name.to_str()) != Some("scripts") {
        return Err(format!(
            "conversion script path must live under <source-root>/scripts: {}",
            script_path.display()
        ));
    }
    scripts_dir.parent().ok_or_else(|| {
        format!(
            "conversion script path must live under <source-root>/scripts: {}",
            script_path.display()
        )
    })
}

/// Read package provenance metadata from the process environment.
pub fn package_provenance_from_env() -> PackageProvenance {
    PackageProvenance::new(
        std::env::var("VESC_PKG_GIT_COMMIT").ok(),
        std::env::var("VESC_PKG_BUILD_DATE").ok(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::source_root_from_conversion_script;

    #[test]
    fn conversion_script_source_root_requires_scripts_directory() {
        let root = source_root_from_conversion_script(Path::new("/repo/scripts/conv.py"))
            .expect("source root");
        assert_eq!(root, Path::new("/repo"));

        let error = source_root_from_conversion_script(Path::new("/repo/foo/conv.py"))
            .expect_err("bad script directory");
        assert!(error.contains("<source-root>/scripts"));
    }
}
