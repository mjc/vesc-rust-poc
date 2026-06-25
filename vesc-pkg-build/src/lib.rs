pub mod abi_inventory;
pub mod hygiene;
pub mod native_lib_baseline;

pub use abi_inventory::{minimal_test_package_abi, AbiRequirement, AbiRequirementKind};
pub use native_lib_baseline::{
    baseline_input_paths, baseline_output_paths, native_lib_baseline_root, NativeLibBaselinePath,
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

    pub fn staging_dir(&self) -> PathBuf {
        Path::new("target")
            .join("vescpkg")
            .join(format!("{}-{}", sanitize(&self.package_name), sanitize(&self.version)))
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
    use super::PackageLayout;

    #[test]
    fn renders_stable_artifact_paths() {
        let layout = PackageLayout::new("Rust VESC package", "0.1.0");

        assert_eq!(layout.artifact_name(), "Rust-VESC-package-0.1.0.vescpkg");
        assert_eq!(
            layout.staging_dir(),
            std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0")
        );
        assert_eq!(
            layout.descriptor_path(),
            std::path::PathBuf::from("target/vescpkg/Rust-VESC-package-0.1.0/pkgdesc.qml")
        );
    }
}
