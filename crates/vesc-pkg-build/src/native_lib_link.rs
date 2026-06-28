use std::path::{Path, PathBuf};

pub const RUST_STATICLIB_PATH: &str = "target/thumbv7em-none-eabihf/release/libvesc_ble_loopback.a";
pub const NATIVE_LIB_ELF_PATH: &str = "target/native-lib-baseline/native_lib.elf";
pub const NATIVE_LIB_BIN_PATH: &str = "target/native-lib-baseline/native_lib.bin";
pub const NATIVE_LIB_LINKER_SCRIPT: &str = "fixtures/native-lib-baseline/src/link.ld";
pub const PACKAGE_C_SOURCE_PATH: &str = "fixtures/native-lib-baseline/src/package_lib.c";
pub const PACKAGE_C_OBJECT_PATH: &str = "target/native-lib-baseline/package_lib.o";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibLinkPlan {
    root: PathBuf,
}

impl NativeLibLinkPlan {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn cargo_target_dir(&self) -> PathBuf {
        self.root.join("target")
    }

    pub fn rust_staticlib_path(&self) -> PathBuf {
        self.root.join(RUST_STATICLIB_PATH)
    }

    pub fn elf_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_ELF_PATH)
    }

    pub fn native_lib_bin_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_BIN_PATH)
    }

    pub fn linker_script_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_LINKER_SCRIPT)
    }

    pub fn package_c_source_path(&self) -> PathBuf {
        self.root.join(PACKAGE_C_SOURCE_PATH)
    }

    pub fn package_c_object_path(&self) -> PathBuf {
        self.root.join(PACKAGE_C_OBJECT_PATH)
    }

    pub fn link_inputs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [
            self.rust_staticlib_path(),
            self.package_c_source_path(),
            self.linker_script_path(),
        ]
        .into_iter()
    }
}

pub fn native_lib_link_plan() -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(crate::hygiene::repo_root())
}

pub fn native_lib_link_plan_for_native_binary(native_binary_path: &Path) -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(artifact_root_from_native_binary(native_binary_path))
}

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
