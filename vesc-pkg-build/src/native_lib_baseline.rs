use std::path::PathBuf;

pub const NATIVE_LIB_BASELINE_INPUTS: [&str; 8] = [
    "src/package_lib.c",
    "src/vesc_c_if.h",
    "src/rules.mk",
    "src/link.ld",
    "src/conv.py",
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

pub const NATIVE_LIB_BASELINE_OUTPUTS: [&str; 4] = [
    "target/native-lib-baseline/native_lib.elf",
    "target/native-lib-baseline/native_lib.bin",
    "target/native-lib-baseline/package_lib.bin",
    "target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg",
];

pub const VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES: u64 = 128 * 1024;

pub const NATIVE_LIB_BASELINE_PACKAGE_INPUTS: [&str; 3] = [
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibBaselinePath {
    root: PathBuf,
}

impl NativeLibBaselinePath {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        NATIVE_LIB_BASELINE_INPUTS
            .iter()
            .map(move |relative| self.root.join(relative))
    }

    pub fn package_input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        NATIVE_LIB_BASELINE_PACKAGE_INPUTS
            .iter()
            .map(move |relative| self.root.join(relative))
    }
}

pub fn native_lib_baseline_root() -> NativeLibBaselinePath {
    NativeLibBaselinePath::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/native-lib-baseline"),
    )
}

pub fn baseline_input_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_INPUTS.iter().copied()
}

pub fn baseline_output_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_OUTPUTS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::{
        baseline_input_paths, baseline_output_paths, native_lib_baseline_root,
        NATIVE_LIB_BASELINE_INPUTS, NATIVE_LIB_BASELINE_OUTPUTS,
        VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES,
    };
    use std::fs;

    #[test]
    fn lists_expected_baseline_inputs() {
        assert_eq!(
            baseline_input_paths().collect::<Vec<_>>(),
            NATIVE_LIB_BASELINE_INPUTS
        );
    }

    #[test]
    fn lists_expected_baseline_outputs() {
        assert_eq!(
            baseline_output_paths().collect::<Vec<_>>(),
            NATIVE_LIB_BASELINE_OUTPUTS
        );
    }

    #[test]
    fn fixture_contains_the_expected_input_layout() {
        let root = native_lib_baseline_root();

        let missing = root
            .input_paths()
            .filter(|path| !path.exists())
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "missing native-lib baseline files: {missing:?}"
        );
    }

    #[test]
    fn package_payload_stays_well_below_the_vesc_tool_flash_block_limit() {
        let root = native_lib_baseline_root();
        let sizes = root
            .package_input_paths()
            .map(|path| {
                let size = fs::metadata(&path).expect("package input metadata").len();
                (path, size)
            })
            .collect::<Vec<_>>();

        let total_size = sizes.iter().map(|(_, size)| *size).sum::<u64>();
        let biggest = sizes.iter().map(|(_, size)| *size).max().unwrap_or(0);

        assert!(
            biggest < VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES,
            "largest package input reaches the VESC flash block limit: {sizes:?}"
        );
        assert!(
            total_size < VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES,
            "package payload is too large for the VESC flash block: {sizes:?}"
        );
    }
}
