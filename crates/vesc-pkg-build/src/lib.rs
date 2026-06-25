pub mod abi_inventory;
pub mod cargo_vescpkg_command;
pub mod hygiene;
pub mod native_lib_baseline;
pub mod native_lib_link;
pub mod package_artifacts;
pub mod package_assets;
pub mod package_build;
pub mod package_conversion;
pub mod package_format;
pub mod package_runtime;
pub mod package_target;
pub mod rust_package_api_roadmap;
pub mod symbol_audit;

pub const BLE_LOOPBACK_PACKAGE_NAME: &str = "Rust BLE loopback test package";

pub use abi_inventory::{minimal_test_package_abi, AbiRequirement, AbiRequirementKind};
pub use native_lib_baseline::{
    baseline_input_paths, baseline_output_paths, native_lib_baseline_root, NativeLibBaselinePath,
};
pub use native_lib_link::{native_lib_link_plan, NativeLibLinkPlan};
pub use package_artifacts::{
    PackageArtifactInspectionError, PackageArtifactInspectionPlan, PackageArtifactProblem,
    NATIVE_PAYLOAD_PATH,
};
pub use package_assets::{PackageAssets, PackageProvenance};
pub use package_build::PackageBuildPlan;
pub use package_conversion::{
    PackageBinaryConversionCommand, PackageBinaryConversionError, PackageBinaryConversionPlan,
    PackageBinaryConversionRunner, CONVERSION_SCRIPT_PATH, NATIVE_LIB_BINARY_PATH,
    PACKAGE_LIB_BINARY_PATH,
};
pub use package_runtime::{
    FakeFirmwareServices, FirmwareServices, LoopbackPackageRuntime, LoopbackPackageState,
    LoopbackRuntimeError, LoopbackStartError, LoopbackTick,
};
pub use package_target::{PackageTargetError, PackageTargetMode, PackageTargetPlan};
pub use symbol_audit::{
    audit_rust_staticlib_symbols, defined_symbols, is_allowed_runtime_symbol, nm_output,
    undefined_symbols, unexpected_undefined_symbols,
};

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLayout {
    package_name: String,
    version: String,
}

impl PackageLayout {
    pub fn new(package_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            version: version.into(),
        }
    }

    pub fn artifact_name(&self) -> String {
        format!(
            "{}-{}.vescpkg",
            sanitize(&self.package_name),
            sanitize(&self.version)
        )
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn staging_dir(&self) -> PathBuf {
        Path::new("target").join("vescpkg").join(format!(
            "{}-{}",
            sanitize(&self.package_name),
            sanitize(&self.version)
        ))
    }

    pub fn descriptor_path(&self) -> PathBuf {
        self.staging_dir().join("pkgdesc.qml")
    }
}

fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PackageLayout, BLE_LOOPBACK_PACKAGE_NAME};

    #[test]
    fn renders_stable_artifact_paths() {
        let layout = PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, "0.1.0");

        assert_eq!(
            layout.artifact_name(),
            "Rust-BLE-loopback-test-package-0.1.0.vescpkg"
        );
        assert_eq!(
            layout.staging_dir(),
            std::path::PathBuf::from("target/vescpkg/Rust-BLE-loopback-test-package-0.1.0")
        );
        assert_eq!(
            layout.descriptor_path(),
            std::path::PathBuf::from(
                "target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/pkgdesc.qml"
            )
        );
    }
}
