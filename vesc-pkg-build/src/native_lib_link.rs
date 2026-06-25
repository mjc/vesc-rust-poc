use std::path::PathBuf;

pub const NATIVE_LIB_SHIM_OBJECT: &str = "target/native-lib-baseline/package_lib.o";
pub const RUST_STATICLIB_PATH: &str = "target/thumbv7em-none-eabihf/release/libvesc_rust_poc.a";
pub const NATIVE_LIB_ELF_PATH: &str = "target/native-lib-baseline/native_lib.elf";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibLinkPlan {
    root: PathBuf,
}

impl NativeLibLinkPlan {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn shim_object_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_SHIM_OBJECT)
    }

    pub fn rust_staticlib_path(&self) -> PathBuf {
        self.root.join(RUST_STATICLIB_PATH)
    }

    pub fn elf_path(&self) -> PathBuf {
        self.root.join(NATIVE_LIB_ELF_PATH)
    }

    pub fn link_inputs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [self.shim_object_path(), self.rust_staticlib_path()].into_iter()
    }
}

pub fn native_lib_link_plan() -> NativeLibLinkPlan {
    NativeLibLinkPlan::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        native_lib_link_plan, NativeLibLinkPlan, NATIVE_LIB_ELF_PATH, NATIVE_LIB_SHIM_OBJECT,
        RUST_STATICLIB_PATH,
    };

    #[test]
    fn links_the_rust_staticlib_into_the_native_lib_flow() {
        let plan = native_lib_link_plan();

        assert_eq!(
            plan.link_inputs().collect::<Vec<_>>(),
            vec![plan.shim_object_path(), plan.rust_staticlib_path(),]
        );
        assert_eq!(
            plan.elf_path(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(NATIVE_LIB_ELF_PATH)
        );
    }

    #[test]
    fn uses_the_expected_baseline_paths() {
        let plan = NativeLibLinkPlan::new("fixtures/native-lib-baseline");

        assert_eq!(
            plan.shim_object_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(NATIVE_LIB_SHIM_OBJECT)
        );
        assert_eq!(
            plan.rust_staticlib_path(),
            PathBuf::from("fixtures/native-lib-baseline").join(RUST_STATICLIB_PATH)
        );
    }
}
