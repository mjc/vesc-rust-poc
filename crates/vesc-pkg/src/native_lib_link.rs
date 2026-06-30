use std::path::{Path, PathBuf};

/// Repository-relative Rust static library path used by the native-lib link.
pub const RUST_STATICLIB_PATH: &str =
    "target/thumbv7em-none-eabihf/release/libvesc_example_loopback.a";
/// Repository-relative linked native ELF output path.
pub const NATIVE_LIB_ELF_PATH: &str = "target/native-lib-baseline/native_lib.elf";
/// Repository-relative flattened native binary output path.
pub const NATIVE_LIB_BIN_PATH: &str = "target/native-lib-baseline/native_lib.bin";
/// Repository-relative linker script used for native-lib packaging.
pub const NATIVE_LIB_LINKER_SCRIPT: &str = "fixtures/native-lib-baseline/src/link.ld";
/// Repository-relative C shim source path linked with the Rust static library.
pub const PACKAGE_C_SOURCE_PATH: &str = "fixtures/native-lib-baseline/src/package_lib.c";
/// Repository-relative C shim object path used by the final native-lib link.
pub const PACKAGE_C_OBJECT_PATH: &str = "target/native-lib-baseline/package_lib.o";

/// Native-library link plan rooted at a concrete artifact directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibLinkPlan {
    /// Artifact root that owns all linked native-lib outputs.
    pub root: PathBuf,
}

impl NativeLibLinkPlan {
    /// Creates a link plan rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the artifact root for this plan.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the Cargo target directory used for Rust staticlib outputs.
    pub fn cargo_target_dir(&self) -> PathBuf {
        self.root.join("target")
    }

    /// Returns the Rust static library input path.
    pub fn rust_staticlib_path(&self) -> PathBuf {
        self.root.join(RUST_STATICLIB_PATH)
    }

    /// Returns the linked native ELF output path.
    pub fn elf_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_ELF_PATH)
    }

    /// Returns the flattened native binary output path.
    pub fn native_lib_bin_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_BIN_PATH)
    }

    /// Returns the linker script path used for the native-lib link.
    pub fn linker_script_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_LINKER_SCRIPT)
    }

    /// Returns the C package shim source path.
    pub fn package_c_source_path(&self) -> PathBuf {
        self.root.join(PACKAGE_C_SOURCE_PATH)
    }

    /// Returns the C package shim object path.
    pub fn package_c_object_path(&self) -> PathBuf {
        self.root.join(PACKAGE_C_OBJECT_PATH)
    }

    /// Iterates over files that must exist before the final native link.
    pub fn link_inputs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            self.rust_staticlib_path(),
            self.package_c_source_path(),
            self.linker_script_path(),
        ]
        .into_iter()
    }
}

/// Returns the repository-default native-lib link plan.
pub fn native_lib_link_plan() -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(crate::hygiene::repo_root())
}

/// Returns a link plan rooted from a requested native binary output path.
pub fn native_lib_link_plan_for_native_binary(native_binary_path: &Path) -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(artifact_root_from_native_binary(native_binary_path))
}

/// Derives an artifact root from a native binary output path.
pub fn artifact_root_from_native_binary(native_binary_path: &Path) -> PathBuf {
    native_binary_path
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect(
            "native binary path must live under <root>/target/native-lib-baseline/native_lib.bin",
        )
        .to_path_buf()
}

/// Returns the repository-default native ELF output path.
pub fn native_lib_elf_path() -> PathBuf {
    native_lib_link_plan().elf_path()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        NATIVE_LIB_ELF_PATH, NATIVE_LIB_LINKER_SCRIPT, NativeLibLinkPlan, PACKAGE_C_OBJECT_PATH,
        PACKAGE_C_SOURCE_PATH, RUST_STATICLIB_PATH, native_lib_link_plan,
    };

    #[test]
    fn links_the_rust_staticlib_into_the_native_lib_flow() {
        let plan = native_lib_link_plan();

        assert_eq!(
            plan.link_inputs().collect::<Vec<_>>(),
            vec![
                plan.rust_staticlib_path(),
                plan.package_c_source_path(),
                plan.linker_script_path()
            ]
        );
        assert_eq!(
            plan.elf_path(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(NATIVE_LIB_ELF_PATH)
        );
    }

    #[test]
    fn uses_the_expected_baseline_paths() {
        let plan = NativeLibLinkPlan::new("fixtures/native-lib-baseline");

        assert_eq!(
            plan.rust_staticlib_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(RUST_STATICLIB_PATH)
        );
        assert_eq!(
            plan.linker_script_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(NATIVE_LIB_LINKER_SCRIPT)
        );
        assert_eq!(
            plan.package_c_source_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(PACKAGE_C_SOURCE_PATH)
        );
        assert_eq!(
            plan.package_c_object_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(PACKAGE_C_OBJECT_PATH)
        );
    }
}
