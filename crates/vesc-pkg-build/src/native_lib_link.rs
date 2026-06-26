use std::path::PathBuf;

pub const RUST_STATICLIB_PATH: &str = "target/thumbv7em-none-eabihf/release/libvesc_rust_poc.a";
pub const NATIVE_LIB_ELF_PATH: &str = "target/native-lib-baseline/native_lib.elf";
pub const NATIVE_LIB_LINKER_SCRIPT: &str = "fixtures/native-lib-baseline/src/link.ld";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibLinkPlan {
    root: PathBuf,
}

impl NativeLibLinkPlan {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn rust_staticlib_path(&self) -> PathBuf {
        self.root.join(RUST_STATICLIB_PATH)
    }

    pub fn elf_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_ELF_PATH)
    }

    pub fn linker_script_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_LINKER_SCRIPT)
    }

    pub fn link_inputs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [self.rust_staticlib_path(), self.linker_script_path()].into_iter()
    }
}

pub fn native_lib_link_plan() -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn native_lib_elf_path() -> PathBuf {
    native_lib_link_plan().elf_path()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        native_lib_link_plan, NativeLibLinkPlan, NATIVE_LIB_ELF_PATH, NATIVE_LIB_LINKER_SCRIPT,
        RUST_STATICLIB_PATH,
    };

    #[test]
    fn links_the_rust_staticlib_into_the_native_lib_flow() {
        let plan = native_lib_link_plan();

        assert_eq!(
            plan.link_inputs().collect::<Vec<_>>(),
            vec![plan.rust_staticlib_path(), plan.linker_script_path()]
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
    }
}
