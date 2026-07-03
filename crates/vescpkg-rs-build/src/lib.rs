//! Host-side VESC package format and build support.
//!
//! This crate reads/writes `.vescpkg` files and provides package build
//! primitives for tools and CLIs. It does not run inside the VESC firmware.

/// ABI inventory used by package layout and audit checks.
pub mod abi_inventory;
/// Cargo subcommand parsing and command-file helpers.
pub mod cargo_vescpkg_command;
/// Helpers for comparing the Rust ABI surface against the C header.
pub mod ffi_compare;
/// Golden fixture data for the loopback package and native-lib checks.
#[allow(dead_code, unused_imports)]
pub(crate) mod golden;
/// Repository hygiene helpers and generated-path checks.
#[allow(dead_code, unused_imports)]
pub(crate) mod hygiene;
/// Package manifest discovery and parsing helpers.
pub mod manifest;
/// Native artifact audit helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_audit;
/// Native build orchestration helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_build;
/// Native binary comparison helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_compare;
/// Native ELF disassembly helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_disasm;
/// Native ELF semantic analysis helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_elf_semantics;
/// Native artifact inspection helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_inspect;
/// Native-library audit helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_lib_audit;
/// Native-library baseline fixture helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_lib_baseline;
/// Native-library link-plan helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_lib_link;
/// Native-library materialization helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_lib_materialize;
/// Native-library toolchain helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod native_lib_toolchain;
/// Package model and builder types.
pub mod package;
/// Package artifact inspection helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod package_artifacts;
/// Package provenance and asset metadata.
pub mod package_assets;
/// Package build-plan orchestration.
#[allow(dead_code, unused_imports)]
pub(crate) mod package_build;
/// Package binary conversion helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod package_conversion;
/// Package encoding and decoding helpers.
pub mod package_format;
/// Package wire-format decoding helpers.
pub mod package_format_decode;
/// Golden package generation and comparison helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod package_golden;
/// Native package runner helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod package_runner;
/// Loopback package runtime helpers.
pub mod package_runtime;
/// Package target and staging helpers.
pub mod package_target;
/// Host-side package wire format helpers.
pub mod package_wire;
/// Refloat source-tree native payload build helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod refloat_native_build;
/// Refloat source-tree package asset generation helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod refloat_package_assets;
/// Roadmap notes for the Rust package API.
#[allow(dead_code, unused_imports)]
pub(crate) mod rust_package_api_roadmap;
/// Symbol audit helpers.
#[allow(dead_code, unused_imports)]
pub(crate) mod symbol_audit;
/// Test-support harnesses and fake runners for package tooling tests.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Canonical name used by the loopback package fixtures.
pub const BLE_LOOPBACK_PACKAGE_NAME: &str = "Rust BLE loopback test package";
/// Canonical name used by the Snake example package.
pub const SNAKE_PACKAGE_NAME: &str = "Rust Snake example package";
/// Canonical name used by the Refloat package.
pub const REFLOAT_PACKAGE_NAME: &str = "Refloat";
/// Canonical Refloat package version used for the v1.2.1 parity target.
pub const REFLOAT_PACKAGE_VERSION: &str = "1.2.1";

pub use abi_inventory::{AbiRequirement, AbiRequirementKind, minimal_test_package_abi};
pub use golden::{
    FINGERPRINTS_TOML, LISP_DATA, NATIVE_LIB_BIN, NATIVE_LIB_ELF, PACKAGE_LIB, VERSION,
    fixture_dir, pack_lisp_data, payload_contains_probe_extension, probe_extension_name,
};
pub use manifest::{manifest_path, parse_pkgdesc, staging_dir_from_manifest};
pub use native_audit::audit_device_proven_fixture;
pub use native_compare::native_binary_comparison_report;
pub use native_elf_semantics::assert_native_lib_semantics;
pub use native_lib_audit::{
    NativeLibArtifactPaths, audit_native_lib_artifacts, audit_native_lib_flat_binary,
    audit_native_lib_layout, audit_native_lib_symbols, audit_refloat_native_lib_artifacts,
    semantic_snapshot_report,
};
pub use native_lib_baseline::{
    NativeLibBaselinePath, audit_baseline_fixture_layout, audit_vesc_c_if_abi_pins,
    baseline_input_paths, baseline_output_paths, fingerprint_bytes, native_lib_baseline_root,
};
pub use native_lib_link::{NativeLibLinkPlan, native_lib_link_plan};
pub use package::{Builder, Package, PackageError};
pub use package_artifacts::{
    NATIVE_PAYLOAD_PATH, PackageArtifactInspectionError, PackageArtifactInspectionPlan,
    PackageArtifactProblem,
};
pub use package_assets::{PackageAssets, PackageProvenance};
pub use package_build::{PackageBuildPlan, PackageExample};
pub use package_conversion::{
    CONVERSION_SCRIPT_PATH, NATIVE_LIB_BINARY_PATH, PACKAGE_LIB_BINARY_PATH,
    PackageBinaryConversionCommand, PackageBinaryConversionError, PackageBinaryConversionPlan,
    PackageBinaryConversionRunner,
};
pub use package_golden::{
    GOLDEN_FINGERPRINTS_TOML, GOLDEN_FIXTURE_DIR, GOLDEN_FIXTURE_VERSION, GOLDEN_LISP_DATA_BIN,
    GOLDEN_PACKAGE_LIB_BIN, build_and_copy_package_lib_bin, golden_fixture_root,
    package_lib_output_path, repo_root,
};
pub use package_runner::{
    RealPackageRunner, ensure_native_lib_artifacts, ensure_repo_native_lib_artifacts,
    package_provenance_from_env,
};
#[cfg(any(test, feature = "test-support"))]
pub use package_runtime::FakeFirmwareServices;
pub use package_runtime::{
    FirmwareServices, LoopbackPackageRuntime, LoopbackPackageState, LoopbackRuntimeError,
    LoopbackStartError, LoopbackTick,
};
pub use package_target::{PackageTargetError, PackageTargetMode, PackageTargetPlan};
pub use package_wire::{
    LispImport, PackageField, VESC_PACKET_HEADER, WireError, decompress_vescpkg, field_bytes,
    parse_decompressed_vescpkg, parse_lisp_imports, parse_vescpkg, wire_comparison_report,
    wire_snapshot_report,
};
pub use symbol_audit::{
    audit_rust_staticlib_symbols, defined_symbols, is_allowed_runtime_symbol, nm_output,
    undefined_symbols, unexpected_undefined_symbols,
};

use std::path::{Path, PathBuf};

/// Filesystem layout for a single package version and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLayout {
    package_name: String,
    version: String,
}

impl PackageLayout {
    /// Create a layout from a package name and version.
    pub fn new(package_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            version: version.into(),
        }
    }

    /// Return the `.vescpkg` artifact filename.
    pub fn artifact_name(&self) -> String {
        format!(
            "{}-{}.vescpkg",
            sanitize(&self.package_name),
            sanitize(&self.version)
        )
    }

    /// Return the package name used by the layout.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Return the package version used by the layout.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Return the staging directory for the package.
    pub fn staging_dir(&self) -> PathBuf {
        Path::new("target").join("vescpkg").join(format!(
            "{}-{}",
            sanitize(&self.package_name),
            sanitize(&self.version)
        ))
    }

    /// Return the staged `pkgdesc.qml` path.
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
    use super::{BLE_LOOPBACK_PACKAGE_NAME, PackageLayout};

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
